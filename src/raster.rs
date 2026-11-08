// CPU-only images and analysis. No platform, renderer, or filesystem dependency.
// Non-dev and web builds use only part of this module; the analysis serves the dev editor.
#![allow(dead_code, unused_imports)]
use std::io::Cursor;

mod analysis;
pub use analysis::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl RgbaImage {
    pub fn new(width: u32, height: u32) -> Self {
        let len = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .expect("RGBA image dimensions overflow address space");
        Self {
            width,
            height,
            pixels: vec![0; len],
        }
    }
    fn index(&self, x: u32, y: u32) -> usize {
        assert!(x < self.width && y < self.height, "pixel outside image");
        (y as usize * self.width as usize + x as usize) * 4
    }
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = self.index(x, y);
        self.pixels[i..i + 4].try_into().expect("RGBA pixel")
    }
    pub fn set_pixel(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        let i = self.index(x, y);
        self.pixels[i..i + 4].copy_from_slice(&rgba);
    }
    pub fn fill_rect(&mut self, x: u32, y: u32, width: u32, height: u32, rgba: [u8; 4]) {
        for py in y..y.saturating_add(height).min(self.height) {
            for px in x..x.saturating_add(width).min(self.width) {
                self.set_pixel(px, py, rgba);
            }
        }
    }
    pub fn from_png(bytes: &[u8]) -> Result<Self, String> {
        let mut decoder = png::Decoder::new(Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
        let len = reader
            .output_buffer_size()
            .ok_or("PNG output exceeds address space")?;
        let mut buffer = vec![0; len];
        let info = reader.next_frame(&mut buffer).map_err(|e| e.to_string())?;
        if info.width == 0 || info.height == 0 {
            return Err("PNG dimensions must be positive".into());
        }
        let mut image = Self::new(info.width, info.height);
        let data = &buffer[..info.buffer_size()];
        match info.color_type {
            png::ColorType::Rgba => image.pixels.copy_from_slice(data),
            png::ColorType::Rgb => {
                for (src, dst) in data
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .zip(image.pixels.as_chunks_mut::<4>().0.iter_mut())
                {
                    dst.copy_from_slice(&[src[0], src[1], src[2], 255]);
                }
            }
            png::ColorType::Grayscale => {
                for (&gray, dst) in data
                    .iter()
                    .zip(image.pixels.as_chunks_mut::<4>().0.iter_mut())
                {
                    dst.copy_from_slice(&[gray, gray, gray, 255]);
                }
            }
            png::ColorType::GrayscaleAlpha => {
                for (src, dst) in data
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .zip(image.pixels.as_chunks_mut::<4>().0.iter_mut())
                {
                    dst.copy_from_slice(&[src[0], src[0], src[0], src[1]]);
                }
            }
            png::ColorType::Indexed => return Err("PNG palette was not expanded".into()),
        }
        Ok(image)
    }
    pub fn to_png(&self) -> Result<Vec<u8>, String> {
        if self.width == 0
            || self.height == 0
            || u64::from(self.width) * u64::from(self.height) * 4 != self.pixels.len() as u64
        {
            return Err("invalid RGBA image dimensions or pixel length".into());
        }
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, self.width, self.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
            writer
                .write_image_data(&self.pixels)
                .map_err(|e| e.to_string())?;
        }
        Ok(bytes)
    }
    pub fn crop(&self, x: u32, y: u32, width: u32, height: u32) -> Option<Self> {
        if width == 0
            || height == 0
            || x.checked_add(width)? > self.width
            || y.checked_add(height)? > self.height
        {
            return None;
        }
        let mut result = Self::new(width, height);
        for row in 0..height {
            let src = self.index(x, y + row);
            let dst = row as usize * width as usize * 4;
            result.pixels[dst..dst + width as usize * 4]
                .copy_from_slice(&self.pixels[src..src + width as usize * 4]);
        }
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn png_bytes(color: png::ColorType, data: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 2, 2);
            encoder.set_color(color);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(data)
                .unwrap();
        }
        bytes
    }
    #[test]
    fn decode_color_models_and_roundtrip() {
        for (color, data, pixel) in [
            (
                png::ColorType::Rgba,
                [10, 20, 30, 40].repeat(4),
                [10, 20, 30, 40],
            ),
            (
                png::ColorType::Rgb,
                [10, 20, 30].repeat(4),
                [10, 20, 30, 255],
            ),
            (png::ColorType::Grayscale, vec![10; 4], [10, 10, 10, 255]),
            (
                png::ColorType::GrayscaleAlpha,
                [10, 40].repeat(4),
                [10, 10, 10, 40],
            ),
        ] {
            let image = RgbaImage::from_png(&png_bytes(color, &data)).unwrap();
            assert_eq!(image.pixel(1, 1), pixel);
            assert_eq!(
                RgbaImage::from_png(&image.to_png().unwrap()).unwrap(),
                image
            );
            assert_eq!(image.crop(1, 1, 1, 1).unwrap().pixel(0, 0), pixel);
            assert!(image.crop(1, 1, 2, 1).is_none());
            assert!(image.crop(u32::MAX, 0, 1, 1).is_none());
        }
        assert!(RgbaImage::from_png(b"broken").is_err());
    }
}
