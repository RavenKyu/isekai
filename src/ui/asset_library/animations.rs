use super::{
    SheetEditor, edit, format_tags, log, parse_tags,
    preview::{self, PartView, PreviewTarget},
    props::PropsDraft,
    tileset::mode_combo,
    view::input_u32,
};
use crate::sprite::{Animation, Mode, Sprite, format::AnimationFrame};
use dear_imgui_rs::{ListClipper, Ui, WindowFlags};
#[derive(Default)]
pub struct AnimationDraft {
    id: Option<u64>,
    tags: String,
    props: PropsDraft,
}
impl AnimationDraft {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_some() {
            self.props.validate()?;
        }
        Ok(())
    }
}
fn animation_row(animation: &Animation) -> String {
    let mode = match animation.mode {
        Mode::Loop => "loop",
        Mode::Once => "once",
        Mode::PingPong => "pingpong",
    };
    let ms: u64 = animation.frames.iter().map(|f| u64::from(f.ms)).sum();
    format!("{mode} - {} frames - {ms} ms", animation.frames.len())
}
fn next_frame(
    animation: &Animation,
    selected: Option<u64>,
    sprites: &[Sprite],
) -> Option<AnimationFrame> {
    let id = selected
        .filter(|id| sprites.iter().any(|s| s.id == *id))
        .or_else(|| {
            sprites
                .iter()
                .min_by(|a, b| a.name.cmp(&b.name))
                .map(|s| s.id)
        })?;
    Some(AnimationFrame {
        sprite_id: id,
        ms: animation.frames.last().map_or(100, |f| f.ms),
    })
}
fn move_frame(frames: &mut [AnimationFrame], index: usize, delta: i32) -> bool {
    let Some(next) = index.checked_add_signed(delta as isize) else {
        return false;
    };
    if index >= frames.len() || next >= frames.len() {
        return false;
    }
    frames.swap(index, next);
    true
}
impl SheetEditor {
    pub fn animations_tab(&mut self, ui: &Ui, tick: u64) {
        let now = (self.now)();
        let sheet = &mut self.sheet;
        let create = ui.button("+ New animation");
        #[cfg(test)]
        {
            let min = ui.item_rect_min();
            let size = ui.item_rect_size();
            self.animation_new_input = [min[0] + size[0] * 0.5, min[1] + size[1] * 0.5];
        }
        if create {
            let name = edit::next_name(
                "animation",
                sheet.meta.animations.iter().map(|a| a.name.as_str()),
            );
            let animation = Animation {
                id: 0,
                name,
                mode: Mode::Loop,
                tags: vec![],
                props: Default::default(),
                frames: vec![],
                modified: None,
            };
            self.selected_animation = Some(sheet.meta.add_animation(animation, now));
            sheet.dirty = true;
            log::append(
                &mut self.log,
                "New animation needs at least one frame before Save".into(),
            );
        }
        ui.child_window("Animation list")
            .size([220.0, 0.0])
            .build(ui, || {
                let view = PartView {
                    id: sheet.imgui_id,
                    grid: sheet.meta.grid,
                    width: sheet.image.width,
                    height: sheet.image.height,
                };
                for index in ListClipper::new(sheet.meta.animations.len())
                    .items_height(54.0)
                    .begin(ui)
                    .iter()
                {
                    let animation = &sheet.meta.animations[index];
                    let row = ui.cursor_screen_pos();
                    let _id = ui.push_id(index as i32);
                    if ui
                        .selectable_config("##animation-row")
                        .selected(self.selected_animation == Some(animation.id))
                        .size([0.0, 50.0])
                        .build()
                    {
                        self.selected_animation = Some(animation.id);
                    }
                    if let Some(sprite) = animation
                        .frames
                        .first()
                        .and_then(|f| sheet.meta.sprites.iter().find(|s| s.id == f.sprite_id))
                    {
                        preview::draw_parts_fit(
                            ui,
                            &view,
                            &sprite.parts(&sheet.meta.grid),
                            sprite.bounds(&sheet.meta.grid),
                            [row[0] + 2.0, row[1] + 5.0],
                            [32.0, 32.0],
                        );
                    }
                    let draw = ui.get_window_draw_list();
                    draw.add_text([row[0] + 40.0, row[1] + 2.0], [1.0; 4], &animation.name);
                    draw.add_text(
                        [row[0] + 40.0, row[1] + 22.0],
                        [0.75, 0.75, 0.75, 1.0],
                        animation_row(animation),
                    );
                }
            });
        ui.same_line();
        ui.child_window("Animation details")
            .size([0.0, 0.0])
            .build(ui, || {
                let Some(index) = sheet
                    .meta
                    .animations
                    .iter()
                    .position(|a| Some(a.id) == self.selected_animation)
                else {
                    return;
                };
                let mut animation = sheet.meta.animations[index].clone();
                self.preview
                    .select(PreviewTarget::Animation(animation.id), tick);
                if self.animation_draft.id != Some(animation.id) {
                    self.animation_draft = AnimationDraft {
                        id: Some(animation.id),
                        tags: format_tags(&animation.tags),
                        props: PropsDraft::from_map(&animation.props),
                    };
                }
                ui.text(format!("ID {}", animation.id));
                let mut changed = ui.input_text("Animation name", &mut animation.name).build();
                changed |= mode_combo(ui, "Animation mode", &mut animation.mode);
                if ui
                    .input_text("Animation tags", &mut self.animation_draft.tags)
                    .build()
                {
                    animation.tags = parse_tags(&self.animation_draft.tags);
                    changed = true;
                }
                if let Some(props) = self.animation_draft.props.draw(ui, "animation-properties") {
                    animation.props = props;
                    changed = true;
                }
                ui.text("Frames");
                if animation.frames.is_empty() {
                    ui.text_disabled("Needs at least one frame to save");
                }
                let names: Vec<_> = sheet.meta.sprites.iter().map(|s| s.name.as_str()).collect();
                let view = PartView {
                    id: sheet.imgui_id,
                    grid: sheet.meta.grid,
                    width: sheet.image.width,
                    height: sheet.image.height,
                };
                let mut operation = None;
                let mut rects = Vec::new();
                let count = animation.frames.len();
                ui.child_window("Frame strip")
                    .size([0.0, 205.0])
                    .flags(WindowFlags::HORIZONTAL_SCROLLBAR)
                    .build(ui, || {
                        for (i, frame) in animation.frames.iter_mut().enumerate() {
                            if i > 0 {
                                ui.same_line();
                            }
                            let _id = ui.push_id(i as i32);
                            ui.group(|| {
                                let pos = ui.cursor_screen_pos();
                                ui.dummy([56.0, 56.0]);
                                rects.push((pos, [pos[0] + 56.0, pos[1] + 56.0]));
                                if let Some(sprite) =
                                    sheet.meta.sprites.iter().find(|s| s.id == frame.sprite_id)
                                {
                                    preview::draw_parts_fit(
                                        ui,
                                        &view,
                                        &sprite.parts(&sheet.meta.grid),
                                        sprite.bounds(&sheet.meta.grid),
                                        pos,
                                        [56.0, 56.0],
                                    );
                                }
                                let mut selected = sheet
                                    .meta
                                    .sprites
                                    .iter()
                                    .position(|s| s.id == frame.sprite_id)
                                    .unwrap_or(0);
                                ui.set_next_item_width(120.0);
                                if !names.is_empty()
                                    && ui.combo_simple_string("##sprite", &mut selected, &names)
                                {
                                    frame.sprite_id = sheet.meta.sprites[selected].id;
                                    changed = true;
                                }
                                ui.set_next_item_width(100.0);
                                changed |= input_u32(ui, "ms", &mut frame.ms, 1);
                                if ui.small_button("<") {
                                    operation = Some((i, 0));
                                }
                                ui.same_line();
                                let remove = ui.small_button("x");
                                #[cfg(test)]
                                {
                                    if i == 0 {
                                        let min = ui.item_rect_min();
                                        let size = ui.item_rect_size();
                                        self.frame_remove_input =
                                            [min[0] + size[0] * 0.5, min[1] + size[1] * 0.5];
                                    }
                                }
                                if remove {
                                    operation = Some((i, 1));
                                }
                                ui.same_line();
                                if ui.small_button(">") {
                                    operation = Some((i, 2));
                                }
                            });
                        }
                        if count > 0 {
                            ui.same_line();
                        }
                        let _disabled = ui.begin_disabled_with_cond(names.is_empty());
                        let add_frame = ui.button("+ frame");
                        #[cfg(test)]
                        {
                            let min = ui.item_rect_min();
                            let size = ui.item_rect_size();
                            self.frame_add_input = [min[0] + size[0] * 0.5, min[1] + size[1] * 0.5];
                        }
                        if add_frame
                            && let Some(frame) =
                                next_frame(&animation, self.selected_sprite, &sheet.meta.sprites)
                        {
                            animation.frames.push(frame);
                            changed = true;
                        }
                        drop(_disabled);
                        if let Some((i, op)) = operation {
                            match op {
                                0 => changed |= move_frame(&mut animation.frames, i, -1),
                                1 => {
                                    animation.frames.remove(i);
                                    changed = true;
                                }
                                _ => changed |= move_frame(&mut animation.frames, i, 1),
                            }
                        }
                        let playing = self.preview.playing_frame(&animation, tick);
                        #[cfg(test)]
                        {
                            self.preview.highlighted = playing;
                        }
                        if let Some(frame) = playing
                            && let Some(&(min, max)) = rects.get(frame)
                        {
                            ui.get_window_draw_list()
                                .add_rect(min, max, [1.0, 0.75, 0.1, 1.0])
                                .thickness(3.0)
                                .build();
                        }
                    });
                if names.is_empty() {
                    ui.text_disabled("Create a sprite before adding frames");
                }
                if changed {
                    match sheet.meta.put_animation(animation.id, animation, now) {
                        Ok(true) => {
                            sheet.dirty = true;
                            log::append_repeat(&mut self.log, "Animation updated");
                        }
                        Ok(false) => {}
                        Err(error) => log::append(&mut self.log, error),
                    }
                }
                if ui.button("Delete animation") {
                    sheet.meta.animations.remove(index);
                    self.selected_animation = None;
                    self.animation_draft = Default::default();
                    sheet.dirty = true;
                }
            });
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn row_defaults_and_moves() {
        let (_, sheet) = crate::sprite::example();
        let animation = &sheet.animations[0];
        assert_eq!(animation_row(animation), "loop - 4 frames - 600 ms");
        let mut a = animation.clone();
        a.frames.clear();
        assert_eq!(animation_row(&a), "loop - 0 frames - 0 ms");
        assert_eq!(next_frame(&a, None, &sheet.sprites).unwrap().sprite_id, 1);
        assert_eq!(
            next_frame(&a, Some(3), &sheet.sprites).unwrap().sprite_id,
            3
        );
        assert!(next_frame(&a, None, &[]).is_none());
        a.frames = animation.frames.clone();
        a.frames.last_mut().unwrap().ms = 321;
        assert_eq!(next_frame(&a, None, &sheet.sprites).unwrap().ms, 321);
        assert!(!move_frame(&mut a.frames, 0, -1));
        assert!(!move_frame(&mut a.frames, 3, 1));
        assert!(move_frame(&mut a.frames, 0, 1));
        assert_eq!(a.frames[0].sprite_id, 2);
        let mut sheet = sheet;
        let id = a.id;
        assert!(sheet.put_animation(id, a, 77).unwrap());
        assert_eq!(sheet.animations[0].modified, Some(77));
    }
}
