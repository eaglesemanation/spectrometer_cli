#![cfg_attr(not(feature = "std"), no_std)]
#![feature(error_in_core)]

mod types;
mod error;
mod command_encoder;
mod response_parser;

#[cfg(feature = "std")]
mod hex_parser;

#[cfg(test)]
mod tests;
