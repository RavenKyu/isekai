use super::{
    SpriteLibrary, edit,
    view::{Overlay, input_u32},
};
use crate::sprite::format::{Part, Shape};
use dear_imgui_rs::Ui;

impl SpriteLibrary {
    pub fn sprite_overlays(&self) -> Vec<Overlay> {
        let Some(sheet) = &self.sheet else {
            return vec![];
        };
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
        let Some(sheet) = &mut self.sheet else {
            return;
        };
        let selection = &sheet.selection.cells;
        if ui.button("New rect from selection") {
            let name =
                edit::next_name("sprite", sheet.meta.sprites.iter().map(|s| s.name.as_str()));
            if let Some(sprite) = edit::rect_from_cells(name, selection, &sheet.meta.grid) {
                self.selected_sprite = Some(sprite.name.clone());
                sheet.meta.sprites.push(sprite);
                sheet.dirty = true;
            } else {
                self.status = "Select tiles first".into();
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
                self.status = "Select tiles first".into();
            }
        }
        if ui.button("Suggest from objects") {
            self.suggestions = edit::suggestions(&sheet.image, &sheet.meta.grid);
            if self.suggestions.is_empty() {
                self.status = "No object suggestions".into();
            }
        }
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
        if ui.input_text("Sprite name", &mut sprite.name).build() {
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
                    self.status = "Sprite renamed and clip references updated".into();
                }
                Err(error) => {
                    self.status = error;
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
                    self.status = "Sprite updated".into();
                }
                Err(errors) => self.status = errors.join("; "),
            }
        }
        if ui.button("Delete sprite") {
            let name = sheet.meta.sprites[index].name.clone();
            match sheet.meta.remove_sprite(&name) {
                Ok(()) => {
                    self.selected_sprite = None;
                    sheet.dirty = true;
                }
                Err(error) => self.status = error,
            }
        }
    }
}
