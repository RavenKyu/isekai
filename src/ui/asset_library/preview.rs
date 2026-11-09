use super::{EditorTab, SheetEditor};
use crate::sprite::{Grid, Mode, Timeline, format::Part};
use dear_imgui_rs::{TextureId, Ui};

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum PreviewTarget {
    #[default]
    None,
    Selection,
    Animation(u64),
    Sprite(u64),
}
pub struct PreviewState {
    pub target: PreviewTarget,
    pub playing: bool,
    started_ms: u64,
    elapsed_base: u64,
    timeline: Option<Timeline>,
    signature: Option<(Vec<u32>, Mode)>,
    #[cfg(test)]
    builds: u32,
    #[cfg(test)]
    pub(super) drawn_parts: usize,
    pub zoom: u32,
    pub show_pivot: bool,
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
            #[cfg(test)]
            builds: 0,
            #[cfg(test)]
            drawn_parts: 0,
            zoom: 3,
            show_pivot: true,
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
            #[cfg(test)]
            {
                self.builds += 1;
            }
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
    draw_parts_scaled(ui, view, parts, origin, zoom as f32);
}
fn draw_parts_scaled(ui: &Ui, view: &PartView, parts: &[Part], origin: [f32; 2], scale: f32) {
    let draw = ui.get_window_draw_list();
    for part in parts {
        let (uv0, uv1) = view.grid.uv(part.col, part.row, view.width, view.height);
        let min = part_position(part, origin, scale);
        let max = [
            min[0] + view.grid.tile as f32 * scale,
            min[1] + view.grid.tile as f32 * scale,
        ];
        let [(p1, uv1), (p2, uv2), (p3, uv3), (p4, uv4)] =
            image_quad(min, max, part.transform().uv_corners(uv0, uv1));
        draw.add_image_quad(view.id, p1, p2, p3, p4, uv1, uv2, uv3, uv4, [1.0; 4]);
    }
}
fn part_position(part: &Part, origin: [f32; 2], scale: f32) -> [f32; 2] {
    [
        origin[0] + part.dx as f32 * scale,
        origin[1] + part.dy as f32 * scale,
    ]
}
/// imgui walks the quad clockwise from top-left; the shared corners use
/// triangle-strip order (top-left, top-right, bottom-left, bottom-right).
fn image_quad(min: [f32; 2], max: [f32; 2], corners: [[f32; 2]; 4]) -> [([f32; 2], [f32; 2]); 4] {
    [
        (min, corners[0]),
        ([max[0], min[1]], corners[1]),
        (max, corners[3]),
        ([min[0], max[1]], corners[2]),
    ]
}
fn selection_renderable(len: usize) -> bool {
    len <= 256
}
fn preview_target(
    tab: EditorTab,
    selection_len: usize,
    sprite: Option<u64>,
    animation: Option<u64>,
) -> PreviewTarget {
    match tab {
        EditorTab::Tileset | EditorTab::Import => {
            if selection_len == 0 {
                PreviewTarget::None
            } else {
                PreviewTarget::Selection
            }
        }
        EditorTab::Sprites => sprite.map_or(PreviewTarget::None, PreviewTarget::Sprite),
        EditorTab::Animations => animation.map_or(PreviewTarget::None, PreviewTarget::Animation),
    }
}
impl SheetEditor {
    pub fn preview_panel(&mut self, ui: &Ui, now: u64) {
        #[cfg(test)]
        {
            self.preview.drawn_parts = 0;
        }
        let target = preview_target(
            self.active_tab,
            self.sheet.selection.cells.len(),
            self.selected_sprite,
            self.selected_animation,
        );
        self.preview.select(target, now);
        ui.text("Preview");
        if self.active_tab == EditorTab::Animations {
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
        }
        ui.set_next_item_width(140.0);
        ui.slider("Preview zoom", 1, 8, &mut self.preview.zoom);
        if matches!(self.active_tab, EditorTab::Sprites | EditorTab::Animations) {
            ui.checkbox("Pivot", &mut self.preview.show_pivot);
        }
        let sheet = &self.sheet;
        let view = PartView {
            id: sheet.imgui_id,
            grid: sheet.meta.grid,
            width: sheet.image.width,
            height: sheet.image.height,
        };
        if self.preview.target == PreviewTarget::Selection {
            let selection = &sheet.selection.cells;
            let min_col = selection.iter().map(|p| p.0).min().unwrap();
            let max_col = selection.iter().map(|p| p.0).max().unwrap();
            let min_row = selection.iter().map(|p| p.1).min().unwrap();
            let max_row = selection.iter().map(|p| p.1).max().unwrap();
            let cols = max_col - min_col + 1;
            let rows = max_row - min_row + 1;
            if selection.len() == 1 {
                ui.text(format!("Tile ({min_col}, {min_row})"));
                if let Some(tile) = sheet
                    .meta
                    .tiles
                    .iter()
                    .find(|t| t.col == min_col && t.row == min_row)
                {
                    if let Some(name) = &tile.name {
                        ui.text(name);
                    }
                    if tile.props.is_empty() {
                        ui.text_disabled("No properties");
                    } else {
                        for (k, v) in &tile.props {
                            ui.text(format!("{k}={v}"));
                        }
                    }
                } else {
                    ui.text_disabled("No properties");
                }
            } else {
                ui.text(format!("{} cells ({} x {})", selection.len(), cols, rows));
            }
            if !selection_renderable(selection.len()) {
                ui.text_disabled("Selection exceeds 256 cells");
                return;
            }
            let parts: Vec<_> = selection
                .iter()
                .map(|&(col, row)| Part {
                    col,
                    row,
                    dx: ((col - min_col) * sheet.meta.grid.tile) as i32,
                    dy: ((row - min_row) * sheet.meta.grid.tile) as i32,
                    ..Default::default()
                })
                .collect();
            let width = cols as f32 * sheet.meta.grid.tile as f32;
            let height = rows as f32 * sheet.meta.grid.tile as f32;
            let scale = (self.preview.zoom as f32)
                .min((ui.content_region_avail()[0].max(1.0) / width).max(0.01));
            let origin = ui.cursor_screen_pos();
            #[cfg(test)]
            {
                self.preview.drawn_parts = parts.len();
            }
            draw_parts_scaled(ui, &view, &parts, origin, scale);
            ui.dummy([width * scale, height * scale]);
            return;
        }
        let sprite = match self.preview.target {
            PreviewTarget::Sprite(id) => sheet.meta.sprites.iter().find(|s| s.id == id),
            PreviewTarget::Animation(id) => {
                let Some(animation) = sheet.meta.animations.iter().find(|a| a.id == id) else {
                    ui.text("Select an animation");
                    return;
                };
                ui.text(format!("{} ({:?})", animation.name, animation.mode));
                if animation.frames.is_empty() {
                    ui.text("No frames");
                    return;
                }
                if let Err(error) = self.preview.update_timeline(
                    animation.frames.iter().map(|f| f.ms).collect(),
                    animation.mode,
                ) {
                    ui.text_wrapped(error);
                    return;
                }
                let index = self.preview.frame(now);
                let frame = &animation.frames[index];
                let finished = self
                    .preview
                    .timeline
                    .as_ref()
                    .is_some_and(|t| t.finished_at(self.preview.elapsed(now)));
                ui.text(format!(
                    "Frame {} / {}, elapsed {} ms{}",
                    index + 1,
                    animation.frames.len(),
                    self.preview.elapsed(now),
                    if finished { " (finished)" } else { "" }
                ));
                sheet.meta.sprites.iter().find(|s| s.id == frame.sprite_id)
            }
            PreviewTarget::None => {
                ui.text(match self.active_tab {
                    EditorTab::Sprites => "Select or create a sprite",
                    EditorTab::Animations => "Select an animation",
                    _ => "No selection",
                });
                return;
            }
            PreviewTarget::Selection => unreachable!(),
        };
        let Some(sprite) = sprite else {
            ui.text("Select an existing sprite");
            return;
        };
        let (w, h) = sprite.bounds(&sheet.meta.grid);
        ui.text(format!("{}  {} x {} px", sprite.name, w, h));
        let origin = ui.cursor_screen_pos();
        let scale = self.preview.zoom as f32;
        let parts = sprite.parts(&sheet.meta.grid);
        #[cfg(test)]
        {
            self.preview.drawn_parts = parts.len();
        }
        draw_parts(ui, &view, &parts, origin, self.preview.zoom);
        if self.preview.show_pivot {
            let cross = [
                origin[0] + sprite.pivot[0] * scale,
                origin[1] + sprite.pivot[1] * scale,
            ];
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
        }
        ui.dummy([w as f32 * scale, h as f32 * scale]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tab_targets_selection_and_timeline_cache() {
        for tab in [EditorTab::Tileset, EditorTab::Import] {
            assert_eq!(
                preview_target(tab, 0, Some(1), Some(2)),
                PreviewTarget::None
            );
            assert_eq!(preview_target(tab, 1, None, None), PreviewTarget::Selection);
        }
        assert_eq!(
            preview_target(EditorTab::Sprites, 1, Some(3), Some(4)),
            PreviewTarget::Sprite(3)
        );
        assert_eq!(
            preview_target(EditorTab::Sprites, 1, None, Some(4)),
            PreviewTarget::None
        );
        assert_eq!(
            preview_target(EditorTab::Animations, 1, Some(3), Some(4)),
            PreviewTarget::Animation(4)
        );
        assert_eq!(
            preview_target(EditorTab::Animations, 1, Some(3), None),
            PreviewTarget::None
        );
        assert!(selection_renderable(256));
        assert!(!selection_renderable(257));
        let mut state = PreviewState::default();
        state.update_timeline(vec![10, 20], Mode::Loop).unwrap();
        state.update_timeline(vec![10, 20], Mode::Loop).unwrap();
        assert_eq!(state.builds, 1);
        state.update_timeline(vec![10, 21], Mode::Loop).unwrap();
        assert_eq!(state.builds, 2);
    }
    #[test]
    fn part_position_preserves_empty_canvas_edges() {
        let part = Part {
            dx: 16,
            dy: 0,
            ..Default::default()
        };
        assert_eq!(part_position(&part, [10.0, 20.0], 2.0), [42.0, 20.0]);
    }
    #[test]
    fn image_quad_corners_sample_the_transformed_pixels() {
        use crate::{raster::RgbaImage, sprite::Transform};
        let mut image = RgbaImage::new(2, 2);
        for (i, p) in image.pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            *p = [i as u8 + 1, 0, 0, 255];
        }
        for t in Transform::ALL {
            let pixels = image.transformed(t);
            let quad = image_quad([0.0, 0.0], [1.0, 1.0], t.uv_corners([0.0; 2], [1.0; 2]));
            // imgui's AddImageQuad expects p1..p4 clockwise from the top-left.
            let positions: Vec<_> = quad.iter().map(|(p, _)| *p).collect();
            assert_eq!(positions, [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
            for (pos, uv) in quad {
                assert_eq!(
                    image.pixel(uv[0] as u32, uv[1] as u32),
                    pixels.pixel(pos[0] as u32, pos[1] as u32),
                    "{t:?} at {pos:?}"
                );
            }
        }
    }
    #[test]
    fn playback_clock_pause_restart_and_timeline_edits() {
        let mut state = PreviewState::default();
        state.select(PreviewTarget::Animation(1), 1000);
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
