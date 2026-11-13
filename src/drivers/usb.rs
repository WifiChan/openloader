use simpleport::{SimpleRead, SimpleWrite};
use ufmt::uwriteln;

use crate::drivers::delay::nsdelay;
use crate::drivers::{readl, writel};
use crate::drivers::DriverMut;
use crate::drivers::regs::register;
use crate::drivers::uart::Serial;
use crate::err::USBError;

const TYPE_BULK: usize = 2;

// DWC OTG device-mode controller base. The original tloader selects this at
// runtime from [0x13B004] & 3: high-speed -> 0x1600000, full-speed -> 0x1500000.
// The extracted config descriptor uses MPS=512 (high speed), so default to the
// high-speed base. If the device negotiates full speed, flip this to 0x01500000.
const USB_BASE: usize = 0x01600000;

// ZX297520V3 USB PHY registers (reverse engineered from sub_83688).
const USB_PHY_STATUS: usize = 0x13B004;
const USB_PHY_0X14010C: usize = 0x14010C;
const USB_PHY_0X140110: usize = 0x140110;
const USB_PHY_0X140114: usize = 0x140114;
const USB_PHY_0X13B06C: usize = 0x13B06C;
const USB_PHY_0X13B080: usize = 0x13B080;
const USB_PHY_0X13B084: usize = 0x13B084;

/// Read the current USB speed. 2 == high speed, otherwise full speed.
unsafe fn usb_speed() -> usize {
    (unsafe { readl(USB_PHY_STATUS) }) & 3
}

/// Busy-wait delay used by the PHY init sequence. sub_8545C(ticks) waits
/// `ticks * 0xD0` hardware timer counts; we approximate with a NOP loop.
unsafe fn phy_delay(ticks: usize) {
    unsafe { nsdelay((ticks * 0x1000) as u32) };
}

/// Translate the original tloader's USB PHY init (sub_83688). Two paths exist
/// and the original picks them via [dword_879FC + 0x14]; here we key off the
/// live speed register directly.
unsafe fn usb_phy_init() {
    let high_speed = unsafe { usb_speed() } == 2;

    if high_speed {
        // High-speed path: bit9 on 0x14010x, bits2/1/0 on 0x13B06C/0x13B080,
        // then poll 0x13B084 bit0 until set.
        unsafe {
            writel(USB_PHY_0X140110, readl(USB_PHY_0X140110) | 0x200);
            phy_delay(0xA);
            writel(USB_PHY_0X14010C, readl(USB_PHY_0X14010C) | 0x200);
            phy_delay(0xA);
            writel(USB_PHY_0X140114, readl(USB_PHY_0X140114) & !0x200);
            phy_delay(0xA);
            writel(USB_PHY_0X140114, readl(USB_PHY_0X140114) | 0x200);
            phy_delay(0xA);
            writel(USB_PHY_0X14010C, readl(USB_PHY_0X14010C) & !0x200);
            phy_delay(0xA);
            writel(USB_PHY_0X140110, readl(USB_PHY_0X140110) & !0x200);
            phy_delay(0xA);

            writel(USB_PHY_0X13B06C, readl(USB_PHY_0X13B06C) & !4);
            phy_delay(0x14);
            writel(USB_PHY_0X13B06C, readl(USB_PHY_0X13B06C) | 4);
            writel(USB_PHY_0X13B06C, readl(USB_PHY_0X13B06C) & !2);
            phy_delay(0x14);
            writel(USB_PHY_0X13B06C, readl(USB_PHY_0X13B06C) | 2);
            writel(USB_PHY_0X13B06C, readl(USB_PHY_0X13B06C) & !1);
            phy_delay(0x14);
            writel(USB_PHY_0X13B06C, readl(USB_PHY_0X13B06C) | 1);
            writel(USB_PHY_0X13B080, readl(USB_PHY_0X13B080) & !4);
            phy_delay(0x14);

            writel(USB_PHY_0X13B080, readl(USB_PHY_0X13B080) | 4);
            phy_delay(0xA);
            writel(USB_PHY_0X13B080, readl(USB_PHY_0X13B080) & !2);
            phy_delay(0x14);
            writel(USB_PHY_0X13B080, readl(USB_PHY_0X13B080) | 2);
            writel(USB_PHY_0X13B080, readl(USB_PHY_0X13B080) & !1);
            phy_delay(0x14);
            writel(USB_PHY_0X13B080, readl(USB_PHY_0X13B080) | 1);
            phy_delay(0x64);

            let mut timeout = 0xC351usize;
            loop {
                if readl(USB_PHY_0X13B084) & 1 != 0 {
                    break;
                }
                timeout -= 1;
                if timeout == 0 {
                    break;
                }
                phy_delay(0x14);
            }
        }
    } else {
        // Full-speed path: bit8 on 0x14010x, bits4/3 on 0x13B06C,
        // bits5/4/3 on 0x13B080, then poll 0x13B084 bit1 until set.
        unsafe {
            writel(USB_PHY_0X140110, readl(USB_PHY_0X140110) | 0x100);
            phy_delay(0xA);
            writel(USB_PHY_0X14010C, readl(USB_PHY_0X14010C) | 0x100);
            phy_delay(0xA);
            writel(USB_PHY_0X140114, readl(USB_PHY_0X140114) & !0x100);
            phy_delay(0xA);
            writel(USB_PHY_0X140114, readl(USB_PHY_0X140114) | 0x100);
            phy_delay(0xA);
            writel(USB_PHY_0X14010C, readl(USB_PHY_0X14010C) & !0x100);
            phy_delay(0xA);
            writel(USB_PHY_0X140110, readl(USB_PHY_0X140110) & !0x100);
            phy_delay(0xA);

            writel(USB_PHY_0X13B06C, readl(USB_PHY_0X13B06C) & !0x10);
            phy_delay(0x14);
            writel(USB_PHY_0X13B06C, readl(USB_PHY_0X13B06C) | 0x10);
            writel(USB_PHY_0X13B06C, readl(USB_PHY_0X13B06C) & !8);
            phy_delay(0x14);
            writel(USB_PHY_0X13B06C, readl(USB_PHY_0X13B06C) | 8);
            phy_delay(0x64);
            writel(USB_PHY_0X13B080, readl(USB_PHY_0X13B080) & !0x20);
            phy_delay(0x64);
            writel(USB_PHY_0X13B080, readl(USB_PHY_0X13B080) | 0x20);
            writel(USB_PHY_0X13B080, readl(USB_PHY_0X13B080) & !0x10);
            phy_delay(0x64);
            writel(USB_PHY_0X13B080, readl(USB_PHY_0X13B080) | 0x10);
            writel(USB_PHY_0X13B080, readl(USB_PHY_0X13B080) & !8);
            phy_delay(0x64);
            writel(USB_PHY_0X13B080, readl(USB_PHY_0X13B080) | 8);
            phy_delay(0x64);

            let mut timeout = 0xC351usize;
            loop {
                if readl(USB_PHY_0X13B084) & 2 != 0 {
                    break;
                }
                timeout -= 1;
                if timeout == 0 {
                    break;
                }
                phy_delay(0x14);
            }
        }
    }
}

