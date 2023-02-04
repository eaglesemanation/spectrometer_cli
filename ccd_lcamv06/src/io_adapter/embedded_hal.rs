use super::IoAdapter;
use crate::error::{Error, Result};
use embedded_hal_nb::serial::{Read, Write};
use nb::block;

pub struct EmbeddedHalNbAdapter<IO: Read + Write> {
    io: IO,
}

impl<IO: Read + Write> IoAdapter for EmbeddedHalNbAdapter<IO> {
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        for b in buf {
            block!(self.io.write(*b)).map_err(|_| Error::EmbeddedHalNbError)?;
        }
        Ok(())
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        for (i, b) in buf.iter_mut().enumerate() {
            // TODO: Add a timeout for this block
            match block!(self.io.read()) {
                Ok(val) => {
                    *b = val;
                }
                // TODO: Maybe somehow validate that error? Probably there needs to be a change in
                // embedded-hal traits for that to happen
                Err(_) => {
                    return Ok(i);
                }
            }
        }
        Ok(buf.len())
    }
}
