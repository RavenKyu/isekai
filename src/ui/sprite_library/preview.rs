use super::SheetEditor;
use crate::sprite::{Grid, Mode, Timeline, format::Part};
use dear_imgui_rs::{TextureId, Ui};

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum PreviewTarget {
    #[default]
    None,
    Clip(String),
    Tile(u32, u32),
    Sprite(String),
}
pub struct PreviewState {
    pub target: PreviewTarget,
    pub playing: bool,
    started_ms: u64,
    elapsed_base: u64,
    timeline: Option<Timeline>,
    signature: Option<(Vec<u32>, Mode)>,
    pub zoom: u32,
}
impl Default for PreviewState {
    fn default() -> Self {
        Self {
            target: PreviewTarget::None,
            playing: true,
            started_ms: 0,
            elapsed_base: 0,
            timeline: None,
            signature: None,
            zoom: 3,
        }
    }
}
impl PreviewState {
    pub fn elapsed(&self, now: u64) -> u64 {
        self.elapsed_base.saturating_add(if self.playing {
            now.saturating_sub(self.started_ms)
        } else {
            0
        })
    }
    pub fn pause_or_play(&mut self, now: u64) {
        self.elapsed_base = self.elapsed(now);
        self.started_ms = now;
        self.playing = !self.playing;
    }
    pub fn restart(&mut self, now: u64) {
        self.started_ms = now;
        self.elapsed_base = 0;
    }
    pub fn select(&mut self, target: PreviewTarget, now: u64) {
        if self.target != target {
            self.target = target;
            self.restart(now);
            self.signature = None;
            self.timeline = None;
        }
    }
    pub fn update_timeline(&mut self, durations: Vec<u32>, mode: Mode) -> Result<(), String> {
        let signature = (durations, mode);
        if self.signature.as_ref() != Some(&signature) {
            self.timeline = Some(Timeline::new(signature.0.clone(), mode)?);
            self.signature = Some(signature);
        }
        Ok(())
    }
    pub fn frame(&self, now: u64) -> usize {
        self.timeline
            .as_ref()
            .map_or(0, |t| t.frame_at(self.elapsed(now)))
    }
}

