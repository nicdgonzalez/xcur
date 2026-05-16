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
use std::io::{self, Read, Write};
use std::path::Path;
use std::time::Duration;
use std::{fmt, mem};

use crate::error::ParseError;
use crate::parser::Parser;

pub struct Xcursor {
    images: Vec<Image>,
    comments: Vec<Comment>,
}

#[derive(Debug, Clone)]
pub struct Image {
    width: u16,
    height: u16,
    hotspot_x: u16,
    hotspot_y: u16,
    delay: Duration,
    argb: Vec<u8>,
}

impl Image {
    pub fn write<W>(&self, mut writer: W) -> io::Result<()>
    where
        W: Write,
    {
        let width = u32::from(self.width);
        let height = u32::from(self.height);
        let hotspot_x = u32::from(self.hotspot_x);
        let hotspot_y = u32::from(self.hotspot_y);

        // Nominal size subtype -- usually the height/width for square cursors.
        let subtype = width;

        let delay = self.delay.as_millis().try_into().unwrap_or(u32::MAX);

        writer.write_all(u32::to_le_bytes(36).as_ref())?;
        writer.write_all((Type::Image as u32).to_le_bytes().as_ref())?;
        writer.write_all(u32::to_le_bytes(subtype).as_ref())?;
        writer.write_all(u32::to_le_bytes(1).as_ref())?;
        writer.write_all(u32::to_le_bytes(width).as_ref())?;
        writer.write_all(u32::to_le_bytes(height).as_ref())?;
        writer.write_all(u32::to_le_bytes(hotspot_x).as_ref())?;
        writer.write_all(u32::to_le_bytes(hotspot_y).as_ref())?;
        writer.write_all(u32::to_le_bytes(delay).as_ref())?;
        writer.write_all(&self.argb)?;

        Ok(())
    }
}

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

    pub fn write<W>(&self, mut writer: W) -> io::Result<()>
    where
        W: Write,
    {
        let entry_count = self.comments.len() + self.images.len();
        let entry_count = u32::try_from(entry_count).expect("usize overflowed u32");

        writer.write_all(b"Xcur")?;
        writer.write_all(u32::to_le_bytes(16).as_ref())?;
        writer.write_all(u32::to_le_bytes(0x0001_0000).as_ref())?;
        writer.write_all(u32::to_le_bytes(entry_count).as_ref())?;

        let mut offset = 16 + (12 * entry_count); // Start from the end of the Table of Contents.

        // Construct the Table of Contents.
        for entry in &self.comments {
            let comment_length = entry.buffer.len();
            let comment_length =
                u32::try_from(comment_length).expect("comment length should not exceed u32");

            writer.write_all((Type::Comment as u32).to_le_bytes().as_ref())?;
            writer.write_all((entry.kind as u32).to_le_bytes().as_ref())?;
            writer.write_all(offset.to_le_bytes().as_ref())?;

            offset += 20 + comment_length;
        }

        for entry in &self.images {
            let image_length = entry.argb.len();
            let image_length =
                u32::try_from(image_length).expect("image length should not exceed u32");

            writer.write_all((Type::Image as u32).to_le_bytes().as_ref())?;
            writer.write_all(entry.width.to_le_bytes().as_ref())?;
            writer.write_all(offset.to_le_bytes().as_ref())?;

            offset += 36 + image_length;
        }

        for entry in &self.comments {
            entry.write(&mut writer)?;
        }

        for entry in &self.images {
            entry.write(&mut writer)?;
        }

        Ok(())
    }
}

const ENTRY_SIZE: usize = mem::size_of::<Entry>();

impl TryFrom<&[u8]> for Xcursor {
    type Error = ParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut parser = Parser::new(value);

        let signature = parser.read_bytes(4)?;

        if signature != b"Xcur" {
            return Err(ParseError::InvalidSignature);
        }

        println!("Signature: {}", String::from_utf8_lossy(signature));

        let _header_size = parser.read_card32()?;
        let version = parser.read_card32()?;
        println!("Version: {version}");

        let entry_count = parser.read_card32()?;
        println!("Entries in Table of Contents: {entry_count}");

        let table_of_contents_size =
            ENTRY_SIZE * usize::try_from(entry_count).expect("u32 overflowed usize");

        let table_of_contents = parser
            .read_bytes(table_of_contents_size)?
            .as_chunks::<ENTRY_SIZE>()
            .0 // Skip remainder because we know there is none.
            .iter()
            .filter_map(parse_entry)
            .collect::<Vec<_>>();

