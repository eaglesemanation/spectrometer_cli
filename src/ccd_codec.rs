use bytes::{Buf, BytesMut};
use std::{io, str::{from_utf8, FromStr}};
use tokio_util::codec::{Decoder, Encoder};

pub struct CCDCodec;

#[derive(PartialEq, Eq, Debug)]
pub enum TriggerMode {
    SoftTrigger = 0x00,
    ContiniousHardTrigger = 0x01,
    SingleHardTrigger = 0x02,
}

#[derive(PartialEq, Eq, Debug)]
pub enum BaudRate {
    Baud115200 = 0x01,
    Baud384000 = 0x02,
    Baud921600 = 0x03,
}

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

fn command_code(cmd: &Command) -> u8 {
    use Command::*;

    match *cmd {
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

const HEAD_SIZE: usize = 5;
const CRC_SIZE: usize = 2;

impl Encoder<Command> for CCDCodec {
    type Error = io::Error;

    fn encode(&mut self, cmd: Command, dst: &mut BytesMut) -> Result<(), Self::Error> {
        use Command::*;

        dst.reserve(HEAD_SIZE);
        dst.extend_from_slice(&[0x81, command_code(&cmd)]);
        match cmd {
            SetIntegrationTime(t) => dst.extend_from_slice(&t.to_be_bytes()),
            SetTrigerMode(m) => dst.extend_from_slice(&[m as u8, 0x00]),
            SetAverageTime(t) => dst.extend_from_slice(&[t as u8, 0x00]),
            SetSerialBaudRate(r) => dst.extend_from_slice(&[r as u8, 0x00]),
            _ => dst.extend_from_slice(&[0x00, 0x00]),
        }
        dst.extend_from_slice(&[0xff]);

        Ok(())
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct VersionDetails {
    hardware_version: String,
    sensor_type: String,
    firmware_version: String,
    serial_number: String,
}

impl FromStr for VersionDetails {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(", ").collect();
        if parts.len() != 4 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid format of version information"));
        }
        Ok(VersionDetails{
            hardware_version: parts[0].to_string(),
            sensor_type: parts[1].to_string(),
            firmware_version: parts[2].to_string(),
            serial_number: parts[3].to_string(),
        })
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum Response {
    SingleRead,
    ExposureTime(u16),
    AverageTime(u8),
    SerialBaudRate(BaudRate),
    VersionInfo(VersionDetails),
}

impl Decoder for CCDCodec {
    type Item = Response;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        use BaudRate::*;
        use Response::*;

        if src.len() < HEAD_SIZE {
            // Wait until at least head is available
            return Ok(None);
        }

        let mut head = [0u8; HEAD_SIZE];
        head.copy_from_slice(&src[..HEAD_SIZE]);

        // Return without a header, probably version info
        if head[0] != 0x81 {
            if let Some(idx) = src.clone().into_iter().position(|b| b == '\n' as u8) {
                let mut buf = Vec::new();
                buf.extend_from_slice(&src[..idx]);
                let version_info = from_utf8(&buf)
                    .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
                    .and_then(|s| VersionDetails::from_str(s))?;
                src.advance(idx + 1);
                return Ok(Some(VersionInfo(version_info)));
            }
            if let Some(idx) = src.clone().into_iter().position(|b| b == 0x81) {
                // Align buffer with the first recieved message
                src.advance(idx);
                return Ok(None);
            }
            if src.len() < 64 {
                // Let buffer fill a bit before deciding that it's garbage
                return Ok(None);
            }
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Could not find a structured response",
            ));
        }

        // Header should be present, either 5 byte command, or 4200 byte scan of ccd pixels
        let (result, advance_size) = match head[1] {
            0x01 => {
                let scan_size: usize = (((head[2] as u16) << 8) | (head[3] as u16)).into();
                let package_size = HEAD_SIZE + scan_size + CRC_SIZE;
                if src.len() < package_size {
                    if src.capacity() < package_size {
                        // Preallocate space for a frame
                        src.reserve(package_size - src.len())
                    }
                    // Reading is not complete
                    (Ok(None), 0)
                } else {
                    // TODO: Check CRC and form a response
                    todo!();
                }
            }
            0x02 => {
                let exp_t = ((head[2] as u16) << 8) | (head[3] as u16);
                match head[4] {
                    0xff => (Ok(Some(ExposureTime(exp_t))), HEAD_SIZE),
                    _ => (
                        Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Unexpected end of package",
                        )),
                        HEAD_SIZE,
                    ),
                }
            }
            0x0e => {
                let avg_t = head[2];
                match (head[3], head[4]) {
                    (0x00, 0xff) => (Ok(Some(AverageTime(avg_t))), HEAD_SIZE),
                    _ => (
                        Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Unexpected end of package",
                        )),
                        HEAD_SIZE,
                    ),
                }
            }
            0x16 => {
                let baud_rate = head[2];
                (
                    match baud_rate {
                        1 => Ok(Some(SerialBaudRate(Baud115200))),
                        2 => Ok(Some(SerialBaudRate(Baud384000))),
                        3 => Ok(Some(SerialBaudRate(Baud921600))),
                        _ => Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Unexpected baud rate reported",
                        )),
                    },
                    HEAD_SIZE,
                )
            }
            _ => (
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Unexpected command code for return value",
                )),
                HEAD_SIZE,
            ),
        };
        src.advance(advance_size);

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_encoding() {
        let mut codec = CCDCodec {};
        let mut dst = BytesMut::with_capacity(0);

        codec
            .encode(Command::SetIntegrationTime(10), &mut dst)
            .unwrap();

        assert_eq!(&dst[..], &[0x81, 0x03, 0x00, 0x0a, 0xff]);
    }

    #[test]
    fn versioninfo_decoding() {
        let mut codec = CCDCodec {};
        let mut src = BytesMut::with_capacity(0);
        src.extend_from_slice("LCAM_V8.4.2, S11639, V4.2, 202111161548\n".as_bytes());

        let res = codec.decode(&mut src).unwrap().unwrap();

        assert_eq!(
            res,
            Response::VersionInfo(VersionDetails{
                hardware_version: "LCAM_V8.4.2".to_string(),
                sensor_type: "S11639".to_string(), 
                firmware_version: "V4.2".to_string(), 
                serial_number: "202111161548".to_string(),
            })
        );
    }
}
