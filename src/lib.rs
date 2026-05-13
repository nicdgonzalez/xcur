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
use std::mem;
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
        let mut parser = Parser::new(value);

        // Parse the Table of Contents
        let signature = parser.read_bytes(4)?;

        if signature != b"Xcur" {
            return Err(ParseError::InvalidSignature);
        }

        let header_size = parser.read_card32()?;
        println!("Header size: {header_size}");

        let version = parser.read_card32()?;
        println!("File version: {version}");

        let ntoc = parser.read_card32()?;
        let ntoc = usize::try_from(ntoc).expect("u32 overflowed usize");
        let toc = parser
            .read_bytes(mem::size_of::<TocEntry>() * ntoc)?
            .chunks_exact(mem::size_of::<TocEntry>())
            .filter_map(|chunk| {
                let mut fields = chunk
                    .chunks_exact(mem::size_of::<u32>())
                    .map(|field| field.try_into().map(u32::from_le_bytes).unwrap());

                let r#type = match fields.next().unwrap() {
                    0xFFFE_0001 => Type::Comment,
                    0xFFFD_0002 => Type::Image,
                    n => {
                        tracing::warn!("ignoring unknown entry type: {n}");
                        return None;
                    }
                };

                let subtype = match fields.next().unwrap() {
                    n if r#type == Type::Comment => match n {
                        1 => Subtype::Comment(Comment::Copyright),
                        2 => Subtype::Comment(Comment::License),
                        3 => Subtype::Comment(Comment::Other),
                        n => {
                            tracing::warn!("ignoring unknown entry subtype: {n}");
                            return None;
                        }
                    },
                    nominal if r#type == Type::Image => Subtype::Image(nominal),
                    _ => unreachable!("should have returned already if unknown type"),
                };

                let position = fields.next().unwrap();

                Some(TocEntry {
                    r#type,
                    subtype,
                    position,
                })
            })
            .collect::<Vec<TocEntry>>();
        println!("Table of contents: {toc:?}");

        todo!()
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TocEntry {
    r#type: Type,
    subtype: Subtype,
    position: u32,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Comment = 0xFFFE_0001,
    Image = 0xFFFD_0002,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subtype {
    Comment(Comment),
    Image(u32),
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comment {
    Copyright = 1,
    License = 2,
    Other = 3,
}
