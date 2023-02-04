#![cfg_attr(not(feature = "std"), no_std)]

pub mod error;
pub(crate) mod flags;
pub(crate) mod command;
pub(crate) mod response;

pub mod io_adapter;
pub use io_adapter::IoAdapter;
#[cfg(feature = "std")]
pub use io_adapter::std_io::StdIoAdapter;
#[cfg(feature = "embedded_hal")]
pub use io_adapter::embedded_hal::EmbeddedHalAdapter;

pub mod ccd;
pub use ccd::CCD;

pub use flags::{BaudRate, TriggerMode};
pub use response::{Frame, FRAME_PIXEL_COUNT, VersionDetails};
