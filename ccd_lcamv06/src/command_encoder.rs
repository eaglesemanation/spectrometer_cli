use crate::types::{Command, Command::*};

pub fn encode_command(cmd: &Command) -> [u8; 5] {
    let [data1, data2] = match cmd {
        SetIntegrationTime(t) => t.to_be_bytes(),
        SetTrigerMode(m) => [*m as u8, 0x00],
        SetAverageTime(t) => [*t as u8, 0x00],
        SetSerialBaudRate(r) => [r.to_code(), 0x00],
        _ => [0x00, 0x00],
    };
    [0x81, cmd.code(), data1, data2, 0xFF]
}
