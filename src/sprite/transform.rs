use crate::raster::RgbaImage;

/// Screen-clockwise quarter turns, after a horizontal flip. The inverse pixel
/// mapping in `source` is the single definition used by pixels and UVs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Transform {
    flip_x: bool,
    rotation: u16,
}
impl Transform {
    pub const IDENTITY: Self = Self {
        flip_x: false,
        rotation: 0,
    };
    pub const ALL: [Self; 8] = [
        Self::IDENTITY,
        Self {
            flip_x: false,
            rotation: 90,
        },
        Self {
            flip_x: false,
            rotation: 180,
        },
        Self {
            flip_x: false,
            rotation: 270,
        },
        Self {
            flip_x: true,
            rotation: 0,
        },
        Self {
            flip_x: true,
            rotation: 90,
        },
        Self {
            flip_x: true,
            rotation: 180,
        },
        Self {
            flip_x: true,
            rotation: 270,
        },
    ];
    pub fn new(flip_x: bool, rotation: u16) -> Result<Self, String> {
        if !matches!(rotation, 0 | 90 | 180 | 270) {
            return Err(format!(
                "invalid rotation {rotation}; expected 0, 90, 180 or 270"
            ));
        }
        Ok(Self { flip_x, rotation })
    }
    pub fn flip_x(self) -> bool {
        self.flip_x
    }
    pub fn rotation(self) -> u16 {
        self.rotation
    }
    pub fn is_identity(self) -> bool {
        self == Self::IDENTITY
    }
    /// Output pixel (x,y) samples this input pixel of a square of side n.
    pub fn source(self, x: u32, y: u32, n: u32) -> (u32, u32) {
        assert!(x < n && y < n, "transform pixel outside square");
        let (sx, sy) = match self.rotation {
            0 => (x, y),
            90 => (y, n - 1 - x),
            180 => (n - 1 - x, n - 1 - y),
            270 => (n - 1 - y, x),
            _ => unreachable!("validated rotation"),
        };
        (if self.flip_x { n - 1 - sx } else { sx }, sy)
    }
}
impl RgbaImage {
    pub fn transformed(&self, transform: Transform) -> Self {
        assert_eq!(self.width, self.height, "transforms require square tiles");
        let mut output = Self::new(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let (sx, sy) = transform.source(x, y, self.width);
                output.set_pixel(x, y, self.pixel(sx, sy));
            }
        }
        output
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn eight_screen_clockwise_flip_first_mappings() {
        let mut image = RgbaImage::new(2, 2);
        for (i, pixel) in image.pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            pixel.copy_from_slice(&[i as u8 + 1, 0, 0, 255]);
        }
        let expected = [
            [1, 2, 3, 4],
            [3, 1, 4, 2],
            [4, 3, 2, 1],
            [2, 4, 1, 3],
            [2, 1, 4, 3],
            [4, 2, 3, 1],
            [3, 4, 1, 2],
            [1, 3, 2, 4],
        ];
        for (transform, expected) in Transform::ALL.into_iter().zip(expected) {
            let result = image.transformed(transform);
            let actual: Vec<_> = result
                .pixels
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| p[0])
                .collect();
            assert_eq!(actual, expected, "{transform:?}");
        }
        let mut image = RgbaImage::new(3, 3);
        for (i, pixel) in image.pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            pixel[0] = i as u8;
        }
        let variants: std::collections::HashSet<_> = Transform::ALL
            .into_iter()
            .map(|t| image.transformed(t).pixels)
            .collect();
        assert_eq!(variants.len(), 8);
        assert_eq!(image.transformed(Transform::IDENTITY), image);
        assert!(Transform::new(false, 45).is_err());
    }
}
