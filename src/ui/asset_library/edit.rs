use crate::sprite::{
    Grid, Sprite,
    format::{Part, Shape},
};
use std::collections::BTreeSet;

pub fn next_name<'a>(prefix: &str, names: impl IntoIterator<Item = &'a str>) -> String {
    let existing: BTreeSet<_> = names.into_iter().collect();
    (1u64..)
        .map(|n| format!("{prefix}-{n}"))
        .find(|name| !existing.contains(name.as_str()))
        .expect("available name")
}
fn cell_bounds(cells: &BTreeSet<(u32, u32)>) -> Option<(u32, u32, u32, u32)> {
    Some((
        cells.iter().map(|p| p.0).min()?,
        cells.iter().map(|p| p.1).min()?,
        cells.iter().map(|p| p.0).max()?,
        cells.iter().map(|p| p.1).max()?,
    ))
}
pub fn bottom_centre_pivot(w: u32, h: u32, tile: u32) -> [f32; 2] {
    [w as f32 * tile as f32 / 2.0, h as f32 * tile as f32]
}
pub fn set_from_cells(name: String, cells: &BTreeSet<(u32, u32)>, grid: &Grid) -> Option<Sprite> {
    let (col, row, max_col, max_row) = cell_bounds(cells)?;
    if max_col - col + 1 > 8 || max_row - row + 1 > 8 {
        return None;
    }
    Some(Sprite {
        id: 0,
        props: Default::default(),
        modified: None,
        name,
        shape: Shape::Set {
            canvas: [max_col - col + 1, max_row - row + 1],
            parts: cells
                .iter()
                .map(|&(c, r)| Part {
                    col: c,
                    row: r,
                    dx: ((c - col) * grid.tile) as i32,
                    dy: ((r - row) * grid.tile) as i32,
                    ..Default::default()
                })
                .collect(),
        },
        pivot: bottom_centre_pivot(max_col - col + 1, max_row - row + 1, grid.tile),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selections_names_and_references() {
        let grid = Grid {
            tile: 16,
            offset: [0, 0],
            spacing: 1,
        };
        let cells = [(1, 2), (2, 3)].into_iter().collect();
        let set = set_from_cells("x".into(), &cells, &grid).unwrap();
        assert_eq!(
            set.parts(&grid),
            vec![
                Part {
                    col: 1,
                    row: 2,
                    dx: 0,
                    dy: 0,
                    ..Default::default()
                },
                Part {
                    col: 2,
                    row: 3,
                    dx: 16,
                    dy: 16,
                    ..Default::default()
                }
            ]
        );
        assert_eq!(next_name("sprite", ["sprite-1", "sprite-3"]), "sprite-2");
        let (_, mut sheet) = crate::sprite::example();
        assert!(sheet.rename_sprite(1, "b", 1, (64, 16)).is_err());
        assert!(sheet.remove_sprite(1).is_err());
        sheet.rename_sprite(1, "renamed", 1, (64, 16)).unwrap();
        assert_eq!(sheet.animations[0].frames[0].sprite_id, 1);
        let error = sheet.remove_sprite(1).unwrap_err();
        assert!(error.contains("\"renamed\""), "{error}");
        assert!(sheet.remove_sprite(999).is_err());
        sheet.rename_sprite(1, "a", 1, (64, 16)).unwrap();
        sheet.sprites[0].modified = None;
        assert_eq!(
            crate::sprite::store::to_json(&sheet),
            crate::sprite::example::EXAMPLE_JSON
        );
        sheet.animations.clear();
        sheet.remove_sprite(1).unwrap();
        assert_eq!(sheet.sprites.len(), 3);
    }
    #[test]
    fn example_files_open_and_rename_roundtrip() {
        let temp = crate::test_support::TempDir::new("editor-roundtrip");
        let dir = &temp.0;
        let path = crate::sprite::example::write_example(dir).unwrap();
        let loaded = crate::sprite::store::load(&path).unwrap();
        let (_, mut sheet, dirty) = super::super::open_sheet(
            &loaded.image_path,
            &std::fs::read(&loaded.image_path).unwrap(),
            None,
            Some(loaded.sheet),
        )
        .unwrap();
        assert!(!dirty);
        sheet.rename_sprite(1, "temporary", 1, (64, 16)).unwrap();
        sheet.rename_sprite(1, "a", 1, (64, 16)).unwrap();
        sheet.sprites[0].modified = None;
        crate::sprite::store::save(&path, &sheet).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            crate::sprite::example::EXAMPLE_JSON
        );
    }
}
