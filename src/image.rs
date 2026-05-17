use std::io::{self, Write};
use std::time::Duration;

use crate::{Type, write_u32_le};

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
    ) -> Self {
        Self {
            width,
            height,
            hotspot_x,
            hotspot_y,
            delay,
            argb,
        }
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
