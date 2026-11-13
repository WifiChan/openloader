pub mod clk;
pub(super) mod delay;
pub mod dram;
pub(super) mod dram_control;
pub(super) mod dram_phy;
pub mod efuse;
pub mod iram;
pub(super) mod regs;
pub mod uart;
pub mod usb;
pub mod zte_protocol;
pub mod spi;

pub trait StatelessDriver {
    unsafe fn init() -> Self;
}

pub trait Driver {
    unsafe fn init(&self);
}

pub trait DriverMut {
    unsafe fn init(&mut self);
}

pub(super) unsafe fn readl_raw<T>(reg: *const T) -> T {
    unsafe { reg.read_volatile() }
}

pub(super) unsafe fn writel_raw<T>(reg: *mut T, value: T) {
    unsafe { reg.write_volatile(value) };
}

pub(super) unsafe fn readl(reg: usize) -> usize {
    unsafe { readl_raw(reg as *const usize) }
}

pub(super) unsafe fn writel(reg: usize, value: usize) {
    unsafe { writel_raw(reg as *mut usize, value) };
}

pub(super) const fn bit(n: usize) -> usize {
    1 << n
}

pub(super) const fn genmask(h: usize, l: usize) -> usize {
    (!0 << l) & (!0 >> (32 - 1 - h))
}

macro_rules! shift {
    ($name:ident, $shift:expr) => {
        #[allow(non_snake_case)]
        const fn $name(value: usize) -> usize {
            value << $shift
        }
    };
}

pub(super) use shift;
