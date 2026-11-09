use crate::sprite::{
    Grid, Sprite,
    format::{Part, Shape},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CanvasCell {
    pub col: u32,
    pub row: u32,
    pub flip_x: bool,
    pub rotation: u16,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Canvas {
    pub w: u32,
    pub h: u32,
    pub cells: Vec<Option<CanvasCell>>,
    #[cfg(test)]
    pub positions: Vec<[f32; 2]>,
    #[cfg(test)]
    pub flip_input: [f32; 2],
    #[cfg(test)]
    pub rotate_input: [f32; 2],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotEditable {
    Large,
    OffGrid,
}
impl Canvas {
    pub fn empty(w: u32, h: u32) -> Self {
        Self {
            w,
            h,
            cells: vec![None; (w * h) as usize],
            #[cfg(test)]
            positions: vec![[0.0; 2]; (w * h) as usize],
            #[cfg(test)]
            flip_input: [0.0; 2],
            #[cfg(test)]
            rotate_input: [0.0; 2],
        }
    }
    pub fn from_sprite(sprite: &Sprite, grid: &Grid) -> Result<Self, NotEditable> {
        let [w, h] = match &sprite.shape {
            Shape::Rect { cols, rows, .. } => [*cols, *rows],
            Shape::Set { canvas, .. } => *canvas,
        };
        if w > 8 || h > 8 {
            return Err(NotEditable::Large);
        }
        if w == 0 || h == 0 || grid.tile == 0 {
            return Err(NotEditable::OffGrid);
        }
        let mut canvas = Self::empty(w, h);
        for p in sprite.parts(grid) {
            if p.dx < 0
                || p.dy < 0
                || !(p.dx as u32).is_multiple_of(grid.tile)
                || !(p.dy as u32).is_multiple_of(grid.tile)
            {
                return Err(NotEditable::OffGrid);
            }
            let (x, y) = (p.dx as u32 / grid.tile, p.dy as u32 / grid.tile);
            if x >= w || y >= h {
                return Err(NotEditable::OffGrid);
            }
            let slot = &mut canvas.cells[(y * w + x) as usize];
            if slot.is_some() {
                return Err(NotEditable::OffGrid);
            }
            *slot = Some(CanvasCell {
                col: p.col,
                row: p.row,
                flip_x: p.flip_x,
                rotation: p.rotation,
            });
        }
        Ok(canvas)
    }
    pub fn resize(&mut self, w: u32, h: u32) -> usize {
        let mut next = Self::empty(w, h);
        let mut dropped = 0;
        for y in 0..self.h {
            for x in 0..self.w {
                let cell = self.cells[(y * self.w + x) as usize];
                if x < w && y < h {
                    next.cells[(y * w + x) as usize] = cell;
                } else if cell.is_some() {
                    dropped += 1;
                }
            }
        }
        *self = next;
        dropped
    }
    pub fn commit(&self, sprite: &Sprite, grid: &Grid) -> Sprite {
        let mut result = sprite.clone();
        let mut parts = Vec::new();
        for y in 0..self.h {
            for x in 0..self.w {
                if let Some(c) = self.cells[(y * self.w + x) as usize] {
                    parts.push(Part {
                        col: c.col,
                        row: c.row,
                        dx: (x * grid.tile) as i32,
                        dy: (y * grid.tile) as i32,
                        flip_x: c.flip_x,
                        rotation: c.rotation,
                    });
                }
            }
        }
        result.shape = Shape::Set {
            canvas: [self.w, self.h],
            parts,
        };
        result
    }
    pub fn put(&mut self, x: u32, y: u32, cell: Option<CanvasCell>) {
        self.cells[(y * self.w + x) as usize] = cell;
    }
}
pub fn draw(
    ui: &dear_imgui_rs::Ui,
    canvas: &mut Canvas,
    sprite: &Sprite,
    grid: &Grid,
    view: &super::preview::PartView,
    sheet_id: super::SheetId,
    log: &mut super::log::SheetLog,
) -> bool {
    use dear_imgui_rs::{DragDropTargetFlags, MouseButton, StyleVar};
    let mut changed = false;
    let options = ["1", "2", "3", "4", "5", "6", "7", "8"];
    let (mut wi, mut hi) = ((canvas.w - 1) as usize, (canvas.h - 1) as usize);
    ui.set_next_item_width(70.0);
    let mut resize = ui.combo_simple_string("W", &mut wi, &options);
    ui.same_line();
    ui.set_next_item_width(70.0);
    resize |= ui.combo_simple_string("H", &mut hi, &options);
    if resize {
        let dropped = canvas.resize(wi as u32 + 1, hi as u32 + 1);
        if dropped > 0 {
            super::log::append(log, format!("{dropped} cells dropped by resize"));
        }
        changed = true;
    }
    let origin = ui.cursor_screen_pos();
    let cell_size = 52.0;
    {
        let _spacing = ui.push_style_var(StyleVar::ItemSpacing([0.0, 0.0]));
        for y in 0..canvas.h {
            for x in 0..canvas.w {
                if x > 0 {
                    ui.same_line();
                }
                let _id = ui.push_id((y * canvas.w + x) as i32);
                ui.invisible_button("canvas-cell", [cell_size, cell_size]);
                let min = ui.item_rect_min();
                #[cfg(test)]
                {
                    canvas.positions[(y * canvas.w + x) as usize] =
                        [min[0] + cell_size * 0.5, min[1] + cell_size * 0.5];
                }
                let hovered = ui.is_item_hovered();
                let draw = ui.get_window_draw_list();
                let color = if (x + y) % 2 == 0 {
                    [0.20, 0.20, 0.20, 1.0]
                } else {
                    [0.27, 0.27, 0.27, 1.0]
                };
                draw.add_rect(min, [min[0] + cell_size, min[1] + cell_size], color)
                    .filled(true)
                    .build();
                drop(draw);
                if let Some(cell) = canvas.cells[(y * canvas.w + x) as usize] {
                    let part = Part {
                        col: cell.col,
                        row: cell.row,
                        dx: 0,
                        dy: 0,
                        flip_x: cell.flip_x,
                        rotation: cell.rotation,
                    };
                    super::preview::draw_parts_fit(
                        ui,
                        view,
                        &[part],
                        (grid.tile, grid.tile),
                        min,
                        [cell_size, cell_size],
                    );
                    if hovered {
                        ui.get_window_draw_list().add_text(
                            [min[0] + cell_size - 13.0, min[1] + 2.0],
                            [1.0; 4],
                            "x",
                        );
                        let mouse = ui.io().mouse_pos();
                        if ui.is_mouse_clicked(MouseButton::Left)
                            && mouse[0] >= min[0] + cell_size - 16.0
                            && mouse[1] < min[1] + 16.0
                        {
                            canvas.put(x, y, None);
                            changed = true;
                        }
                    }
                }
                ui.get_window_draw_list()
                    .add_rect(
                        min,
                        [min[0] + cell_size, min[1] + cell_size],
                        [0.65, 0.65, 0.65, 0.8],
                    )
                    .build();
                if let Some(target) = ui.drag_drop_target()
                    && let Some(Ok(payload)) = target.accept_payload::<super::TileDrag, _>(
                        super::TILE_DRAG,
                        DragDropTargetFlags::empty(),
                    )
                    && payload.delivery
                {
                    if payload.data.sheet != sheet_id {
                        super::log::append(log, "Drop tiles from this sheet only".into());
                    } else {
                        canvas.put(
                            x,
                            y,
                            Some(CanvasCell {
                                col: payload.data.col,
                                row: payload.data.row,
                                flip_x: false,
                                rotation: 0,
                            }),
                        );
                        changed = true;
                    }
                }
                if let Some(_popup) = ui.begin_popup_context_item() {
                    let index = (y * canvas.w + x) as usize;
                    if let Some(mut cell) = canvas.cells[index] {
                        let flipped =
                            ui.menu_item_enabled_selected_no_shortcut("Flip X", cell.flip_x, true);
                        #[cfg(test)]
                        {
                            let min = ui.item_rect_min();
                            let size = ui.item_rect_size();
                            canvas.flip_input = [min[0] + size[0] * 0.5, min[1] + size[1] * 0.5];
                        }
                        if flipped {
                            cell.flip_x = !cell.flip_x;
                            canvas.cells[index] = Some(cell);
                            changed = true;
                        }
                        for angle in [0, 90, 180, 270] {
                            let rotated = ui.menu_item_enabled_selected_no_shortcut(
                                format!("Rotation {angle}"),
                                cell.rotation == angle,
                                true,
                            );
                            #[cfg(test)]
                            {
                                if angle == 90 {
                                    let min = ui.item_rect_min();
                                    let size = ui.item_rect_size();
                                    canvas.rotate_input =
                                        [min[0] + size[0] * 0.5, min[1] + size[1] * 0.5];
                                }
                            }
                            if rotated {
                                cell.rotation = angle;
                                canvas.cells[index] = Some(cell);
                                changed = true;
                            }
                        }
                    }
                    if ui.menu_item_enabled_selected_no_shortcut(
                        "Clear",
                        false,
                        canvas.cells[index].is_some(),
                    ) {
                        canvas.cells[index] = None;
                        changed = true;
                    }
                }
            }
        }
    }
    let scale = cell_size / grid.tile as f32;
    let cross = [
        origin[0] + sprite.pivot[0] * scale,
        origin[1] + sprite.pivot[1] * scale,
    ];
    let draw = ui.get_window_draw_list();
    draw.add_line(
        [cross[0] - 5.0, cross[1]],
        [cross[0] + 5.0, cross[1]],
        [1.0, 0.25, 0.2, 1.0],
    )
    .build();
    draw.add_line(
        [cross[0], cross[1] - 5.0],
        [cross[0], cross[1] + 5.0],
        [1.0, 0.25, 0.2, 1.0],
    )
    .build();
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rect_conversion_preserves_parts_and_identity() {
        let (_, sheet) = crate::sprite::example();
        let mut s = sheet.sprites[0].clone();
        s.shape = Shape::Rect {
            col: 0,
            row: 0,
            cols: 2,
            rows: 2,
        };
        let canvas = Canvas::from_sprite(&s, &sheet.grid).unwrap();
        let result = canvas.commit(&s, &sheet.grid);
        assert_eq!(result.id, s.id);
        assert_eq!(result.parts(&sheet.grid), s.parts(&sheet.grid));
        assert!(matches!(result.shape, Shape::Set { canvas: [2, 2], .. }));
        assert!(matches!(s.shape, Shape::Rect { .. }));
    }
    #[test]
    fn empty_edges_resize_order_and_transforms_survive() {
        let (_, mut sheet) = crate::sprite::example();
        let original = sheet.sprites[0].clone();
        let mut canvas = Canvas::empty(3, 2);
        canvas.put(
            2,
            0,
            Some(CanvasCell {
                col: 2,
                row: 0,
                flip_x: true,
                rotation: 90,
            }),
        );
        canvas.put(
            1,
            0,
            Some(CanvasCell {
                col: 1,
                row: 0,
                flip_x: false,
                rotation: 0,
            }),
        );
        let sprite = canvas.commit(&original, &sheet.grid);
        let Shape::Set { canvas: dim, parts } = &sprite.shape else {
            panic!()
        };
        assert_eq!(*dim, [3, 2]);
        assert_eq!(parts.iter().map(|p| p.col).collect::<Vec<_>>(), vec![1, 2]);
        assert_eq!(parts[1].rotation, 90);
        assert!(parts[1].flip_x);
        assert_eq!(sprite.pivot, original.pivot);
        sheet.sprites[0] = sprite;
        let json = crate::sprite::store::to_json(&sheet);
        let read = crate::sprite::store::parse(&json).unwrap();
        assert_eq!(
            Canvas::from_sprite(&read.sprites[0], &read.grid).unwrap(),
            canvas
        );
        assert_eq!(canvas.resize(2, 1), 1);
        assert!(canvas.cells[1].is_some());
        let shrunk = canvas.commit(&original, &sheet.grid);
        assert_eq!(shrunk.pivot, original.pivot);
        assert!(matches!(shrunk.shape, Shape::Set { canvas: [2, 1], .. }));
        let empty = Canvas::empty(2, 2).commit(&original, &sheet.grid);
        sheet.sprites[0] = empty;
        assert!(sheet.validate().is_err());
    }
    #[test]
    fn commit_is_row_major_even_when_cells_are_set_out_of_order() {
        let (_, sheet) = crate::sprite::example();
        let mut canvas = Canvas::empty(2, 2);
        canvas.put(
            0,
            1,
            Some(CanvasCell {
                col: 3,
                row: 0,
                flip_x: false,
                rotation: 0,
            }),
        );
        canvas.put(
            1,
            0,
            Some(CanvasCell {
                col: 1,
                row: 0,
                flip_x: false,
                rotation: 0,
            }),
        );
        let sprite = canvas.commit(&sheet.sprites[0], &sheet.grid);
        assert_eq!(
            sprite
                .parts(&sheet.grid)
                .iter()
                .map(|p| p.col)
                .collect::<Vec<_>>(),
            vec![1, 3]
        );
    }
    #[test]
    fn rejects_large_off_grid_and_overlapping_parts() {
        let (_, sheet) = crate::sprite::example();
        let mut s = sheet.sprites[0].clone();
        let p = Part {
            dx: 1,
            ..Default::default()
        };
        s.shape = Shape::Set {
            canvas: [2, 2],
            parts: vec![p],
        };
        assert_eq!(
            Canvas::from_sprite(&s, &sheet.grid),
            Err(NotEditable::OffGrid)
        );
        s.shape = Shape::Set {
            canvas: [2, 2],
            parts: vec![Part::default(), Part::default()],
        };
        assert_eq!(
            Canvas::from_sprite(&s, &sheet.grid),
            Err(NotEditable::OffGrid)
        );
        s.shape = Shape::Set {
            canvas: [9, 1],
            parts: vec![Part::default()],
        };
        assert_eq!(
            Canvas::from_sprite(&s, &sheet.grid),
            Err(NotEditable::Large)
        );
    }
}
