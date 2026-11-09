use super::{SpriteSheet, Timeline, draw::SpriteQuad, texture::SheetTexture};
use crate::raster::RgbaImage;
use std::rc::Rc;

pub struct Playback {
    texture: SheetTexture,
    quad: SpriteQuad,
    sheet: SpriteSheet,
    animation_index: usize,
    timeline: Timeline,
    started_ms: Option<u64>,
}

fn select_animation(sheet: &SpriteSheet, name: Option<&str>) -> Result<usize, String> {
    match name {
        Some(name) => sheet
            .animations
            .iter()
            .position(|animation| animation.name == name)
            .ok_or_else(|| format!("no animation named {name:?}")),
        None => sheet
            .animations
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.name.cmp(&b.name))
            .map(|(index, _)| index)
            .ok_or_else(|| "sprite sheet has no animations".into()),
    }
}

impl Playback {
    #[cfg(not(target_os = "emscripten"))]
    pub fn open_native(
        sidecar: &std::path::Path,
        animation: Option<&str>,
        gl: &Rc<glow::Context>,
    ) -> Result<Self, String> {
        let loaded = super::store::load(sidecar).map_err(|e| e.to_string())?;
        if let Some(warning) =
            super::store::dropped_tile_animation_warning(&loaded.dropped_tile_animations)
        {
            eprintln!("rusty-nails: warning: {warning}");
        }
        let parent = sidecar
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(std::path::Path::new("."));
        if parent.join(&loaded.sheet.image).parent() != Some(parent) {
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
        Self::from_image(image, loaded.sheet, animation, gl)
    }

    #[cfg(target_os = "emscripten")]
    pub fn open_embedded(
        name: &str,
        animation: Option<&str>,
        gl: &Rc<glow::Context>,
    ) -> Result<Self, String> {
        let (image, sheet) = super::embedded::lookup(name)
            .ok_or_else(|| format!("no embedded sprite sheet named {name:?}"))?;
        Self::from_image(image, sheet, animation, gl)
    }

    fn from_image(
        image: RgbaImage,
        sheet: SpriteSheet,
        animation: Option<&str>,
        gl: &Rc<glow::Context>,
    ) -> Result<Self, String> {
        sheet
            .validate_bounds(image.width, image.height)
            .map_err(|errors| errors.join("; "))?;
        let animation_index = select_animation(&sheet, animation)?;
        let animation = &sheet.animations[animation_index];
        let timeline = Timeline::new(
            animation.frames.iter().map(|f| f.ms).collect(),
            animation.mode,
        )?;
        let texture = SheetTexture::upload(gl, &image)?;
        let quad = SpriteQuad::new(gl)?;
        Ok(Self {
            texture,
            quad,
            sheet,
            animation_index,
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
        let id = self.sheet.animations[self.animation_index].frames[frame].sprite_id;
        let sprite = self
            .sheet
            .sprites
            .iter()
            .find(|sprite| sprite.id == id)
            .ok_or_else(|| format!("no sprite with id {id}"))?;
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
            self.quad.draw(
                self.texture.raw(),
                dest,
                part.transform().uv_corners(uv0, uv1),
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn animation_selection_is_named_or_lexical() {
        let (_, mut sheet) = super::super::example();
        assert_eq!(select_animation(&sheet, None), Ok(0));
        assert!(select_animation(&sheet, Some("missing")).is_err());
        let mut another = sheet.animations[0].clone();
        another.name = "aaa".into();
        sheet.animations.push(another);
        assert_eq!(select_animation(&sheet, None), Ok(1));
        assert_eq!(select_animation(&sheet, Some("cycle")), Ok(0));
        sheet.animations.clear();
        assert!(select_animation(&sheet, None).is_err());
    }
}
