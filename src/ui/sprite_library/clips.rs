use super::{
    SheetEditor, edit, format_props, format_tags, parse_props, parse_tags, tiles::mode_combo,
    view::input_u32,
};
use crate::sprite::{Clip, Mode, format::ClipFrame};
use dear_imgui_rs::Ui;
#[derive(Default)]
pub struct ClipDraft {
    name: Option<String>,
    tags: String,
    props: String,
}
impl ClipDraft {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_some() {
            parse_props(&self.props)?;
        }
        Ok(())
    }
}
impl SheetEditor {
    pub fn clips_tab(&mut self, ui: &Ui) {
        let sheet = &mut self.sheet;
        if ui.button("New clip") {
            let name = edit::next_name("clip", sheet.meta.clips.iter().map(|c| c.name.as_str()));
            self.selected_clip = Some(name.clone());
            sheet.meta.clips.push(Clip {
                name,
                mode: Mode::Loop,
                tags: vec![],
                props: Default::default(),
                frames: vec![],
            });
            sheet.dirty = true;
            self.status = "New clip needs at least one frame before Save".into();
        }
        for clip in &sheet.meta.clips {
            if ui
                .selectable_config(format!("{} ({} frames)", clip.name, clip.frames.len()))
                .selected(self.selected_clip.as_ref() == Some(&clip.name))
                .build()
            {
                self.selected_clip = Some(clip.name.clone());
            }
        }
        let Some(index) = sheet
            .meta
            .clips
            .iter()
            .position(|c| Some(&c.name) == self.selected_clip.as_ref())
        else {
            return;
        };
        let mut clip = sheet.meta.clips[index].clone();
        if self.clip_draft.name.as_ref() != Some(&clip.name) {
            self.clip_draft = ClipDraft {
                name: Some(clip.name.clone()),
                tags: format_tags(&clip.tags),
                props: format_props(&clip.props),
            };
        }
        let mut name = clip.name.clone();
        let mut changed = false;
        if ui.input_text("Clip name", &mut name).build() {
            if name.trim().is_empty()
                || sheet
                    .meta
                    .clips
                    .iter()
                    .enumerate()
                    .any(|(i, c)| i != index && c.name == name)
            {
                self.status = format!("empty or duplicate clip name: {name:?}");
            } else {
                if self.preview.target == super::preview::PreviewTarget::Clip(clip.name.clone()) {
                    self.preview.target = super::preview::PreviewTarget::Clip(name.clone());
                }
                clip.name = name.clone();
                self.selected_clip = Some(name.clone());
                self.clip_draft.name = Some(name);
                changed = true;
            }
        }
        changed |= mode_combo(ui, "Clip mode", &mut clip.mode);
        let mut text_changed = ui
            .input_text("Clip tags", &mut self.clip_draft.tags)
            .build();
        text_changed |= ui
            .input_text_multiline(
                "Clip props (key=value)",
                &mut self.clip_draft.props,
                [0.0, 65.0],
            )
            .build();
        if text_changed {
            clip.tags = parse_tags(&self.clip_draft.tags);
            match parse_props(&self.clip_draft.props) {
                Ok(props) => {
                    clip.props = props;
                    changed = true;
                }
                Err(error) => self.status = error,
            }
        }
        let names: Vec<_> = sheet.meta.sprites.iter().map(|s| s.name.as_str()).collect();
        let frame_count = clip.frames.len();
        let mut operation = None;
        for (i, frame) in clip.frames.iter_mut().enumerate() {
            let _id = ui.push_id(&format!("clip-frame-{i}"));
            let mut sprite = names.iter().position(|&s| s == frame.sprite).unwrap_or(0);
            if !names.is_empty() && ui.combo_simple_string("Frame sprite", &mut sprite, &names) {
                frame.sprite = names[sprite].into();
                changed = true;
            }
            ui.set_next_item_width(160.0);
            changed |= input_u32(ui, "Frame ms", &mut frame.ms, 1);
            if ui.small_button("Up") && i > 0 {
                operation = Some((i, 0));
            }
            ui.same_line();
            if ui.small_button("Down") && i + 1 < frame_count {
                operation = Some((i, 1));
            }
            ui.same_line();
            if ui.small_button("Remove frame") {
                operation = Some((i, 2));
            }
        }
        if let Some((i, op)) = operation {
            match op {
                0 => clip.frames.swap(i, i - 1),
                1 => clip.frames.swap(i, i + 1),
                _ => {
                    clip.frames.remove(i);
                }
            };
            changed = true;
        }
        let mut selected = names
            .iter()
            .position(|&s| Some(s) == self.selected_sprite.as_deref())
            .unwrap_or(0);
        if !names.is_empty() {
            if ui.combo_simple_string("Add sprite", &mut selected, &names) {
                self.selected_sprite = Some(names[selected].into());
            }
            if ui.button("Add frame") {
                clip.frames.push(ClipFrame {
                    sprite: names[selected].into(),
                    ms: 100,
                });
                changed = true;
            }
        } else {
            ui.text("Create a sprite before adding frames");
        }
        if changed {
            sheet.meta.clips[index] = clip;
            sheet.dirty = true;
            self.status = "Clip updated".into();
        }
        if ui.button("Delete clip") {
            sheet.meta.clips.remove(index);
            self.selected_clip = None;
            self.clip_draft = Default::default();
            sheet.dirty = true;
        }
    }
}
