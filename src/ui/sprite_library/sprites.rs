use super::log;
use super::{
    SheetEditor, edit,
    view::{Overlay, input_u32},
};
use crate::sprite::format::{Part, Shape};
use dear_imgui_rs::Ui;

impl SheetEditor {
    pub fn sprite_overlays(&self) -> Vec<Overlay> {
        let sheet = &self.sheet;
        let Some(sprite) = sheet
            .meta
            .sprites
            .iter()
            .find(|s| Some(&s.name) == self.selected_sprite.as_ref())
        else {
            return vec![];
        };
        sprite
            .parts(&sheet.meta.grid)
            .iter()
            .map(|p| Overlay {
                col: p.col,
                row: p.row,
                cols: 1,
                rows: 1,
                color: [1.0, 0.7, 0.15, 1.0],
            })
            .collect()
    }
    pub fn sprites_tab(&mut self, ui: &Ui) {
        let sheet = &mut self.sheet;
        let selection = &sheet.selection.cells;
        if ui.button("New rect from selection") {
            let name =
                edit::next_name("sprite", sheet.meta.sprites.iter().map(|s| s.name.as_str()));
            if let Some(sprite) = edit::rect_from_cells(name, selection, &sheet.meta.grid) {
                self.selected_sprite = Some(sprite.name.clone());
                sheet.meta.sprites.push(sprite);
                sheet.dirty = true;
            } else {
                log::append(&mut self.log, "Select tiles first".into());
            }
        }
        ui.same_line();
        if ui.button("New set from selection") {
            let name =
                edit::next_name("sprite", sheet.meta.sprites.iter().map(|s| s.name.as_str()));
            if let Some(sprite) = edit::set_from_cells(name, selection, &sheet.meta.grid) {
                self.selected_sprite = Some(sprite.name.clone());
                sheet.meta.sprites.push(sprite);
                sheet.dirty = true;
            } else {
                log::append(&mut self.log, "Select tiles first".into());
            }
        }
        if let Some(job) = &self.suggest_job {
            let fraction = job.fraction();
            ui.progress_bar(fraction)
                .size([240.0, 0.0])
                .overlay_text(format!("Suggesting... {:.0}%", fraction * 100.0))
                .build();
            ui.same_line();
            if ui.button("Cancel Suggest") {
                self.cancel_suggest();
            }
        } else if ui.button("Suggest from objects") {
            self.start_suggest();
        }
        let sheet = &mut self.sheet;
        for (i, rect) in self.suggestions.iter().enumerate() {
            let _id = ui.push_id(&format!("object-{i}"));
            ui.text(format!(
                "({}, {}) {} x {} cells",
                rect.col, rect.row, rect.cols, rect.rows
            ));
            ui.same_line();
            if ui.small_button("Add") {
                let cells = [
                    (rect.col, rect.row),
                    (rect.col + rect.cols - 1, rect.row + rect.rows - 1),
                ]
                .into_iter()
                .collect();
                let name =
                    edit::next_name("object", sheet.meta.sprites.iter().map(|s| s.name.as_str()));
                if let Some(sprite) = edit::rect_from_cells(name, &cells, &sheet.meta.grid) {
                    self.selected_sprite = Some(sprite.name.clone());
                    sheet.meta.sprites.push(sprite);
                    sheet.dirty = true;
                }
            }
        }
        ui.separator();
        for sprite in &sheet.meta.sprites {
            let (w, h) = sprite.bounds(&sheet.meta.grid);
            let kind = match sprite.shape {
                Shape::Rect { .. } => "rect",
                Shape::Set { .. } => "set",
            };
            if ui
                .selectable_config(format!("{} ({kind}, {w}x{h} px)", sprite.name))
                .selected(self.selected_sprite.as_ref() == Some(&sprite.name))
                .build()
            {
                self.selected_sprite = Some(sprite.name.clone());
            }
        }
        let Some(index) = sheet
            .meta
            .sprites
            .iter()
            .position(|s| Some(&s.name) == self.selected_sprite.as_ref())
        else {
            return;
        };
        let mut sprite = sheet.meta.sprites[index].clone();
        let old_name = sprite.name.clone();
        let renamed = ui.input_text("Sprite name", &mut sprite.name).build();
        #[cfg(test)]
        {
            let min = ui.item_rect_min();
            let size = ui.item_rect_size();
            self.sprite_name_input = [min[0] + 20.0, min[1] + size[1] * 0.5];
        }
        if renamed {
            match sheet.meta.rename_sprite(&old_name, &sprite.name) {
                Ok(()) => {
                    self.selected_sprite = Some(sprite.name.clone());
                    if self.preview.target
                        == super::preview::PreviewTarget::Sprite(old_name.clone())
                    {
                        self.preview.target =
                            super::preview::PreviewTarget::Sprite(sprite.name.clone());
                    }
                    sheet.dirty = true;
                    // Fires on every keystroke of the name field, like "Sprite updated".
                    log::append_repeat(&mut self.log, "Sprite renamed and clip references updated");
                }
                Err(error) => {
                    log::append(&mut self.log, error);
                    sprite.name = old_name.clone();
                }
            }
        }
        let mut changed = ui.input_float2("Pivot (px)", &mut sprite.pivot).build();
        match &mut sprite.shape {
            Shape::Rect {
                col,
                row,
                cols,
                rows,
            } => {
                changed |= input_u32(ui, "Sprite col", col, 0);
                changed |= input_u32(ui, "Sprite row", row, 0);
                changed |= input_u32(ui, "Sprite cols", cols, 1);
                changed |= input_u32(ui, "Sprite rows", rows, 1);
            }
            Shape::Set { parts } => {
                let count = parts.len();
                let mut remove = None;
                for (i, part) in parts.iter_mut().enumerate() {
                    let _id = ui.push_id(&format!("part-{i}"));
                    ui.text(format!("Part {}", i + 1));
                    ui.set_next_item_width(120.0);
                    changed |= input_u32(ui, "Col", &mut part.col, 0);
                    ui.same_line();
                    ui.set_next_item_width(120.0);
                    changed |= input_u32(ui, "Row", &mut part.row, 0);
                    ui.set_next_item_width(140.0);
                    changed |= ui.input_int("dx", &mut part.dx);
                    ui.same_line();
                    ui.set_next_item_width(140.0);
                    changed |= ui.input_int("dy", &mut part.dy);
                    changed |= ui.checkbox("Flip X", &mut part.flip_x);
                    let mut rotation = usize::from(part.rotation / 90);
                    if ui.combo_simple_string("Rotation", &mut rotation, &["0", "90", "180", "270"])
                    {
                        part.rotation = rotation as u16 * 90;
                        changed = true;
                    }
                    if count > 1 && ui.small_button("Remove part") {
                        remove = Some(i);
                    }
                }
                if let Some(i) = remove {
                    parts.remove(i);
                    changed = true;
                }
                if ui.button("Add selected cell")
                    && let Some((col, row)) = sheet.selection.single()
                {
                    parts.push(Part {
                        col,
                        row,
                        dx: 0,
                        dy: 0,
                        ..Default::default()
                    });
                    changed = true;
                }
            }
        }
        if changed {
            let mut candidate = sheet.meta.clone();
            candidate.sprites[index] = sprite.clone();
            candidate.clips.clear();
            match candidate.validate_bounds(sheet.image.width, sheet.image.height) {
                Ok(()) => {
                    sheet.meta.sprites[index] = sprite;
                    sheet.dirty = true;
                    log::append_repeat(&mut self.log, "Sprite updated");
                }
                Err(errors) => log::append(&mut self.log, errors.join("; ")),
            }
        }
        if ui.button("Delete sprite") {
            let name = sheet.meta.sprites[index].name.clone();
            match sheet.meta.remove_sprite(&name) {
                Ok(()) => {
                    self.selected_sprite = None;
                    sheet.dirty = true;
                }
                Err(error) => log::append(&mut self.log, error),
            }
        }
    }
}
