use std::io::{self, Write};
use std::time::Duration;

use crate::error::ParseError;
use crate::{CARD32_SIZE, Entry, Type, write_u32_le};

pub const IMAGE_HEADER_SIZE: u32 = 36;
const IMAGE_VERSION: u32 = 1;

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
    pub fn new(
        width: u16,
        height: u16,
        hotspot_x: u16,
        hotspot_y: u16,
        delay: Duration,
        argb: Vec<u8>,
    ) -> Result<Self, ParseError> {
        if width > 0x7FFF || height > 0x7FFF {
            return Err(ParseError::ImageSize);
        }

        if hotspot_x > width || hotspot_y > height {
            return Err(ParseError::InvalidHotspot);
        }

        Ok(Self {
            width,
            height,
            hotspot_x,
            hotspot_y,
            delay,
            argb,
        })
    }

    pub(crate) fn from_chunk(buffer: &[u8], entry: &Entry) -> Result<Self, ParseError> {
        let position = usize::try_from(entry.position).expect("u32 overflowed usize");
        let header_size = usize::try_from(IMAGE_HEADER_SIZE).expect("u32 overflowed usize");

        let start = position;
        let end = position + header_size;
        let raw_header = &buffer[start..end];

        let mut fields = raw_header
            .as_chunks::<CARD32_SIZE>()
            .0
            .iter()
            .copied()
            .map(u32::from_le_bytes);

        let _header_size = fields.next().unwrap();
        let _raw_type = fields.next().unwrap();
        let _raw_subtype = fields.next().unwrap();
        let _version = fields.next().unwrap();
        let width = fields.next().unwrap();
        let height = fields.next().unwrap();
        let hotspot_x = fields.next().unwrap();
        let hotspot_y = fields.next().unwrap();
        let delay = fields.next().unwrap();

        if width > 0x7FFF || height > 0x7FFF {
            return Err(ParseError::ImageSize);
        }

        if hotspot_x > width || hotspot_y > height {
            return Err(ParseError::InvalidHotspot);
        }

        let image_size = {
            let value = width * height * 4; // 4 = 1 byte per value in ARGB
            usize::try_from(value).expect("u32 overflowed usize")
        };
        let start = end;
        let end = start + image_size;
        let argb = buffer[start..end].to_vec();

        Ok(Self {
            width: u16::try_from(width).unwrap(),
            height: u16::try_from(height).unwrap(),
            hotspot_x: u16::try_from(hotspot_x).unwrap(),
            hotspot_y: u16::try_from(hotspot_y).unwrap(),
            delay: Duration::from_millis(u64::from(delay)),
            argb,
        })
    }

    pub fn write<W>(&self, mut writer: W) -> io::Result<()>
    where
        W: Write,
    {
        let width = u32::from(self.width);
        let height = u32::from(self.height);
        let hotspot_x = u32::from(self.hotspot_x);
        let hotspot_y = u32::from(self.hotspot_y);

        // Nominal size subtype -- usually width/height for square cursors.
        let subtype = width;

        let delay = self.delay.as_millis().try_into().unwrap_or(u32::MAX);

        write_u32_le(&mut writer, IMAGE_HEADER_SIZE)?;
        write_u32_le(&mut writer, Type::Image as u32)?;
        write_u32_le(&mut writer, subtype)?;
        write_u32_le(&mut writer, IMAGE_VERSION)?;
        write_u32_le(&mut writer, width)?;
        write_u32_le(&mut writer, height)?;
        write_u32_le(&mut writer, hotspot_x)?;
        write_u32_le(&mut writer, hotspot_y)?;
        write_u32_le(&mut writer, delay)?;
        writer.write_all(&self.argb)?;

        Ok(())
    }

    #[must_use]
    pub fn width(&self) -> u16 {
        self.width
    }

    #[must_use]
    pub fn height(&self) -> u16 {
        self.height
    }

    #[must_use]
    pub fn hotspot_x(&self) -> u16 {
        self.hotspot_x
    }

    #[must_use]
    pub fn hotspot_y(&self) -> u16 {
        self.hotspot_y
    }

    #[must_use]
    pub fn delay(&self) -> &Duration {
        &self.delay
    }

    #[must_use]
    pub fn argb(&self) -> &[u8] {
        &self.argb
    }
}