register!(gahbcfg, USB_BASE + 0x008, [
    bit: NPTXFEMPLVL, offset: 5, width: 1;
]);

register!(gintsts, USB_BASE + 0x014, [
    bit: RXFLVL, offset: 4;
    bit: OEPINT, offset: 19;
]);

register!(grxstsp, USB_BASE + 0x020);

register!(dctl, USB_BASE + 0x804, [
    bit: SOFT_RESET1, offset: 8;
    bit: SOFT_RESET2, offset: 10;
]);

register!(dsts, USB_BASE + 0x808);

register!(doepctl1, USB_BASE + 0xb20, [
    field: MPS, offset: 0, width: 11;
    bit: USB_ACTIVE_EP, offset: 15;
    field: EP_TYPE, offset: 18, width: 2;
    bit: CNAK, offset: 26;
    bit: EPENA, offset: 31;
]);

register!(diepint1, USB_BASE + 0x928, [
    bit: XFERCOMPL, offset: 0;
]);

register!(dieptsiz1, USB_BASE + 0x930, [
    field: XFERSIZE, offset: 0, width: 7;
    field: PKTCNT, offset: 19, width: 2;
]);

register!(diepctl1, USB_BASE + 0x920, [
    field: MPS, offset: 0, width: 11;
    bit: USB_ACTIVE_EP, offset: 15;
    field: EP_TYPE, offset: 18, width: 2;
    field: TXFNUM, offset: 22, width: 4;
    bit: CNAK, offset: 26;
    bit: EPENA, offset: 31;
]);

register!(doepint1, USB_BASE + 0xb28, [
    bit: XFERCOMPL, offset: 0;
    bit: SETUP_COMPLETED, offset: 3;
]);

register!(doeptsiz1, USB_BASE + 0xb30, [
    field: SPEED, offset: 0, width: 2;
    bit: PKTCNT, offset: 19;
]);

register!(rx_fifo, USB_BASE + 0x1000);
register!(tx_fifo, USB_BASE + 0x2000);

pub struct Usb {
    rx_buf: [u8; 512],
    rx_ptr: usize,
    rx_cnt: usize,
    ep_mps: usize,
}

