use super::log;
use super::{
    SheetEditor,
    canvas::{self, Canvas, NotEditable},
    edit,
    preview::{self, PartView},
    props::PropsDraft,
    view::{Overlay, input_u32},
};
use crate::sprite::format::{Part, Shape};
use dear_imgui_rs::{ListClipper, TreeNodeFlags, Ui};

impl SheetEditor {
    pub fn sprite_overlays(&self) -> Vec<Overlay> {
        let sheet = &self.sheet;
        let Some(sprite) = sheet
            .meta
            .sprites
            .iter()
            .find(|s| Some(s.id) == self.selected_sprite)
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
        let now = (self.now)();
        let sheet = &mut self.sheet;
        {
            let _disabled = ui.begin_disabled_with_cond(sheet.selection.cells.is_empty());
            let create = ui.button("New from selection");
            #[cfg(test)]
            {
                self.sprite_new_input = super::tests::item_center(ui);
            }
            if create {
                let name =
                    edit::next_name("sprite", sheet.meta.sprites.iter().map(|s| s.name.as_str()));
                if let Some(sprite) =
                    edit::set_from_cells(name, &sheet.selection.cells, &sheet.meta.grid)
                {
                    match sheet.meta.add_sprite(sprite, now) {
                        Ok(id) => {
                            self.selected_sprite = Some(id);
                            sheet.dirty = true;
                        }
                        Err(error) => log::append(&mut self.log, error),
                    }
                } else {
                    log::append(
                        &mut self.log,
                        "Select at most 8 x 8 tiles for a sprite".into(),
                    );
                }
            }
        }
        ui.same_line();
        let new_blank = ui.button("+ New sprite");
        #[cfg(test)]
        {
            self.sprite_blank_input = super::tests::item_center(ui);
        }
        if new_blank {
            let name =
                edit::next_name("sprite", sheet.meta.sprites.iter().map(|s| s.name.as_str()));
            let sprite = crate::sprite::Sprite {
                id: 0,
                name,
                shape: Shape::Set {
                    canvas: [2, 2],
                    parts: vec![],
                },
                pivot: edit::bottom_centre_pivot(2, 2, sheet.meta.grid.tile),
                props: Default::default(),
                modified: None,
            };
            match sheet.meta.add_sprite(sprite, now) {
                Ok(id) => {
                    self.selected_sprite = Some(id);
                    sheet.dirty = true;
                    log::append(
                        &mut self.log,
                        "New sprite needs at least one tile before Save".into(),
                    );
                }
                Err(error) => log::append(&mut self.log, error),
            }
        }
        ui.child_window("Sprite list")
            .size([220.0, 0.0])
            .build(ui, || {
                let view = PartView {
                    id: sheet.imgui_id,
                    grid: sheet.meta.grid,
                    width: sheet.image.width,
                    height: sheet.image.height,
                };
                for index in ListClipper::new(sheet.meta.sprites.len())
                    .items_height(54.0)
                    .begin(ui)
                    .iter()
                {
                    let sprite = &sheet.meta.sprites[index];
                    let row = ui.cursor_screen_pos();
                    let _id = ui.push_id(index as i32);
                    if ui
                        .selectable_config("##sprite-row")
                        .selected(self.selected_sprite == Some(sprite.id))
                        .size([0.0, 50.0])
                        .build()
                    {
                        self.selected_sprite = Some(sprite.id);
                    }
                    preview::draw_parts_fit(
                        ui,
                        &view,
                        &sprite.parts(&sheet.meta.grid),
                        sprite.bounds(&sheet.meta.grid),
                        [row[0] + 2.0, row[1] + 5.0],
                        [32.0, 32.0],
                    );
                    let (w, h) = sprite_cell_size(sprite);
                    let draw = ui.get_window_draw_list();
                    draw.add_text([row[0] + 40.0, row[1] + 2.0], [1.0; 4], &sprite.name);
                    let suffix = if sprite.props.is_empty() {
                        String::new()
                    } else {
                        format!("  {} props", sprite.props.len())
                    };
                    draw.add_text(
                        [row[0] + 40.0, row[1] + 22.0],
                        [0.75, 0.75, 0.75, 1.0],
                        format!(
                            "{w} x {h}  {}{suffix}",
                            super::log::format_local(sprite.modified)
                        ),
                    );
                }
            });
        ui.same_line();
        ui.child_window("Sprite details")
            .size([0.0, 0.0])
            .build(ui, || {
                let Some(index) = sheet
                    .meta
                    .sprites
                    .iter()
                    .position(|s| Some(s.id) == self.selected_sprite)
                else {
                    return;
                };
                let mut sprite = sheet.meta.sprites[index].clone();
                ui.text(format!("ID {}", sprite.id));
                if self.sprite_props_target != Some(sprite.id) {
                    self.sprite_props_target = Some(sprite.id);
                    self.sprite_props = PropsDraft::from_map(&sprite.props);
                }

                let old_name = sprite.name.clone();
                let renamed = ui.input_text("Sprite name", &mut sprite.name).build();
                #[cfg(test)]
                {
                    let min = ui.item_rect_min();
                    let size = ui.item_rect_size();
                    self.sprite_name_input = [min[0] + 20.0, min[1] + size[1] * 0.5];
                }
                if renamed {
                    match sheet.meta.rename_sprite(
                        sprite.id,
                        &sprite.name,
                        now,
                        (sheet.image.width, sheet.image.height),
                    ) {
                        Ok(true) => {
                            sheet.dirty = true;
                            log::append_repeat(&mut self.log, "Sprite renamed");
                        }
                        Ok(false) => {}
                        Err(error) => {
                            log::append(&mut self.log, error);
                            sprite.name = old_name.clone();
                        }
                    }
                }
                let mut changed = ui.input_float2("Pivot (px)", &mut sprite.pivot).build();
                if let Some(props) = self.sprite_props.draw(ui, "sprite-properties") {
                    sprite.props = props;
                    changed = true;
                }
                let cache_matches =
                    self.canvas_cache
                        .as_ref()
                        .is_some_and(|(id, shape, grid, _)| {
                            *id == sprite.id && *shape == sprite.shape && *grid == sheet.meta.grid
                        });
                // A sprite the canvas cannot edit never caches, so keep why for the message.
                let mut not_editable = None;
                if !cache_matches {
                    self.canvas_cache = match Canvas::from_sprite(&sprite, &sheet.meta.grid) {
                        Ok(canvas) => {
                            Some((sprite.id, sprite.shape.clone(), sheet.meta.grid, canvas))
                        }
                        Err(reason) => {
                            not_editable = Some(reason);
                            None
                        }
                    };
                }
                if let Some((_, _, _, canvas)) = &mut self.canvas_cache {
                    let view = PartView {
                        id: sheet.imgui_id,
                        grid: sheet.meta.grid,
                        width: sheet.image.width,
                        height: sheet.image.height,
                    };
                    if canvas::draw(
                        ui,
                        canvas,
                        &sprite,
                        &sheet.meta.grid,
                        &view,
                        self.id,
                        &mut self.log,
                    ) {
                        sprite = canvas.commit(&sprite, &sheet.meta.grid);
                        changed = true;
                    }
                } else {
                    let message = match not_editable {
                        Some(NotEditable::Large) => {
                            "This sprite is larger than 8 x 8 tiles; edit it under Parts."
                        }
                        _ => "This sprite has parts off the tile grid; edit them under Parts.",
                    };
                    ui.text_wrapped(message);
                    let view = PartView {
                        id: sheet.imgui_id,
                        grid: sheet.meta.grid,
                        width: sheet.image.width,
                        height: sheet.image.height,
                    };
                    let origin = ui.cursor_screen_pos();
                    preview::draw_parts_fit(
                        ui,
                        &view,
                        &sprite.parts(&sheet.meta.grid),
                        sprite.bounds(&sheet.meta.grid),
                        origin,
                        [160.0, 160.0],
                    );
                    ui.dummy([160.0, 160.0]);
                }
                if ui.collapsing_header("Parts", TreeNodeFlags::empty()) {
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
                        Shape::Set { parts, .. } => {
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
                                if ui.combo_simple_string(
                                    "Rotation",
                                    &mut rotation,
                                    &["0", "90", "180", "270"],
                                ) {
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
                }
                if changed {
                    match sheet.meta.put_sprite(
                        sprite.id,
                        sprite,
                        now,
                        (sheet.image.width, sheet.image.height),
                    ) {
                        Ok(true) => {
                            sheet.dirty = true;
                            log::append_repeat(&mut self.log, "Sprite updated");
                        }
                        Ok(false) => {}
                        Err(error) => {
                            self.canvas_cache = None;
                            log::append(&mut self.log, error);
                        }
                    }
                }
                if ui.button("Delete sprite") {
                    let id = sheet.meta.sprites[index].id;
                    match sheet.meta.remove_sprite(id) {
                        Ok(()) => {
                            self.selected_sprite = None;
                            self.sprite_props_target = None;
                            sheet.dirty = true;
                        }
                        Err(error) => log::append(&mut self.log, error),
                    }
                }
            });
    }
}

fn sprite_cell_size(sprite: &crate::sprite::Sprite) -> (u32, u32) {
    match &sprite.shape {
        Shape::Rect { cols, rows, .. } => (*cols, *rows),
        Shape::Set { canvas, .. } => (canvas[0], canvas[1]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn size_uses_canvas_and_rect_dimensions() {
        let (_, mut sheet) = crate::sprite::example();
        sheet.sprites[0].shape = Shape::Set {
            canvas: [3, 2],
            parts: vec![Part::default()],
        };
        assert_eq!(sprite_cell_size(&sheet.sprites[0]), (3, 2));
        sheet.sprites[0].shape = Shape::Rect {
            col: 0,
            row: 0,
            cols: 3,
            rows: 2,
        };
        assert_eq!(sprite_cell_size(&sheet.sprites[0]), (3, 2));
        assert_eq!(super::super::log::format_local(None), "-");
        let stamp = super::super::log::format_local(Some(0));
        assert_eq!(stamp.len(), 16);
        assert_eq!(stamp.as_bytes()[4], b'-');
        assert_eq!(stamp.as_bytes()[13], b':');
    }
}
