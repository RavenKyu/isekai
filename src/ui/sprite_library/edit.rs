use crate::{
    raster::{self, CellRect, RgbaImage},
    sprite::{
        Grid, Sprite, SpriteSheet,
        format::{Part, Shape},
    },
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
pub fn rect_from_cells(name: String, cells: &BTreeSet<(u32, u32)>, grid: &Grid) -> Option<Sprite> {
    let (col, row, max_col, max_row) = cell_bounds(cells)?;
    let (cols, rows) = (max_col - col + 1, max_row - row + 1);
    Some(Sprite {
        name,
        shape: Shape::Rect {
            col,
            row,
            cols,
            rows,
        },
        pivot: [
            cols as f32 * grid.tile as f32 / 2.0,
            rows as f32 * grid.tile as f32 / 2.0,
        ],
    })
}
pub fn set_from_cells(name: String, cells: &BTreeSet<(u32, u32)>, grid: &Grid) -> Option<Sprite> {
    let (col, row, max_col, max_row) = cell_bounds(cells)?;
    Some(Sprite {
        name,
        shape: Shape::Set {
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
        pivot: [
            (max_col - col + 1) as f32 * grid.tile as f32 / 2.0,
            (max_row - row + 1) as f32 * grid.tile as f32 / 2.0,
        ],
    })
}
pub fn suggestions(image: &RgbaImage, grid: &Grid) -> Vec<CellRect> {
    let mut rects = Vec::new();
    for object in raster::find_objects(
        image,
        raster::Background::Auto,
        raster::DEFAULT_MIN_OBJECT_SIZE,
    ) {
        if let Some(rect) = object.snap(grid, image.width, image.height)
            && !rects.contains(&rect)
        {
            rects.push(rect);
        }
    }
    rects
}
impl SpriteSheet {
    pub fn rename_sprite(&mut self, old: &str, new: &str) -> Result<(), String> {
        if new.trim().is_empty() || self.sprites.iter().any(|s| s.name == new && s.name != old) {
            return Err(format!("empty or duplicate sprite name: {new:?}"));
        }
        let sprite = self
            .sprites
            .iter_mut()
            .find(|s| s.name == old)
            .ok_or("sprite no longer exists")?;
        sprite.name = new.into();
        for clip in &mut self.clips {
            for frame in &mut clip.frames {
                if frame.sprite == old {
                    frame.sprite = new.into();
                }
            }
        }
        Ok(())
    }
    pub fn remove_sprite(&mut self, name: &str) -> Result<(), String> {
        let references: Vec<_> = self
            .clips
            .iter()
            .filter(|c| c.frames.iter().any(|f| f.sprite == name))
            .map(|c| c.name.as_str())
            .collect();
        if !references.is_empty() {
            return Err(format!(
                "sprite {name:?} is used by clips: {}",
                references.join(", ")
            ));
        }
        let index = self
            .sprites
            .iter()
            .position(|s| s.name == name)
            .ok_or("sprite no longer exists")?;
        self.sprites.remove(index);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selections_names_references_and_suggestions() {
        let grid = Grid {
            tile: 16,
            offset: [0, 0],
            spacing: 1,
        };
        let cells = [(1, 2), (2, 3)].into_iter().collect();
        let rect = rect_from_cells("x".into(), &cells, &grid).unwrap();
        assert_eq!(
            rect.shape,
            Shape::Rect {
                col: 1,
                row: 2,
                cols: 2,
                rows: 2
            }
        );
        assert_eq!(rect.pivot, [16.0; 2]);
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
        assert!(sheet.rename_sprite("a", "b").is_err());
        assert!(sheet.remove_sprite("a").is_err());
        sheet.rename_sprite("a", "renamed").unwrap();
        assert_eq!(sheet.clips[0].frames[0].sprite, "renamed");
        assert!(sheet.remove_sprite("renamed").is_err());
        sheet.rename_sprite("renamed", "a").unwrap();
        assert_eq!(
            crate::sprite::store::to_json(&sheet),
            crate::sprite::example::EXAMPLE_JSON
        );
        sheet.clips.clear();
        sheet.remove_sprite("a").unwrap();
        assert_eq!(sheet.sprites.len(), 3);
        let mut image = RgbaImage::new(64, 64);
        image.fill_rect(16, 16, 32, 32, [10, 20, 30, 255]);
        let suggestions = suggestions(&image, &Grid { spacing: 0, ..grid });
        assert_eq!(
            suggestions,
            vec![CellRect {
                col: 1,
                row: 1,
                cols: 2,
                rows: 2
            }]
        );
    }
    #[test]
    fn example_files_open_and_rename_roundtrip() {
        let dir = std::env::temp_dir().join(format!("rn-editor-roundtrip-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let path = crate::sprite::example::write_example(&dir).unwrap();
        let loaded = crate::sprite::store::load(&path).unwrap();
        let (_, mut sheet, dirty) = super::super::open_sheet(
            &loaded.image_path,
            &std::fs::read(&loaded.image_path).unwrap(),
            None,
            Some(loaded.sheet),
        )
        .unwrap();
        assert!(!dirty);
        sheet.rename_sprite("a", "temporary").unwrap();
        sheet.rename_sprite("temporary", "a").unwrap();
        crate::sprite::store::save(&path, &sheet).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            crate::sprite::example::EXAMPLE_JSON
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
