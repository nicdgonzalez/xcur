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

// We intentionally do not use `mem::size_of::<Entry>()` here because our `Entry` struct contains
// nested enums, which increases the struct's size beyond the original layout.
const ENTRY_SIZE: usize = 12;

impl TryFrom<&[u8]> for Xcursor {
    type Error = ParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut parser = Parser::new(value);

        let signature = parser.read_bytes(4)?;

        if signature != b"Xcur" {
            return Err(ParseError::InvalidSignature);
        }

        let header_size = parser.read_card32()?;
        println!("Header size: {header_size}");
        let version = parser.read_card32()?;
        println!("File version: {version}");

        let ntoc = parser.read_card32()?;
        println!("Entries in Table of Contents: {ntoc}");

        let ntoc_usize = usize::try_from(ntoc).expect("u32 overflowed usize");
        let toc_size = ENTRY_SIZE * ntoc_usize;

        let toc = parser
            .read_bytes(toc_size)?
            .as_chunks::<ENTRY_SIZE>()
            .0
            .iter()
            .filter_map(parse_entry)
            .collect::<Vec<Entry>>();

        println!("Table of contents: {toc:#?}");

        todo!()
    }
}

fn parse_entry(chunk: &[u8; ENTRY_SIZE]) -> Option<Entry> {
    let mut fields = chunk
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&bytes| u32::from_le_bytes(bytes));

    let r#type = Type::try_from(fields.next().unwrap())
        .inspect_err(|n| tracing::warn!("unknown entry type: {n}"))
        .ok()?;

    let subtype = {
        let value = fields.next().unwrap();
        match r#type {
            Type::Comment => Comment::try_from(value)
                .inspect_err(|n| tracing::warn!("unknown comment entry subtype: {n}"))
                .map(Subtype::Comment)
                .ok()?,
            Type::Image => Subtype::Image(value),
        }
    };

    let position = fields.next().unwrap();

    Some(Entry {
        r#type,
        subtype,
        position,
    })
}

/// Represents an entry in the Table of Contents.
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    r#type: Type,
    subtype: Subtype,
    position: u32,
}

/// Represents the available chunk types.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Image = 0xFFFD_0002,
    Comment = 0xFFFE_0001,
}

impl TryFrom<u32> for Type {
    type Error = u32;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0xFFFD_0002 => Ok(Self::Image),
            0xFFFE_0001 => Ok(Self::Comment),
            n => Err(n),
        }
    }
}

/// Represents the available chunk subtypes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subtype {
    Image(u32),
    Comment(Comment),
}

/// Represents the different subtypes of a comment chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comment {
    Copyright = 1,
    License = 2,
    Other = 3,
}

impl TryFrom<u32> for Comment {
    type Error = u32;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Copyright),
            2 => Ok(Self::License),
            3 => Ok(Self::Other),
            n => Err(n),
        }
    }
}
