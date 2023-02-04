#[cfg(feature = "std")]
pub(crate) mod std_io;
#[cfg(feature = "embedded-hal-nb")]
pub(crate) mod embedded_hal;

use crate::{error::Result, ccd::CCD};

pub trait IoAdapter {
    fn write_all(&mut self, buf: &[u8]) -> Result<()>;
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

    fn open_ccd(self) -> CCD<Self>
    where
        Self: Sized
    {
        CCD::new(self)
    }
}
