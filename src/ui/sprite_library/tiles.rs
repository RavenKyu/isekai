use super::log;
use super::{
    SheetEditor, format_props, format_tags, model::Selection, parse_props, parse_tags,
    view::input_u32,
};
use crate::sprite::{
    Mode,
    format::{TileAnimation, TileFrame, TileMeta},
};
use dear_imgui_rs::Ui;

#[derive(Default)]
pub struct TileDraft {
    cell: Option<(u32, u32)>,
    name: String,
    tags: String,
    props: String,
    /// While pinned, the edited tile stays put and the selection picks frame cells.
    pinned: bool,
}
impl TileDraft {
    /// The tile being edited: the pinned one, otherwise the single selected cell.
    pub fn target(&self, selection: &Selection) -> Option<(u32, u32)> {
        if self.pinned {
            self.cell
        } else {
            selection.single()
        }
    }
    pub fn validate(&self, meta: &crate::sprite::SpriteSheet) -> Result<(), String> {
        let Some(cell) = self.cell else {
            return Ok(());
        };
        parse_props(&self.props)?;
        if !self.name.is_empty()
            && (self.name.trim().is_empty()
                || meta.tiles.iter().any(|t| {
                    (t.col, t.row) != cell && t.name.as_deref() == Some(self.name.as_str())
                }))
        {
            return Err(format!("empty or duplicate tile name: {:?}", self.name));
        }
        Ok(())
    }
}
/// Frames for the selected cells, row by row and left to right within a row.
pub fn frames_from_selection(selection: &Selection, ms: u32) -> Vec<TileFrame> {
    let mut cells: Vec<_> = selection.cells.iter().copied().collect();
    cells.sort_by_key(|&(col, row)| (row, col));
    cells
        .into_iter()
        .map(|(col, row)| TileFrame { col, row, ms })
        .collect()
}
/// The format keeps only tiles with metadata: a cleared tile leaves the list.
pub fn put_tile(tiles: &mut Vec<TileMeta>, tile: TileMeta) {
    let index = tiles
        .iter()
        .position(|t| (t.col, t.row) == (tile.col, tile.row));
    let empty = tile.name.is_none()
        && tile.tags.is_empty()
        && tile.props.is_empty()
        && tile.animation.is_none();
    match (index, empty) {
        (Some(i), true) => {
            tiles.remove(i);
        }
        (Some(i), false) => tiles[i] = tile,
        (None, false) => tiles.push(tile),
        (None, true) => {}
    }
}
pub fn mode_combo(ui: &Ui, label: &str, mode: &mut Mode) -> bool {
    let mut current = match mode {
        Mode::Loop => 0,
        Mode::Once => 1,
        Mode::PingPong => 2,
    };
    if ui.combo_simple_string(label, &mut current, &["loop", "once", "pingpong"]) {
        *mode = [Mode::Loop, Mode::Once, Mode::PingPong][current];
        true
    } else {
        false
    }
}
impl SheetEditor {
    pub fn tiles_tab(&mut self, ui: &Ui) {
        let sheet = &mut self.sheet;
        let Some((col, row)) = self.tile_draft.target(&sheet.selection) else {
            self.tile_draft = Default::default();
            ui.text(format!("{} tiles selected", sheet.selection.cells.len()));
            return;
        };
        ui.text(format!(
            "Tile ({col}, {row}), empty: {}",
            sheet.image.tile_is_empty(&sheet.meta.grid, col, row)
        ));
        let index = sheet
            .meta
            .tiles
            .iter()
            .position(|t| t.col == col && t.row == row);
        let mut tile = index
            .map(|i| sheet.meta.tiles[i].clone())
            .unwrap_or(TileMeta {
                col,
                row,
                name: None,
                tags: vec![],
                props: Default::default(),
                animation: None,
            });
        if self.tile_draft.cell != Some((col, row)) {
            self.tile_draft = TileDraft {
                cell: Some((col, row)),
                name: tile.name.clone().unwrap_or_default(),
                tags: format_tags(&tile.tags),
                props: format_props(&tile.props),
                pinned: false,
            };
        }
        ui.checkbox("Pin tile", &mut self.tile_draft.pinned);
        if self.tile_draft.pinned {
            ui.same_line();
            ui.text_disabled("(select frame cells on the sheet)");
        }
        let mut changed = ui
            .input_text("Tile name", &mut self.tile_draft.name)
            .build();
        #[cfg(test)]
        {
            let min = ui.item_rect_min();
            let size = ui.item_rect_size();
            self.tile_name_input = [min[0] + 20.0, min[1] + size[1] * 0.5];
        }
        changed |= ui
            .input_text("Tile tags", &mut self.tile_draft.tags)
            .build();
        changed |= ui
            .input_text_multiline(
                "Tile props (key=value)",
                &mut self.tile_draft.props,
                [0.0, 65.0],
            )
            .build();
        if changed {
            tile.name = (!self.tile_draft.name.is_empty()).then(|| self.tile_draft.name.clone());
            tile.tags = parse_tags(&self.tile_draft.tags);
            match parse_props(&self.tile_draft.props) {
                Ok(props) => tile.props = props,
                Err(error) => {
                    log::append(&mut self.log, error);
                    return;
                }
            }
        }
        let mut animated = tile.animation.is_some();
        if ui.checkbox("Tile animation", &mut animated) {
            changed = true;
            tile.animation = animated.then(|| TileAnimation {
                mode: Mode::Loop,
                frames: vec![TileFrame { col, row, ms: 100 }],
            });
        }
        if let Some(animation) = &mut tile.animation {
            changed |= mode_combo(ui, "Tile mode", &mut animation.mode);
            let mut operation = None;
            let frame_count = animation.frames.len();
            for (i, frame) in animation.frames.iter_mut().enumerate() {
                let _id = ui.push_id(i as i32);
                ui.set_next_item_width(110.0);
                changed |= input_u32(ui, "Col", &mut frame.col, 0);
                ui.same_line();
                ui.set_next_item_width(110.0);
                changed |= input_u32(ui, "Row", &mut frame.row, 0);
                ui.same_line();
                ui.set_next_item_width(120.0);
                changed |= input_u32(ui, "ms", &mut frame.ms, 1);
                if ui.small_button("Up") && i > 0 {
                    operation = Some((i, 0));
                }
                ui.same_line();
                if ui.small_button("Down") && i + 1 < frame_count {
                    operation = Some((i, 1));
                }
                ui.same_line();
                if ui.small_button("Remove") {
                    operation = Some((i, 2));
                }
            }
            if let Some((i, op)) = operation {
                match op {
                    0 => animation.frames.swap(i, i - 1),
                    1 => animation.frames.swap(i, i + 1),
                    _ => {
                        animation.frames.remove(i);
                    }
                };
                changed = true;
            }
            let label = if self.tile_draft.pinned {
                "Add selected cells"
            } else {
                "Add this tile"
            };
            let adding = frames_from_selection(&sheet.selection, 100);
            {
                let _disabled = ui.begin_disabled_with_cond(adding.is_empty());
                if ui.button(label) {
                    animation.frames.extend(adding);
                    changed = true;
                }
            }
            if !self.tile_draft.pinned {
                ui.same_line();
                ui.text_disabled("Pin tile to add other cells");
            }
        }
        if changed {
            let mut candidate = sheet.meta.clone();
            put_tile(&mut candidate.tiles, tile);
            match candidate.validate_bounds(sheet.image.width, sheet.image.height) {
                Ok(()) => {
                    sheet.meta = candidate;
                    sheet.dirty = true;
                    log::append_repeat(&mut self.log, "Tile metadata updated");
                }
                Err(errors) => log::append(&mut self.log, errors.join("; ")),
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pinned_tile_keeps_target_and_takes_selected_cells_in_row_order() {
        let mut selection = Selection::default();
        selection.apply(Some((0, 0)), false, false);
        let mut draft = TileDraft {
            cell: Some((0, 0)),
            ..Default::default()
        };
        assert_eq!(draft.target(&selection), Some((0, 0)));
        draft.pinned = true;
        selection.apply(Some((2, 1)), false, false);
        selection.apply(Some((1, 0)), false, true);
        selection.apply(Some((1, 1)), false, true);
        assert_eq!(draft.target(&selection), Some((0, 0)));
        let cells: Vec<_> = frames_from_selection(&selection, 80)
            .iter()
            .map(|f| (f.col, f.row, f.ms))
            .collect();
        assert_eq!(cells, vec![(1, 0, 80), (1, 1, 80), (2, 1, 80)]);
        draft.pinned = false;
        assert_eq!(draft.target(&selection), None);
        selection.apply(Some((2, 1)), false, false);
        assert_eq!(draft.target(&selection), Some((2, 1)));
    }
    #[test]
    fn cleared_tile_metadata_leaves_the_sparse_list() {
        let blank = TileMeta {
            col: 1,
            row: 2,
            name: None,
            tags: vec![],
            props: Default::default(),
            animation: None,
        };
        let mut tiles = Vec::new();
        put_tile(&mut tiles, blank.clone());
        assert!(tiles.is_empty());
        let named = TileMeta {
            name: Some("torch".into()),
            ..blank.clone()
        };
        put_tile(&mut tiles, named.clone());
        assert_eq!(tiles, vec![named]);
        put_tile(&mut tiles, blank);
        assert!(tiles.is_empty());
    }
}
