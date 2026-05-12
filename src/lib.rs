//! # Xcursor
//!
//! Parser for the Xcursor file format.

#![warn(
    clippy::correctness,
    clippy::suspicious,
    clippy::complexity,
    clippy::perf,
    clippy::style,
    clippy::pedantic
)]

mod error;
mod parser;

use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::error::ParseError;
use crate::parser::Parser;

pub struct Xcursor;

impl Xcursor {
    /// Read the file at path and decode it.
    ///
    /// # Errors
    ///
    /// This function returns an error if:
    ///
    /// - Failed to read file due to I/O errors (e.g., lacking permissions)
    /// - Data is malformed
    pub fn open<P>(path: P) -> Result<Self, ParseError>
    where
        P: AsRef<Path>,
    {
        File::open(path)
            .map_err(|err| ParseError::Io { source: err })
            .and_then(Self::from_reader)
    }

    /// Read and decode the data provided by reader.
    ///
    /// # Errors
    ///
    /// This function returns an error if:
    ///
    /// - Failed to read data due to I/O errors
    /// - Data is malformed
    pub fn from_reader<R>(mut reader: R) -> Result<Self, ParseError>
    where
        R: Read,
    {
        let mut buffer = Vec::new();
        _ = reader
            .read_to_end(&mut buffer)
            .map_err(|err| ParseError::Io { source: err })?;

        Self::try_from(buffer.as_slice())
    }

    /// Decode the data provided.
    ///
    /// # Errors
    ///
    /// This function returns an error if:
    ///
    /// - Data is malformed
    pub fn from_bytes(buffer: &[u8]) -> Result<Self, ParseError> {
        Self::try_from(buffer)
    }
}

impl TryFrom<&[u8]> for Xcursor {
    type Error = ParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut _parser = Parser::new(value);
        todo!()
    }
}