pub struct PartView {
    pub id: TextureId,
    pub grid: Grid,
    pub width: u32,
    pub height: u32,
}
pub fn draw_parts(ui: &Ui, view: &PartView, parts: &[Part], origin: [f32; 2], zoom: u32) {
    let min_x = parts.iter().map(|p| p.dx).min().unwrap_or(0);
    let min_y = parts.iter().map(|p| p.dy).min().unwrap_or(0);
    let scale = zoom as f32;
    let draw = ui.get_window_draw_list();
    for part in parts {
        let (uv0, uv1) = view.grid.uv(part.col, part.row, view.width, view.height);
        let min = [
            origin[0] + (i64::from(part.dx) - i64::from(min_x)) as f32 * scale,
            origin[1] + (i64::from(part.dy) - i64::from(min_y)) as f32 * scale,
        ];
        let max = [
            min[0] + view.grid.tile as f32 * scale,
            min[1] + view.grid.tile as f32 * scale,
        ];
        draw.add_image(view.id, min, max, uv0, uv1, [1.0; 4]);
    }
}
impl SheetEditor {
    pub fn preview_tab(&mut self, ui: &Ui, now: u64) {
        let sheet = &self.sheet;
        let mut choices = vec![("None".to_string(), PreviewTarget::None)];
        for clip in &sheet.meta.clips {
            choices.push((
                format!("Clip: {}", clip.name),
                PreviewTarget::Clip(clip.name.clone()),
            ));
        }
        for tile in &sheet.meta.tiles {
            if tile.animation.is_some() {
                choices.push((
                    format!("Tile: ({}, {})", tile.col, tile.row),
                    PreviewTarget::Tile(tile.col, tile.row),
                ));
            }
        }
        for sprite in &sheet.meta.sprites {
            choices.push((
                format!("Sprite: {}", sprite.name),
                PreviewTarget::Sprite(sprite.name.clone()),
            ));
        }
        let mut index = choices
            .iter()
            .position(|(_, t)| *t == self.preview.target)
            .unwrap_or(0);
        let labels: Vec<_> = choices.iter().map(|(label, _)| label.as_str()).collect();
        if ui.combo_simple_string("Preview target", &mut index, &labels) {
            self.preview.select(choices[index].1.clone(), now);
        }
        if ui.button(if self.preview.playing {
            "Pause"
        } else {
            "Play"
        }) {
            self.preview.pause_or_play(now);
        }
        ui.same_line();
        if ui.button("Restart") {
            self.preview.restart(now);
        }
        ui.set_next_item_width(200.0);
        ui.slider("Preview zoom", 1, 8, &mut self.preview.zoom);
        let mut parts = Vec::new();
        let mut pivot = [0.0; 2];
        let mut bounds = (0, 0);
        let mut frame_count = 1;
        let result: Result<(), String> = (|| {
            let sprite = match &self.preview.target {
                PreviewTarget::Clip(name) => {
                    let clip = sheet
                        .meta
                        .clips
                        .iter()
                        .find(|c| c.name == *name)
                        .ok_or("Select an existing clip")?;
                    self.preview
                        .update_timeline(clip.frames.iter().map(|f| f.ms).collect(), clip.mode)?;
                    frame_count = clip.frames.len();
                    let frame = &clip.frames[self.preview.frame(now)];
                    Some(
                        sheet
                            .meta
                            .sprites
                            .iter()
                            .find(|s| s.name == frame.sprite)
                            .ok_or("clip refers to a missing sprite")?,
                    )
                }
                PreviewTarget::Sprite(name) => Some(
                    sheet
                        .meta
                        .sprites
                        .iter()
                        .find(|s| s.name == *name)
                        .ok_or("Select an existing sprite")?,
                ),
                PreviewTarget::Tile(col, row) => {
                    let animation = sheet
                        .meta
                        .tiles
                        .iter()
                        .find(|t| t.col == *col && t.row == *row)
                        .and_then(|t| t.animation.as_ref())
                        .ok_or("Select an animated tile")?;
                    self.preview.update_timeline(
                        animation.frames.iter().map(|f| f.ms).collect(),
                        animation.mode,
                    )?;
                    frame_count = animation.frames.len();
                    let frame = &animation.frames[self.preview.frame(now)];
                    parts.push(Part {
                        col: frame.col,
                        row: frame.row,
                        dx: 0,
                        dy: 0,
                    });
                    bounds = (sheet.meta.grid.tile, sheet.meta.grid.tile);
                    pivot = [bounds.0 as f32 / 2.0, bounds.1 as f32 / 2.0];
                    None
                }
                PreviewTarget::None => None,
            };
            if let Some(sprite) = sprite {
                parts = sprite.parts(&sheet.meta.grid);
                bounds = sprite.bounds(&sheet.meta.grid);
                pivot = sprite.pivot;
            }
            Ok(())
        })();
        if let Err(error) = result {
            ui.text_wrapped(error);
            return;
        }
        if parts.is_empty() {
            ui.text("Select a clip, an animated tile, or a sprite");
            return;
        }
        let moving = matches!(
            self.preview.target,
            PreviewTarget::Clip(_) | PreviewTarget::Tile(..)
        );
        let frame = if moving { self.preview.frame(now) } else { 0 };
        ui.text(format!(
            "Frame {} / {}, elapsed {} ms{}",
            frame + 1,
            frame_count,
            self.preview.elapsed(now),
            if moving
                && self
                    .preview
                    .timeline
                    .as_ref()
                    .is_some_and(|t| t.finished_at(self.preview.elapsed(now)))
            {
                " (finished)"
            } else {
                ""
            }
        ));
        let cursor = ui.cursor_screen_pos();
        let origin = [cursor[0] + 16.0, cursor[1] + 16.0];
        let scale = self.preview.zoom as f32;
        let view = PartView {
            id: sheet.imgui_id,
            grid: sheet.meta.grid,
            width: sheet.image.width,
            height: sheet.image.height,
        };
        draw_parts(ui, &view, &parts, origin, self.preview.zoom);
        let cross = [origin[0] + pivot[0] * scale, origin[1] + pivot[1] * scale];
        let draw = ui.get_window_draw_list();
        draw.add_line(
            [cross[0] - 5.0, cross[1]],
            [cross[0] + 5.0, cross[1]],
            [1.0, 0.3, 0.2, 1.0],
        )
        .build();
        draw.add_line(
            [cross[0], cross[1] - 5.0],
            [cross[0], cross[1] + 5.0],
            [1.0, 0.3, 0.2, 1.0],
        )
        .build();
        ui.dummy([
            bounds.0 as f32 * scale + 32.0,
            bounds.1 as f32 * scale + 32.0,
        ]);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn playback_clock_pause_restart_and_timeline_edits() {
        let mut state = PreviewState::default();
        state.select(PreviewTarget::Clip("test".into()), 1000);
        state.update_timeline(vec![100, 100], Mode::Loop).unwrap();
        assert_eq!(state.frame(1150), 1);
        state.pause_or_play(1150);
        assert_eq!(state.frame(10000), 1);
        state.restart(20000);
        assert_eq!(state.frame(30000), 0);
        state.pause_or_play(30000);
        assert_eq!(state.frame(30150), 1);
        state.update_timeline(vec![100, 100], Mode::Once).unwrap();
        assert_eq!(state.frame(40000), 1);
        assert!(
            state
                .timeline
                .as_ref()
                .unwrap()
                .finished_at(state.elapsed(40000))
        );
        state.restart(40000);
        assert_eq!(state.frame(40000), 0);
        state.update_timeline(vec![200, 200], Mode::Loop).unwrap();
        assert_eq!(state.frame(40150), 0);
        assert_eq!(state.elapsed(40150), 150);
    }
}
