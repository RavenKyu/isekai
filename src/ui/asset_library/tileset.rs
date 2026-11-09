use super::log;
use super::{SheetEditor, format_tags, model::Selection, parse_tags, props::PropsDraft};
use crate::sprite::{Mode, format::TileMeta};
use dear_imgui_rs::Ui;

#[derive(Default)]
pub struct TileDraft {
    pub(super) cell: Option<(u32, u32)>,
    name: String,
    tags: String,
    pub(super) props: PropsDraft,
}
impl TileDraft {
    pub fn target(&self, selection: &Selection) -> Option<(u32, u32)> {
        selection.single()
    }
    pub fn validate(&self, meta: &crate::sprite::SpriteSheet) -> Result<(), String> {
        let Some(cell) = self.cell else {
            return Ok(());
        };
        self.props.validate()?;
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
/// The format keeps only tiles with metadata: a cleared tile leaves the list.
pub fn put_tile(tiles: &mut Vec<TileMeta>, tile: TileMeta) {
    let index = tiles
        .iter()
        .position(|t| (t.col, t.row) == (tile.col, tile.row));
    let empty = tile.name.is_none() && tile.tags.is_empty() && tile.props.is_empty();
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
    pub fn tileset_tab(&mut self, ui: &Ui) {
        let sheet = &mut self.sheet;
        let Some((col, row)) = self.tile_draft.target(&sheet.selection) else {
            self.tile_draft = Default::default();
            if sheet.selection.cells.is_empty() {
                ui.text("Select a tile on the sheet.");
            } else {
                ui.text_wrapped(format!(
                    "{} tiles selected. Tile metadata is edited one tile at a time.",
                    sheet.selection.cells.len()
                ));
                ui.text_wrapped("Use Sprites > New from selection for several cells.");
            }
            return;
        };
        let (uv0, uv1) = sheet
            .meta
            .grid
            .uv(col, row, sheet.image.width, sheet.image.height);
        ui.image_config(sheet.imgui_id, [48.0, 48.0])
            .uv0(uv0)
            .uv1(uv1)
            .build();
        ui.same_line();
        ui.text(format!(
            "Tile ({col}, {row}){}",
            if sheet.image.tile_is_empty(&sheet.meta.grid, col, row) {
                " - empty"
            } else {
                ""
            }
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
            });
        if self.tile_draft.cell != Some((col, row)) {
            self.tile_draft = TileDraft {
                cell: Some((col, row)),
                name: tile.name.clone().unwrap_or_default(),
                tags: format_tags(&tile.tags),
                props: PropsDraft::from_map(&tile.props),
            };
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
        let props = self.tile_draft.props.draw(ui, "tile-properties");
        changed |= props.is_some();
        if changed {
            tile.name = (!self.tile_draft.name.is_empty()).then(|| self.tile_draft.name.clone());
            tile.tags = parse_tags(&self.tile_draft.tags);
            match self.tile_draft.props.validate() {
                Ok(props) => tile.props = props,
                Err(_) => return,
            }
        }
        if changed {
            match sheet
                .meta
                .validate_tile_edit(&tile, sheet.image.width, sheet.image.height)
            {
                Ok(()) => {
                    if index.map(|i| &sheet.meta.tiles[i]) != Some(&tile)
                        && !(index.is_none()
                            && tile.name.is_none()
                            && tile.tags.is_empty()
                            && tile.props.is_empty())
                    {
                        put_tile(&mut sheet.meta.tiles, tile);
                        sheet.dirty = true;
                        log::append_repeat(&mut self.log, "Tile metadata updated");
                    }
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
    fn cleared_tile_metadata_leaves_the_sparse_list() {
        let blank = TileMeta {
            col: 1,
            row: 2,
            name: None,
            tags: vec![],
            props: Default::default(),
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
