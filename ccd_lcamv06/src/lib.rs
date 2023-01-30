#![cfg_attr(not(feature = "std"), no_std)]
#![feature(error_in_core)]

pub mod error;
pub(crate) mod flags;
pub(crate) mod command;
pub(crate) mod response;

// TODO: Move std::io stuff into separate trait so it could be no_std
#[cfg(feature = "std")]
pub mod ccd;

pub use flags::{BaudRate, TriggerMode};
pub use ccd::CCD;
pub use response::{Frame, FRAME_PIXEL_COUNT, VersionDetails};
