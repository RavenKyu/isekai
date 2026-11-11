//! A smoke-only label strip, measured after the normal GPU render and readback.
//!
//! Each row ends with the same family's real missing-glyph rendering. Comparing
//! the full masks (rather than light counts) rejects tofu even when it draws pixels.

use egui::{Align2, Color32, Context, FontFamily, FontId, LayerId, Order, Rect, pos2, vec2};

const CELL: f32 = 64.0;
const ORIGIN: egui::Pos2 = pos2(16.0, 160.0);
const SAMPLES: [char; 5] = ['A', 'é', '가', '힣', '\u{10FFFF}'];

pub struct GlyphProbe {
    pixels_per_point: f32,
}

impl GlyphProbe {
    pub fn draw(ctx: &Context) -> Self {
        let painter = ctx.layer_painter(LayerId::new(
            Order::Foreground,
            egui::Id::unique("glyph probe"),
        ));
        // Snap the grid in physical pixels before converting to points. At a
        // fractional display scale, equal point offsets could shift identical
        // tofu masks by a pixel and make them look different to the checker.
        let pixels_per_point = ctx.pixels_per_point();
        let cell = (CELL * pixels_per_point).round();
        let origin = pos2(
            (ORIGIN.x * pixels_per_point).round(),
            (ORIGIN.y * pixels_per_point).round(),
        );
        for (row, family) in [FontFamily::Proportional, FontFamily::Monospace]
            .into_iter()
            .enumerate()
        {
            for (column, c) in SAMPLES.into_iter().enumerate() {
                let min =
                    (origin + vec2(column as f32 * cell, row as f32 * cell)) / pixels_per_point;
                painter.rect_filled(
                    Rect::from_min_size(min, vec2(cell, cell) / pixels_per_point),
                    0.,
                    Color32::BLACK,
                );
                painter.text(
                    min + vec2(8., 8.),
                    Align2::LEFT_TOP,
                    c,
                    FontId::new(32., family.clone()),
                    Color32::WHITE,
                );
            }
        }
        Self { pixels_per_point }
    }

    pub fn check(&self, pixels: &[u8], width: u32, height: u32) -> Result<(), String> {
        if pixels.len() != width as usize * height as usize * 4 {
            return Err("glyph probe: invalid framebuffer length".into());
        }
        let cell = (CELL * self.pixels_per_point).round() as usize;
        let x = (ORIGIN.x * self.pixels_per_point).round() as usize;
        let y = (ORIGIN.y * self.pixels_per_point).round() as usize;
        if cell == 0 || x + 5 * cell > width as usize || y + 2 * cell > height as usize {
            return Err("glyph probe: label strip outside framebuffer".into());
        }
        let mask = |row: usize, column: usize| -> Vec<bool> {
            (0..cell)
                .flat_map(|dy| {
                    (0..cell).map(move |dx| {
                        let i =
                            ((y + row * cell + dy) * width as usize + x + column * cell + dx) * 4;
                        pixels[i..i + 3].iter().all(|&v| v > 128)
                    })
                })
                .collect()
        };
        for row in 0..2 {
            let tofu = mask(row, 4);
            let tofu_lit = tofu.iter().filter(|&&p| p).count();
            if tofu_lit == 0 {
                return Err(format!(
                    "glyph probe: missing replacement control in family {row}"
                ));
            }
            for (column, c) in SAMPLES[..4].iter().enumerate() {
                let actual = mask(row, column);
                let lit = actual.iter().filter(|&&p| p).count();
                let different = actual.iter().zip(&tofu).filter(|(a, b)| a != b).count();
                // Permit small raster alignment differences, never a box with
                // the same coverage. Both native and web use this threshold.
                if lit == 0 || different <= (lit + tofu_lit) / 10 {
                    return Err(format!(
                        "glyph probe: {c:?} is blank or tofu in family {row}"
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lit_boxes_cannot_pass_as_glyphs() {
        let probe = GlyphProbe {
            pixels_per_point: 1.,
        };
        let (width, height) = (400, 320);
        let mut pixels = vec![0; width * height * 4];
        for row in 0..2 {
            for column in 0..5 {
                for dy in 8..40 {
                    for dx in 8..30 {
                        if dx == 8 || dx == 29 || dy == 8 || dy == 39 {
                            let i = ((160 + row * 64 + dy) * width + 16 + column * 64 + dx) * 4;
                            pixels[i..i + 3].fill(255);
                        }
                    }
                }
            }
        }
        assert!(
            probe
                .check(&pixels, width as u32, height as u32)
                .unwrap_err()
                .contains("tofu")
        );
        pixels.fill(0);
        assert!(probe.check(&pixels, width as u32, height as u32).is_err());
        assert!(probe.check(&pixels, 1, 1).is_err());
    }
}
