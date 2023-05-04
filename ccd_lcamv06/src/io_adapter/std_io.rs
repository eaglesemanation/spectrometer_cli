use super::IoAdapter;
use crate::error::Result;
use std::io::{Read, Write};

pub struct StdIoAdapter<IO: Read + Write> {
    io: IO,
}

impl<IO: Read + Write> IoAdapter for StdIoAdapter<IO> {
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.io.write_all(buf)?;
        Ok(())
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let count = self.io.read(buf)?;
        Ok(count)
    }
}

impl<IO: Read + Write> StdIoAdapter<IO> {
    pub fn new(io: IO) -> Self {
        StdIoAdapter { io }
    }
}