        println!("Table of Contents: {table_of_contents:#?}");

        for entry in table_of_contents {
            match entry.kind {
                EntryKind::Image(nominal) => {
                    println!("Nominal size: {nominal}");
                    let position = usize::try_from(entry.position).expect("u32 overflowed usize");
                    let raw_header = value
                        .iter()
                        .skip(position)
                        .take(36)
                        .copied()
                        .collect::<Vec<_>>();
                    let mut fields = raw_header
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .copied()
                        .map(u32::from_le_bytes);

                    let header_size = {
                        let value = fields.next().unwrap();
                        usize::try_from(value).expect("u32 overflowed usize")
                    };

                    let _raw_type = fields.next().unwrap();
                    let _raw_subtype = fields.next().unwrap();

                    let version = fields.next().unwrap();
                    println!("Version: {version}");

                    let width = fields.next().unwrap();
                    println!("Width: {width}");
                    assert!(width < 0x7FFF);

                    let height = fields.next().unwrap();
                    println!("Height: {height}");
                    assert!(height < 0x7FFF);

                    let hotspot_x = fields.next().unwrap();
                    println!("Hotspot X: {hotspot_x}");
                    let hotspot_y = fields.next().unwrap();
                    println!("Hotspot Y: {hotspot_y}");

                    let delay = fields.next().unwrap();
                    println!("Delay: {delay}");

                    let image_size =
                        usize::try_from(width * height * 4).expect("u32 overflowed usize");
                    let argb = value
                        .iter()
                        .skip(position + header_size)
                        .take(image_size)
                        .copied()
                        .collect::<Vec<_>>();

                    println!("{:->80}", "");
                }
                EntryKind::Comment(comment) => {
                    println!("Comment subtype: {comment}");
                    // parse comment chunk
                }
            }
        }

        todo!()
    }
}

fn parse_entry(entry: &[u8; ENTRY_SIZE]) -> Option<Entry> {
    const FIELD_SIZE: usize = mem::size_of::<u32>();
    debug_assert_eq!(FIELD_SIZE * 3, ENTRY_SIZE);

    let mut fields = entry
        .as_chunks::<FIELD_SIZE>()
        .0 // Skip remainder because we know there is none.
        .iter()
        .copied()
        .map(u32::from_le_bytes);

    let raw_type = fields.next().unwrap();
    let raw_subtype = fields.next().unwrap();
    let position = fields.next().unwrap();

    let kind = match Type::try_from(raw_type)
        .inspect_err(|n| tracing::warn!("unknown entry type: {n}"))
        .ok()?
    {
        Type::Image => EntryKind::Image(raw_subtype),
        Type::Comment => CommentKind::try_from(raw_subtype)
            .inspect_err(|n| tracing::warn!("unknown comment entry subtype: {n}"))
            .map(EntryKind::Comment)
            .ok()?,
    };

    Some(Entry { kind, position })
}

// Represents an entry in the Table of Contents.
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    kind: EntryKind,
    position: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum EntryKind {
    Image(u32),
    Comment(CommentKind),
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

#[derive(Debug, Clone)]
pub struct Comment {
    kind: CommentKind,
    buffer: String,
}

impl Comment {
    pub fn write<W>(&self, mut writer: W) -> io::Result<()>
    where
        W: Write,
    {
        // TODO: Ensure buffer length is at most u32, per Xcursor specification.
        let buffer_length = u32::try_from(self.buffer.len()).expect("usize overflowed u32");

        writer.write_all(u32::to_le_bytes(20).as_ref())?;
        writer.write_all((Type::Comment as u32).to_le_bytes().as_ref())?;
        writer.write_all((self.kind as u32).to_le_bytes().as_ref())?;
        writer.write_all(u32::to_le_bytes(1).as_ref())?;
        writer.write_all(buffer_length.to_le_bytes().as_ref())?;
        writer.write_all(self.buffer.as_bytes())?;

        Ok(())
    }
}

/// Represents the different subtypes of a comment chunk.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentKind {
    Copyright = 1,
    License = 2,
    Other = 3,
}

impl TryFrom<u32> for CommentKind {
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

impl fmt::Display for CommentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Copyright => "Copyright".fmt(f),
            Self::License => "License".fmt(f),
            Self::Other => "Other".fmt(f),
        }
    }
}
