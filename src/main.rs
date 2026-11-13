#![no_std]
#![no_main]

use core::{arch::global_asm, panic::PanicInfo};

use ufmt::uwriteln;

#[panic_handler]
fn panic_handler(_: &PanicInfo) -> ! {
    loop {}
}

global_asm!(
    ".syntax unified
    .code 16

    .global start
    .section .text.start
    start:
        ldr r1, =0x8a000
        msr msp, r1
        bl main"
);

mod drivers;
mod err;
use drivers::uart::Serial;

use crate::drivers::clk::pll::PLL;
use crate::drivers::clk::soc::SoCClocks;
use crate::drivers::dram::Dram;
use crate::drivers::efuse::Efuse;
use crate::drivers::iram::IRAM;
use crate::drivers::spi::SpiNand;
use crate::drivers::usb::Usb;
use crate::drivers::zte_protocol::ZteProtocol;
use crate::drivers::{Driver, DriverMut, StatelessDriver};

unsafe fn early_init() {
    uwriteln!(&mut Serial, "Early init triggered");

    uwriteln!(&mut Serial, "PLL init");
    unsafe { PLL::init() };

    uwriteln!(&mut Serial, "Clock init");
    unsafe { SoCClocks::init() };

    uwriteln!(&mut Serial, "UART re-init");
    unsafe { Serial::init() };

    uwriteln!(&mut Serial, "Early init finished");
}

unsafe fn init() {
    uwriteln!(&mut Serial, "Init triggered");

    uwriteln!(&mut Serial, "IRAM setup");
    unsafe { IRAM::init() };

    uwriteln!(&mut Serial, "Efuse init");
    let efuse = unsafe { Efuse::init() };
    uwriteln!(&mut Serial, "Efuse provided info:");
    uwriteln!(
        &mut Serial,
        "\tFused device: {}",
        if efuse.secure { "yes" } else { "no" }
    );
    uwriteln!(&mut Serial, "\tDRAM size: {}", efuse.dram_size);

    uwriteln!(&mut Serial, "DRAM init");
    let dram = Dram::new(efuse.dram_size);
    unsafe {
        dram.init();

        if let Err(e) = dram.verify() {
            uwriteln!(&mut Serial, "Error on DRAM verification: {}", e);
        } else {
            uwriteln!(&mut Serial, "DRAM R/W test pass");
        }
    }

    uwriteln!(&mut Serial, "SPI NAND/NOR init");
    let spinand = unsafe { SpiNand::init() };
    if let Ok(id) = unsafe { spinand.read_id() } {
        uwriteln!(
            &mut Serial,
            "SPI Flash ID: {:02x} {:02x} {:02x}",
            id[0],
            id[1],
            id[2]
        );
    } else {
        uwriteln!(&mut Serial, "Error reading SPI Flash ID");
    }

    uwriteln!(&mut Serial, "Init finished");
}

unsafe fn late_init() {
    uwriteln!(&mut Serial, "Late init triggered");

    unsafe {
        let mut usb = Usb::new();
        usb.init();

        let mut protocol = ZteProtocol::new(usb);
        if let Err(e) = protocol.dispatch() {
            uwriteln!(&mut Serial, "Error on running protocol: {}", e);
        }
    }

    uwriteln!(&mut Serial, "Late init finished");
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn main() -> ! {
    uwriteln!(&mut Serial, "Hello from Rust :)");

    unsafe {
        early_init();
        init();
        late_init();
    }

    uwriteln!(&mut Serial, "M0: Waiting for PMU handshake from A7...");

    const M0_FLAG_ADDR: usize = 0x82002cf4;
    const M0_REMAP_REG: usize = 0x00140000;
    const M0_FW_DDR_ADDR: usize = 0x21400000;

    // Reset handshake flag
    core::ptr::write_volatile(M0_FLAG_ADDR as *mut u32, 0);

    // Wait until A7 signals flag == 1
    while core::ptr::read_volatile(M0_FLAG_ADDR as *const u32) != 1 {
        // spin
    }

    uwriteln!(&mut Serial, "M0: Received signal from A7! Remapping...");

    // Remap M0: *(0x00140000) |= 0x800000
    let remap = core::ptr::read_volatile(M0_REMAP_REG as *const u32);
    core::ptr::write_volatile(M0_REMAP_REG as *mut u32, remap | 0x800000);

    // Read uImage header at 0x21400000 (Big Endian)
    let uimage = M0_FW_DDR_ADDR as *const u32;
    let magic = u32::from_be(core::ptr::read_volatile(uimage.add(0)));
    let size = u32::from_be(core::ptr::read_volatile(uimage.add(3))) as usize;
    let load = u32::from_be(core::ptr::read_volatile(uimage.add(4))) as usize;
    let ep = u32::from_be(core::ptr::read_volatile(uimage.add(5))) as usize;

    uwriteln!(&mut Serial, "M0: uImage magic: {:#x}, size: {}", magic, size);

    if magic == 0x27051956 {
        let src = (M0_FW_DDR_ADDR + 0x40) as *const u32;
        let dst = load as *mut u32;
        let words = (size + 3) / 4;

        for i in 0..words {
            core::ptr::write_volatile(dst.add(i), core::ptr::read_volatile(src.add(i)));
        }

        uwriteln!(&mut Serial, "M0: PMU firmware copied! Sending ACK...");

        // ACK to A7: *0x82002cf4 = 0
        core::ptr::write_volatile(M0_FLAG_ADDR as *mut u32, 0);

        // Fetch MSP from vector table at load address
        let msp = core::ptr::read_volatile(load as *const u32);

        uwriteln!(&mut Serial, "M0: Jumping to PMU entry: {:#x}, MSP: {:#x}", ep, msp);

        // Jump to PMU entry
        core::arch::asm!(
            "msr msp, {msp}",
            "bx {ep}",
            msp = in(reg) msp,
            ep = in(reg) ep,
            options(noreturn)
        );
    } else {
        uwriteln!(&mut Serial, "M0: ERROR: invalid uImage magic!");
        loop {}
    }
}
