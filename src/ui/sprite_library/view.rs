use super::{SpriteLibrary, cell_under};
use dear_imgui_rs::{MouseButton, Ui, WindowFlags};

pub struct Overlay {
    pub col: u32,
    pub row: u32,
    pub cols: u32,
    pub rows: u32,
    pub color: [f32; 4],
}
pub fn input_u32(ui: &Ui, label: &str, value: &mut u32, min: u32) -> bool {
    let mut number = (*value).min(i32::MAX as u32) as i32;
    if ui.input_int(label, &mut number) {
        *value = (number.max(min as i32)) as u32;
        true
    } else {
        false
    }
}
impl SpriteLibrary {
    pub fn sheet_view(&mut self, ui: &Ui, overlays: &[Overlay]) {
        let Some(sheet) = &mut self.sheet else {
            return;
        };
        if let Some(texture) = &sheet.texture {
            debug_assert_eq!(texture.width, sheet.image.width);
        }
        ui.child_window("Sheet view")
            .size([0.0, 210.0])
            .flags(WindowFlags::HORIZONTAL_SCROLLBAR)
            .build(ui, || {
                let origin = ui.cursor_screen_pos();
                let zoom = self.zoom as f32;
                ui.image(
                    sheet.imgui_id,
                    [
                        sheet.image.width as f32 * zoom,
                        sheet.image.height as f32 * zoom,
                    ],
                );
                let hovered = ui.is_item_hovered();
                if hovered && ui.io().key_ctrl() && ui.io().mouse_wheel() != 0.0 {
                    self.zoom = (self.zoom as i32 + ui.io().mouse_wheel().signum() as i32)
                        .clamp(1, 16) as u32;
                }
                if hovered && ui.is_mouse_clicked(MouseButton::Left) {
                    let cell = cell_under(
                        &sheet.meta.grid,
                        zoom as u32,
                        origin,
                        ui.io().mouse_pos(),
                        sheet.image.width,
                        sheet.image.height,
                    );
                    sheet
                        .selection
                        .apply(cell, ui.io().key_shift(), ui.io().key_ctrl());
                }
                let grid = &sheet.meta.grid;
                let cells = grid.cells(sheet.image.width, sheet.image.height);
                let draw = ui.get_window_draw_list();
                let visible_top = ui.window_pos();
                let visible_size = ui.window_size();
                let step = (u64::from(grid.tile) + u64::from(grid.spacing)) as f32 * zoom;
                let first_col = ((visible_top[0] - origin[0] - grid.offset[0] as f32 * zoom) / step)
                    .floor()
                    .max(0.0) as u32;
                let first_row = ((visible_top[1] - origin[1] - grid.offset[1] as f32 * zoom) / step)
                    .floor()
                    .max(0.0) as u32;
                let end_col = (((visible_top[0] + visible_size[0] - origin[0]) / step)
                    .ceil()
                    .max(0.0) as u32
                    + 1)
                .min(cells.cols);
                let end_row = (((visible_top[1] + visible_size[1] - origin[1]) / step)
                    .ceil()
                    .max(0.0) as u32
                    + 1)
                .min(cells.rows);
                for row in first_row..end_row {
                    for col in first_col..end_col {
                        let (x, y) = grid.origin(col, row);
                        let min = [origin[0] + x as f32 * zoom, origin[1] + y as f32 * zoom];
                        let max = [
                            min[0] + grid.tile as f32 * zoom,
                            min[1] + grid.tile as f32 * zoom,
                        ];
                        if sheet.image.tile_is_empty(grid, col, row) {
                            draw.add_rect(min, max, [0.0, 0.0, 0.0, 0.35])
                                .filled(true)
                                .build();
                        }
                        if self.show_grid {
                            draw.add_rect(min, max, [0.8, 0.8, 0.8, 0.65]).build();
                        }
                        if sheet.selection.cells.contains(&(col, row)) {
                            draw.add_rect(min, max, [0.3, 0.7, 1.0, 0.45])
                                .filled(true)
                                .build();
                        }
                    }
                }
                for overlay in overlays {
                    for row in overlay.row..overlay.row.saturating_add(overlay.rows).min(cells.rows)
                    {
                        for col in
                            overlay.col..overlay.col.saturating_add(overlay.cols).min(cells.cols)
                        {
                            let (x, y) = grid.origin(col, row);
                            let min = [origin[0] + x as f32 * zoom, origin[1] + y as f32 * zoom];
                            draw.add_rect(
                                min,
                                [
                                    min[0] + grid.tile as f32 * zoom,
                                    min[1] + grid.tile as f32 * zoom,
                                ],
                                overlay.color,
                            )
                            .thickness(2.0)
                            .build();
                        }
                    }
                }
            });
    }
}
