use crate::error::Error;
use core::{
    fmt,
    fmt::{Debug, Display},
};
use num_derive::{FromPrimitive, ToPrimitive};
use arraystring::SmallString;

#[derive(PartialEq, Eq, Debug)]
pub enum Command {
    SingleRead,
    ContinuousRead,
    PauseRead,
    SetIntegrationTime(u16),
    SetTrigerMode(TriggerMode),
    GetExposureTime,
    GetVersion,
    SetAverageTime(u8),
    GetAverageTime,
    SetSerialBaudRate(BaudRate),
    GetSerialBaudRate,
}

impl Command {
    /// Convert command enum into byte code for encoding
    pub(crate) fn code(&self) -> u8 {
        use Command::*;

        match *self {
            SingleRead => 0x01,
            ContinuousRead => 0x02,
            SetIntegrationTime(_) => 0x03,
            PauseRead => 0x06,
            SetTrigerMode(_) => 0x07,
            GetVersion => 0x09,
            GetExposureTime => 0x0a,
            SetAverageTime(_) => 0x0c,
            GetAverageTime => 0x0e,
            SetSerialBaudRate(_) => 0x13,
            GetSerialBaudRate => 0x16,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Response {
    SingleReading(Frame),
    ExposureTime(u16),
    AverageTime(u8),
    SerialBaudRate(BaudRate),
    VersionInfo(VersionDetails),
}

/// Amount of real pixels in a single frame
pub const FRAME_PIXEL_COUNT: usize = 3694;
/// Each reading is prefixed and postfixed with "ghost" pixels, which can be dropped
const FRAME_PIXEL_PREFIX: usize = 0;
const FRAME_PIXEL_POSTFIX: usize = 0;
/// Amount of pixels in a single package
pub(crate) const FRAME_TOTAL_COUNT: usize = FRAME_PIXEL_PREFIX + FRAME_PIXEL_COUNT + FRAME_PIXEL_POSTFIX;

/// CCD captured data
pub type Frame = [u16; FRAME_PIXEL_COUNT];

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum TriggerMode {
    SoftTrigger = 0x00,
    ContiniousHardTrigger = 0x01,
    SingleHardTrigger = 0x02,
}

#[derive(ToPrimitive, FromPrimitive, Debug, PartialEq, Eq, Clone, Copy)]
pub enum BaudRate {
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

    pub(crate) fn to_code(&self) -> u8 {
        use BaudRate::*;
        match *self {
            Baud115200 => 0x01,
            Baud384000 => 0x02,
            Baud921600 => 0x03,
        }
    }
}

impl Default for BaudRate {
    fn default() -> Self {
        BaudRate::Baud115200
    }
}

impl Display for BaudRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{}", *self as u32))
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct VersionDetails {
    hardware_version: SmallString,
    sensor_type: SmallString,
    firmware_version: SmallString,
    serial_number: SmallString,
}

impl VersionDetails {
    pub(crate) fn try_new(hw_ver: &str, sensor: &str, fw_ver: &str, serial: &str) -> Result<VersionDetails, Error> {
        Ok(VersionDetails{
            hardware_version: SmallString::try_from_str(hw_ver).map_err(|_| Error::VersionDetailTooLong("Hardware version"))?,
            sensor_type: SmallString::try_from_str(sensor).map_err(|_| Error::VersionDetailTooLong("Sensor type"))?,
            firmware_version: SmallString::try_from_str(fw_ver).map_err(|_| Error::VersionDetailTooLong("Firmware version"))?,
            serial_number: SmallString::try_from_str(serial).map_err(|_| Error::VersionDetailTooLong("Serial number"))?
        })
    }
}

impl<'a> Display for VersionDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            concat!(
                "Hardware version: {}\n",
                "Firmware version: {}\n",
                "Sensor type: {}\n",
                "Serial number: {}",
            ),
            &self.hardware_version,
            &self.firmware_version,
            &self.sensor_type,
            &self.serial_number
        ))

    }
}