impl DriverMut for Usb {
    unsafe fn init(&mut self) {
        unsafe {
            // MINIMAL-INTERVENTION: the BootROM has already initialized the USB
            // PHY and enumerated the USB CDC device. We must NOT touch the PHY
            // (usb_phy_init) or reprogram the DWC OTG controller registers
            // (gahbcfg/soft-reset/EP config) because that tears down the
            // existing link and we have no re-enumeration logic. Only read the
            // negotiated speed (safe) and reuse the BootROM's EP1 setup as-is.
            //
            // usb_phy_init();
            // gahbcfg::read_modify_write(...);
            // dctl soft reset removed;
            // doepctl1/diepctl1/doeptsiz1 config removed;

            let speed = (dsts::read() >> 1) & 0x3;

            self.ep_mps = if speed == 0 {
                uwriteln!(
                    &mut Serial,
                    "USB: Using USB High Speed Mode (MPS=512 bytes)"
                );
                512
            } else {
                uwriteln!(&mut Serial, "USB: Using USB Full Speed Mode (MPS=64 bytes)");
                64
            };
        }
    }
}

impl Usb {
    pub fn new() -> Self {
        Self {
            rx_buf: [0; 512],
            rx_ptr: 0,
            rx_cnt: 0,
            ep_mps: 0,
        }
    }

    unsafe fn read_u8(&mut self) -> Result<u8, USBError> {
        let mut hang_ctr = 0;
        loop {
            if self.rx_ptr < self.rx_cnt {
                let b = self.rx_buf[self.rx_ptr];
                self.rx_ptr += 1;
                break Ok(b);
            }

            let status = unsafe { gintsts::read() };

            if status.is_set_bit(gintsts::RXFLVL) {
                let rx_status = unsafe { grxstsp::read() };
                let packet_status = (rx_status >> 17) & 0xf;
                let byte_count = ((rx_status >> 4) & 0x7ff) as usize;

                // 2 = OUT received, 6 = Setup received
                if (packet_status == 2 || packet_status == 6) && byte_count > 0 {
                    self.rx_ptr = 0;
                    self.rx_cnt = 0;

                    let words = (byte_count + 3) / 4;
                    for _ in 0..words {
                        let val = unsafe { rx_fifo::read() };
                        let bytes = val.to_le_bytes();
                        for k in 0..4 {
                            if self.rx_cnt < 512 && self.rx_cnt < byte_count {
                                self.rx_buf[self.rx_cnt] = bytes[k];
                                self.rx_cnt += 1;
                            }
                        }
                    }
                }
            }

            if status.is_set_bit(gintsts::OEPINT) {
                unsafe {
                    doepint1::read_modify_write(|r| {
                        use doepint1::*;

                        doepint1::write_raw(r.raw());

                        if r.is_set_bit(XFERCOMPL) || r.is_set_bit(SETUP_COMPLETED) {
                            doeptsiz1::read_modify_write(|r| {
                                r.set_bit(doeptsiz1::PKTCNT)
                                    .set_field(doeptsiz1::SPEED, self.ep_mps);
                            });

                            doepctl1::new_scope(|r| {
                                use doepctl1::*;

                                r.set_bit(EPENA)
                                    .set_bit(CNAK)
                                    .set_field(EP_TYPE, TYPE_BULK)
                                    .set_bit(USB_ACTIVE_EP)
                                    .set_field(MPS, self.ep_mps);
                            });
                        }
                    });
                }
            }

            // Don't time out while waiting for USB host commands
        }
    }

    unsafe fn write_u8(&mut self, b: u8) -> Result<(), USBError> {
        unsafe {
            dieptsiz1::new_scope(|r| {
                use dieptsiz1::*;

                r.set_field(PKTCNT, 1).set_field(XFERSIZE, 1);
            });

            diepctl1::new_scope(|r| {
                use diepctl1::*;

                r.set_bit(EPENA)
                    .set_bit(CNAK)
                    .set_field(EP_TYPE, TYPE_BULK)
                    .set_bit(USB_ACTIVE_EP)
                    .set_field(MPS, self.ep_mps);
            });

            tx_fifo::write(b as usize);

            let mut timeout = 10_000_000;
            loop {
                let intr = diepint1::read();
                if intr.is_set_bit(diepint1::XFERCOMPL) {
                    diepint1::write_raw(1);
                    break Ok(());
                }

                timeout -= 1;
                if timeout == 0 {
                    break Err(USBError::Timeout);
                }
            }
        }
    }
}

impl SimpleRead for Usb {
    type Error = USBError;

    fn read(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        for i in 0..buf.len() {
            buf[i] = unsafe { self.read_u8()? };
        }

        Ok(())
    }
}

impl SimpleWrite for Usb {
    type Error = USBError;

    fn write(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        for b in buf {
            unsafe { self.write_u8(*b)? };
        }

        Ok(())
    }
}
