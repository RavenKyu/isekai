use isekai_core::{
    raster::RgbaImage,
    sprite::{SpriteSheet, Timeline, format::Part},
};
use isekai_render::{Gpu, SheetTexture, SpriteBatch, SpriteInstance, SpritePipeline};
pub struct Playback {
    pub texture: SheetTexture,
    sheet: SpriteSheet,
    animation_index: usize,
    timeline: Timeline,
    parts: Vec<Vec<Part>>,
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
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open_native(
        sidecar: &std::path::Path,
        animation: Option<&str>,
        gpu: &Gpu,
        pipeline: &SpritePipeline,
    ) -> Result<Self, String> {
        let loaded = isekai_core::sprite::store::load(sidecar).map_err(|e| e.to_string())?;
        if let Some(warning) = isekai_core::sprite::store::dropped_tile_animation_warning(
            &loaded.dropped_tile_animations,
        ) {
            eprintln!("isekai: warning: {warning}");
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
        Self::from_image(image, loaded.sheet, animation, gpu, pipeline)
    }

    pub fn from_image(
        image: RgbaImage,
        sheet: SpriteSheet,
        animation: Option<&str>,
        gpu: &Gpu,
        pipeline: &SpritePipeline,
    ) -> Result<Self, String> {
        sheet
            .validate_bounds(image.width, image.height)
            .map_err(|e| e.join("; "))?;
        let animation_index = select_animation(&sheet, animation)?;
        let a = &sheet.animations[animation_index];
        let timeline = Timeline::new(a.frames.iter().map(|f| f.ms).collect(), a.mode)?;
        let texture = SheetTexture::upload(gpu, &pipeline.layout, &image)?;
        let parts = sheet.sprites.iter().map(|s| s.parts(&sheet.grid)).collect();
        Ok(Self {
            texture,
            sheet,
            animation_index,
            timeline,
            parts,
            started_ms: None,
        })
    }
    pub fn fill(
        &mut self,
        now_ms: u64,
        width: u32,
        height: u32,
        batch: &mut SpriteBatch,
    ) -> Result<(), String> {
        if width == 0 || height == 0 {
            return Ok(());
        }
        let started = *self.started_ms.get_or_insert(now_ms);
        let frame = self.timeline.frame_at(now_ms.saturating_sub(started));
        let id = self.sheet.animations[self.animation_index].frames[frame].sprite_id;
        let index = self
            .sheet
            .sprites
            .iter()
            .position(|s| s.id == id)
            .ok_or_else(|| format!("no sprite with id {id}"))?;
        let sprite = &self.sheet.sprites[index];
        let (_, sprite_h) = sprite.bounds(&self.sheet.grid);
        let scale = (height / 4 / sprite_h.max(1)).max(1) as f32;
        let tile = self.sheet.grid.tile as f32 * scale;
        for part in &self.parts[index] {
            let x = width as f32 / 2.0 + (part.dx as f32 - sprite.pivot[0]) * scale;
            let y = height as f32 / 2.0 + (part.dy as f32 - sprite.pivot[1]) * scale;
            let dest = [
                2.0 * x / width as f32 - 1.0,
                1.0 - 2.0 * y / height as f32,
                2.0 * tile / width as f32,
                -2.0 * tile / height as f32,
            ];
            let (uv0, uv1) = self.sheet.grid.uv(
                part.col,
                part.row,
                self.texture.texture.width(),
                self.texture.texture.height(),
            );
            batch.push(
                0,
                SpriteInstance {
                    dest,
                    uv: part.transform().uv_corners(uv0, uv1),
                },
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires a GPU adapter; run scripts/smoke-native-isekai.sh"]
    fn example_scale_pivot_and_hole() {
        let gpu = pollster::block_on(Gpu::new(Gpu::instance(), None)).unwrap();
        let pipeline = SpritePipeline::new(&gpu);
        let (image, sheet) = isekai_core::sprite::example();
        let mut play = Playback::from_image(image, sheet, Some("cycle"), &gpu, &pipeline).unwrap();
        let target = isekai_render::FrameTarget::new(&gpu, 1280, 720);
        let mut batch = SpriteBatch::new(&gpu);
        play.fill(0, 1280, 720, &mut batch).unwrap();
        batch.upload(&gpu);
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            batch.draw(&mut pass, &pipeline, &[&play.texture]);
        }
        gpu.queue.submit([encoder.finish()]);
        let read = isekai_render::Readback::begin(&gpu, &target);
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
        let pixels = read.try_finish().unwrap().unwrap();
        let black =
            |x: usize, y: usize| pixels[(y * 1280 + x) * 4..(y * 1280 + x) * 4 + 3] == [0, 0, 0];
        for (x, y) in [
            (551, 272),
            (552, 271),
            (728, 272),
            (552, 448),
            (618, 338),
            (640, 360),
            (661, 381),
        ] {
            assert!(black(x, y), "expected black at {x},{y}");
        }
        for (x, y) in [
            (552, 272),
            (727, 447),
            (617, 360),
            (662, 360),
            (640, 337),
            (640, 382),
        ] {
            assert!(!black(x, y), "expected sprite at {x},{y}");
        }
    }
    #[test]
    fn animation_selection_is_named_or_lexical() {
        let (_, mut sheet) = isekai_core::sprite::example();
        assert_eq!(select_animation(&sheet, Some("cycle")).unwrap(), 0);
        assert!(
            select_animation(&sheet, Some("missing"))
                .unwrap_err()
                .contains("no animation named")
        );
        sheet.animations.clear();
        assert_eq!(
            select_animation(&sheet, None).unwrap_err(),
            "sprite sheet has no animations"
        );
    }
}
