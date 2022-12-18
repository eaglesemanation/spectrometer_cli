use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    // TODO: Figure out a way to assemble list of baud rates at compile time
    #[error("Baud rate is not in range of accepted values: 115200, 384000, 921600")]
    InvalidBaudRate,
    #[error("Could not parse recieved data correctly: {0}")]
    InvalidData(&'static str),
    #[error("Unexpected end of package")]
    UnexpectedEop,
    #[error("{0} is longer than expected")]
    VersionDetailTooLong(&'static str)
}
