use crate::flags::{TriggerMode, BaudRate};

/// Package that can be sent to CCD
#[derive(PartialEq, Eq, Debug)]
pub(crate) enum Command {
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
    fn code(&self) -> u8 {
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

    pub fn encode(&self) -> [u8; 5] {
        use Command::*;
        let [data1, data2] = match self {
            SetIntegrationTime(t) => t.to_be_bytes(),
            SetTrigerMode(m) => [*m as u8, 0x00],
            SetAverageTime(t) => [*t as u8, 0x00],
            SetSerialBaudRate(r) => [r.to_code(), 0x00],
            _ => [0x00, 0x00],
        };
        [0x81, self.code(), data1, data2, 0xFF]
    }
}
