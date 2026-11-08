use super::{SpriteSheet, Timeline, draw::SpriteQuad, texture::SheetTexture};
use crate::raster::RgbaImage;
use std::rc::Rc;

pub struct Playback {
    texture: SheetTexture,
    quad: SpriteQuad,
    sheet: SpriteSheet,
    clip_index: usize,
    timeline: Timeline,
    started_ms: Option<u64>,
}

fn select_clip(sheet: &SpriteSheet, name: Option<&str>) -> Result<usize, String> {
    match name {
        Some(name) => sheet
            .clips
            .iter()
            .position(|clip| clip.name == name)
            .ok_or_else(|| format!("no clip named {name:?}")),
        None => sheet
            .clips
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.name.cmp(&b.name))
            .map(|(index, _)| index)
            .ok_or_else(|| "sprite sheet has no clips".into()),
    }
}

impl Playback {
    #[cfg(not(target_os = "emscripten"))]
    pub fn open_native(
        sidecar: &std::path::Path,
        clip: Option<&str>,
        gl: &Rc<glow::Context>,
    ) -> Result<Self, String> {
        let loaded = super::store::load(sidecar).map_err(|e| e.to_string())?;
        let parent = sidecar
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(std::path::Path::new("."));
        if loaded.image_path.parent() != Some(parent) {
            return Err(format!(
                "image {} must share sidecar directory {}",
                loaded.image_path.display(),
                parent.display()
            ));
        }
        let bytes = std::fs::read(&loaded.image_path)
            .map_err(|e| format!("image {}: {e}", loaded.image_path.display()))?;
        let mut image = RgbaImage::from_png(&bytes)?;
        image.normalize_alpha();
        Self::from_image(image, loaded.sheet, clip, gl)
    }

    #[cfg(target_os = "emscripten")]
    pub fn open_embedded(
        name: &str,
        clip: Option<&str>,
        gl: &Rc<glow::Context>,
    ) -> Result<Self, String> {
        let (image, sheet) = super::embedded::lookup(name)
            .ok_or_else(|| format!("no embedded sprite sheet named {name:?}"))?;
        Self::from_image(image, sheet, clip, gl)
    }

    fn from_image(
        image: RgbaImage,
        sheet: SpriteSheet,
        clip: Option<&str>,
        gl: &Rc<glow::Context>,
    ) -> Result<Self, String> {
        sheet
            .validate_bounds(image.width, image.height)
            .map_err(|errors| errors.join("; "))?;
        let clip_index = select_clip(&sheet, clip)?;
        let clip = &sheet.clips[clip_index];
        let timeline = Timeline::new(clip.frames.iter().map(|f| f.ms).collect(), clip.mode)?;
        let texture = SheetTexture::upload(gl, &image)?;
        let quad = SpriteQuad::new(gl)?;
        Ok(Self {
            texture,
            quad,
            sheet,
            clip_index,
            timeline,
            started_ms: None,
        })
    }

    pub fn draw(&mut self, now_ms: u64, width: u32, height: u32) -> Result<(), String> {
        if width == 0 || height == 0 {
            return Ok(());
        }
        let started = *self.started_ms.get_or_insert(now_ms);
        let frame = self.timeline.frame_at(now_ms.saturating_sub(started));
        let name = &self.sheet.clips[self.clip_index].frames[frame].sprite;
        let sprite = self
            .sheet
            .sprites
            .iter()
            .find(|sprite| sprite.name == *name)
            .ok_or_else(|| format!("no sprite named {name:?}"))?;
        let (_, sprite_h) = sprite.bounds(&self.sheet.grid);
        let scale = (height / 4 / sprite_h.max(1)).max(1) as f32;
        let (origin_x, origin_y) = sprite.bounds_origin();
        let tile = self.sheet.grid.tile as f32 * scale;
        for part in sprite.parts(&self.sheet.grid) {
            let x =
                width as f32 / 2.0 + (part.dx as f32 - origin_x as f32 - sprite.pivot[0]) * scale;
            let y =
                height as f32 / 2.0 + (part.dy as f32 - origin_y as f32 - sprite.pivot[1]) * scale;
            let dest = [
                2.0 * x / width as f32 - 1.0,
                1.0 - 2.0 * y / height as f32,
                2.0 * tile / width as f32,
                -2.0 * tile / height as f32,
            ];
            let (uv0, uv1) =
                self.sheet
                    .grid
                    .uv(part.col, part.row, self.texture.width, self.texture.height);
            self.quad
                .draw(self.texture.raw(), dest, [uv0[0], uv0[1], uv1[0], uv1[1]])?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clip_selection_is_named_or_lexical() {
        let (_, mut sheet) = super::super::example();
        assert_eq!(select_clip(&sheet, None), Ok(0));
        assert!(select_clip(&sheet, Some("missing")).is_err());
        let mut another = sheet.clips[0].clone();
        another.name = "aaa".into();
        sheet.clips.push(another);
        assert_eq!(select_clip(&sheet, None), Ok(1));
        assert_eq!(select_clip(&sheet, Some("cycle")), Ok(0));
        sheet.clips.clear();
        assert!(select_clip(&sheet, None).is_err());
    }
}
