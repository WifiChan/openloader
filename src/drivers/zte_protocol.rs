use core::slice;
use derive_ctor::ctor;
use ufmt::uwriteln;

use crate::{
    drivers::{uart::Serial, usb::Usb, writel},
    err::Error,
};
use simpleport::{SimpleRead, SimpleWrite};

const SYNC_FLAG: u8 = 0x5a;
const WRITE32_FLAG: u8 = 0x6a;
const DOWNLOAD_FLAG: u8 = 0x7a;
const RUN_FLAG: u8 = 0x8a;

const SYNC_ACK: u8 = 0xa5;
const WRITE32_ACK: u8 = 0xa6;
const DOWNLOAD_HEADER_ACK: u8 = 0xa1;
const DOWNLOAD_COMPLETE_ACK: u8 = 0xa7;
const RUN_ACK: u8 = 0xa8;

const IRAM1_BASE: usize = 0x100000;
const A53_SUBSYS_CFG: usize = 0x013b138;
const A53_SW_RSTEN: usize = 0xf;

#[derive(ctor)]
pub struct ZteProtocol {
    usb: Usb,
}

impl ZteProtocol {
    pub unsafe fn dispatch(&mut self) -> Result<(), Error> {
        loop {
            // Check UART Serial first
            if !unsafe { Serial::rx_empty() } {
                let cmd = unsafe { Serial::raw_getc() };
                match cmd {
                    SYNC_FLAG => Serial::putc(SYNC_ACK),
                    WRITE32_FLAG => unsafe {
                        let mut addr_bytes = [0u8; 4];
                        for b in &mut addr_bytes { *b = Serial::raw_getc(); }
                        let addr = u32::from_be_bytes(addr_bytes);

                        let mut value_bytes = [0u8; 4];
                        for b in &mut value_bytes { *b = Serial::raw_getc(); }
                        let value = u32::from_be_bytes(value_bytes);

                        if addr != 0 {
                            writel(addr as usize, value as usize);
                        } else {
                            Serial::putc(WRITE32_ACK);
                        }
                    },
                    DOWNLOAD_FLAG => unsafe {
                        let mut addr_bytes = [0u8; 4];
                        for b in &mut addr_bytes { *b = Serial::raw_getc(); }
                        let addr = u32::from_be_bytes(addr_bytes);

                        let mut size_bytes = [0u8; 4];
                        for b in &mut size_bytes { *b = Serial::raw_getc(); }
                        let size = u32::from_be_bytes(size_bytes);

                        Serial::putc(DOWNLOAD_HEADER_ACK);

                        let buf = slice::from_raw_parts_mut(addr as *mut u8, size as usize);
                        for b in buf { *b = Serial::raw_getc(); }

                        Serial::putc(DOWNLOAD_COMPLETE_ACK);
                    },
                    RUN_FLAG => unsafe {
                        let mut addr_bytes = [0u8; 4];
                        for b in &mut addr_bytes { *b = Serial::raw_getc(); }
                        let addr = u32::from_be_bytes(addr_bytes);

                        Self::boot_ap(addr as usize);
                        Serial::putc(RUN_ACK);
                        break Ok(());
                    },
                    _ => {}
                }
            }

            // Also check USB
            let cmd = self.usb.read_u8();
            if let Ok(cmd) = cmd {
                match cmd {
                    SYNC_FLAG => self.usb.write_u8(SYNC_ACK)?,
                    WRITE32_FLAG => unsafe {
                        let addr = self.usb.read_u32_be()?;
                        let value = self.usb.read_u32_be()?;

                        if addr != 0 {
                            writel(addr as usize, value as usize);
                        } else {
                            self.usb.write_u8(WRITE32_ACK)?;
                        }
                    },
                    DOWNLOAD_FLAG => unsafe {
                        let addr = self.usb.read_u32_be()?;
                        let size = self.usb.read_u32_be()?;

                        self.usb.write_u8(DOWNLOAD_HEADER_ACK)?;

                        self.usb
                            .read(slice::from_raw_parts_mut(addr as *mut u8, size as usize))?;

                        self.usb.write_u8(DOWNLOAD_COMPLETE_ACK)?;
                    },
                    RUN_FLAG => unsafe {
                        let addr = self.usb.read_u32_be()?;
                        Self::boot_ap(addr as usize);

                        self.usb.write_u8(RUN_ACK)?;

                        break Ok(());
                    },
                    _ => {}
                }
            }
        }
    }

    unsafe fn boot_ap(uboot_entry: usize) {
        unsafe {
            writel(IRAM1_BASE, 0xe59ff000);
            writel(IRAM1_BASE + 8, uboot_entry);
            writel(A53_SUBSYS_CFG, A53_SW_RSTEN);
        }
    }
}
