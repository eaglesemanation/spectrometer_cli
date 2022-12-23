#![cfg_attr(not(feature = "std"), no_std)]
#![feature(error_in_core)]

mod command_encoder;
mod response_parser;

pub mod types;
pub mod error;
// TODO: Move to separate crate
pub mod codec;

#[cfg(feature = "std")]
pub mod hex_parser;

#[cfg(test)]
mod tests;
