use ufmt::{uDisplay, uwrite};

pub enum Error {
    DRAM,
    USB(USBError),
    SPI(SpiError),
}

impl From<USBError> for Error {
    fn from(value: USBError) -> Self {
        Self::USB(value)
    }
}

impl From<SpiError> for Error {
    fn from(value: SpiError) -> Self {
        Self::SPI(value)
    }
}

impl uDisplay for Error {
    fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
    where
        W: ufmt::uWrite + ?Sized,
    {
        match self {
            Self::USB(usb) => uwrite!(f, "USB: {}", usb),
            Self::DRAM => uwrite!(f, "DRAM R/W test failed"),
            Self::SPI(spi) => uwrite!(f, "SPI: {}", spi),
        }
    }
}

pub enum USBError {
    Timeout,
}

impl uDisplay for USBError {
    fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
    where
        W: ufmt::uWrite + ?Sized,
    {
        match self {
            Self::Timeout => uwrite!(f, "Timed out"),
        }
    }
}

pub enum SpiError {
    Timeout,
    ReadFailed,
}

impl uDisplay for SpiError {
    fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
    where
        W: ufmt::uWrite + ?Sized,
    {
        match self {
            Self::Timeout => uwrite!(f, "Timed out"),
            Self::ReadFailed => uwrite!(f, "Read failed"),
        }
    }
}
