#![cfg_attr(not(feature = "std"), no_std)]
#![feature(error_in_core)]

pub mod error;
pub mod config;
pub mod command;
pub mod response;

#[cfg(feature = "std")]
pub mod hex_parser;

#[cfg(test)]
mod tests;
