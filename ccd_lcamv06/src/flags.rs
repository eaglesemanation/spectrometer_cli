use crate::error::Error;
use core::{
    fmt,
    fmt::{Debug, Display},
};
use num_derive::{FromPrimitive, ToPrimitive};

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum TriggerMode {
    SoftTrigger = 0x00,
    ContiniousHardTrigger = 0x01,
    SingleHardTrigger = 0x02,
}

#[derive(ToPrimitive, FromPrimitive, Debug, PartialEq, Eq, Clone, Copy, Default)]
pub enum BaudRate {
    #[default]
    Baud115200 = 115200,
    Baud384000 = 384000,
    Baud921600 = 921600,
}

impl BaudRate {
    pub(crate) fn try_from_code(c: u8) -> Result<Self, Error> {
        use BaudRate::*;
        match c {
            0x01 => Ok(Baud115200),
            0x02 => Ok(Baud384000),
            0x03 => Ok(Baud921600),
            _ => Err(Error::InvalidBaudRate),
        }
    }

    pub(crate) fn to_code(self) -> u8 {
        use BaudRate::*;
        match self {
            Baud115200 => 0x01,
            Baud384000 => 0x02,
            Baud921600 => 0x03,
        }
    }
}

impl Display for BaudRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{}", *self as u32))
    }
}

