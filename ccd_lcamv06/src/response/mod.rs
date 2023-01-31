pub mod parser;
mod version_details;
mod version_parser;

use crate::flags::BaudRate;
pub use version_details::VersionDetails;

// While there is a large difference in response sizes, all of the small ones usually come one at a
// time, while SingleReading may come as a stream. Plus Box<_> cannot be used because of no_std
#[allow(clippy::large_enum_variant)]
/// Package that can be received from CCD
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
const FRAME_TOTAL_COUNT: usize = FRAME_PIXEL_PREFIX + FRAME_PIXEL_COUNT + FRAME_PIXEL_POSTFIX;

/// CCD captured data
pub type Frame = [u16; FRAME_PIXEL_COUNT];
