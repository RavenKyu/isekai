use crate::{
    raster::{self, CellRect},
    sprite::{
        Animation, Grid, Sprite, SpriteSheet,
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
        id: 0,
        props: Default::default(),
        modified: None,
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
        pivot: [
            (max_col - col + 1) as f32 * grid.tile as f32 / 2.0,
            (max_row - row + 1) as f32 * grid.tile as f32 / 2.0,
        ],
    })
}
#[cfg(test)]
pub fn suggestions(image: &raster::RgbaImage, grid: &Grid) -> Vec<CellRect> {
    let objects = raster::find_objects(
        image,
        raster::Background::Auto,
        raster::DEFAULT_MIN_OBJECT_SIZE,
    );
    snap_objects(&objects, grid, image.width, image.height)
}
pub fn snap_objects(
    objects: &[raster::ObjectBox],
    grid: &Grid,
    width: u32,
    height: u32,
) -> Vec<CellRect> {
    let mut rects = Vec::new();
    for object in objects {
        if let Some(rect) = object.snap(grid, width, height)
            && !rects.contains(&rect)
        {
            rects.push(rect);
        }
    }
    rects
}
impl SpriteSheet {
    pub fn add_sprite(&mut self, mut sprite: Sprite, now: u64) -> u64 {
        sprite.id = self.allocate_id();
        sprite.modified = Some(now);
        let id = sprite.id;
        self.sprites.push(sprite);
        id
    }
    pub fn put_sprite(
        &mut self,
        id: u64,
        mut sprite: Sprite,
        now: u64,
        size: (u32, u32),
    ) -> Result<bool, String> {
        if sprite.id != id {
            return Err("sprite id cannot change".into());
        }
        let index = self
            .sprites
            .iter()
            .position(|s| s.id == id)
            .ok_or("sprite no longer exists")?;
        self.validate_sprite_edit(&sprite, size.0, size.1)
            .map_err(|e| e.join("; "))?;
        sprite.modified = self.sprites[index].modified;
        if self.sprites[index] == sprite {
            return Ok(false);
        }
        sprite.modified = Some(now);
        self.sprites[index] = sprite;
        Ok(true)
    }
    pub fn add_animation(&mut self, mut animation: Animation, now: u64) -> u64 {
        animation.id = self.allocate_id();
        animation.modified = Some(now);
        let id = animation.id;
        self.animations.push(animation);
        id
    }
    pub fn put_animation(
        &mut self,
        id: u64,
        mut animation: Animation,
        now: u64,
    ) -> Result<bool, String> {
        if animation.id != id {
            return Err("animation id cannot change".into());
        }
        let index = self
            .animations
            .iter()
            .position(|a| a.id == id)
            .ok_or("animation no longer exists")?;
        self.validate_animation_edit(&animation)
            .map_err(|e| e.join("; "))?;
        animation.modified = self.animations[index].modified;
        if self.animations[index] == animation {
            return Ok(false);
        }
        animation.modified = Some(now);
        self.animations[index] = animation;
        Ok(true)
    }
    pub fn rename_sprite(
        &mut self,
        id: u64,
        name: &str,
        now: u64,
        size: (u32, u32),
    ) -> Result<bool, String> {
        let mut sprite = self
            .sprites
            .iter()
            .find(|s| s.id == id)
            .ok_or("sprite no longer exists")?
            .clone();
        sprite.name = name.into();
        self.put_sprite(id, sprite, now, size)
    }
    pub fn remove_sprite(&mut self, name: &str) -> Result<(), String> {
        let id = self
            .sprites
            .iter()
            .find(|s| s.name == name)
            .ok_or("sprite no longer exists")?
            .id;
        let references: Vec<_> = self
            .animations
            .iter()
            .filter(|c| c.frames.iter().any(|f| f.sprite_id == id))
            .map(|c| c.name.as_str())
            .collect();
        if !references.is_empty() {
            return Err(format!(
                "sprite {name:?} is used by animations: {}",
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
    use crate::raster::RgbaImage;
    #[test]
    fn edits_track_real_changes_and_validate_only_the_item() {
        let (_, mut sheet) = crate::sprite::example();
        let sprite =
            rect_from_cells("new".into(), &[(0, 0)].into_iter().collect(), &sheet.grid).unwrap();
        let id = sheet.add_sprite(sprite, 10);
        assert_eq!(id, 6);
        assert_eq!(sheet.next_id, 7);
        assert_eq!(sheet.sprites.last().unwrap().modified, Some(10));
        let animation = crate::sprite::Animation {
            id: 0,
            name: "empty".into(),
            mode: crate::sprite::Mode::Loop,
            tags: vec![],
            props: Default::default(),
            frames: vec![],
            modified: None,
        };
        let aid = sheet.add_animation(animation, 11);
        assert_eq!(aid, 7);
        assert_eq!(sheet.animations.last().unwrap().modified, Some(11));
        let old_anim = sheet.animations[0].clone();
        let mut sprite = sheet.sprites.iter().find(|s| s.id == id).unwrap().clone();
        sprite.modified = Some(999);
        assert!(!sheet.put_sprite(id, sprite.clone(), 20, (64, 16)).unwrap());
        assert_eq!(sheet.sprites.last().unwrap().modified, Some(10));
        sprite.pivot = [2.0, 3.0];
        sprite.props.insert("key".into(), "value".into());
        assert!(sheet.put_sprite(id, sprite.clone(), 21, (64, 16)).unwrap());
        assert_eq!(sheet.sprites.last().unwrap().modified, Some(21));
        assert!(sheet.rename_sprite(id, "renamed", 22, (64, 16)).unwrap());
        assert_eq!(sheet.animations[0], old_anim);
        let mut bad = sheet.sprites.last().unwrap().clone();
        bad.id = 1;
        assert!(sheet.put_sprite(id, bad, 23, (64, 16)).is_err());
        let mut bad = sheet.sprites.last().unwrap().clone();
        bad.name = "a".into();
        assert!(sheet.put_sprite(id, bad, 23, (64, 16)).is_err());
        let mut bad = sheet.sprites.last().unwrap().clone();
        bad.shape = Shape::Rect {
            col: 99,
            row: 0,
            cols: 1,
            rows: 1,
        };
        assert!(sheet.put_sprite(id, bad, 23, (64, 16)).is_err());
        let mut animation = sheet.animations.last().unwrap().clone();
        assert!(!sheet.put_animation(aid, animation.clone(), 30).unwrap());
        animation
            .frames
            .push(crate::sprite::format::AnimationFrame {
                sprite_id: id,
                ms: 100,
            });
        animation.tags.push("walk".into());
        assert!(sheet.put_animation(aid, animation.clone(), 31).unwrap());
        assert_eq!(sheet.animations.last().unwrap().modified, Some(31));
        animation.frames[0].ms = 0;
        assert!(sheet.put_animation(aid, animation.clone(), 32).is_err());
        animation.frames[0].ms = 100;
        animation.frames[0].sprite_id = 999;
        assert!(sheet.put_animation(aid, animation, 32).is_err());
        assert_eq!(
            crate::sprite::store::to_json(&sheet),
            crate::sprite::store::to_json(&sheet)
        );
    }
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
        assert!(sheet.rename_sprite(1, "b", 1, (64, 16)).is_err());
        assert!(sheet.remove_sprite("a").is_err());
        sheet.rename_sprite(1, "renamed", 1, (64, 16)).unwrap();
        assert_eq!(sheet.animations[0].frames[0].sprite_id, 1);
        assert!(sheet.remove_sprite("renamed").is_err());
        sheet.rename_sprite(1, "a", 1, (64, 16)).unwrap();
        sheet.sprites[0].modified = None;
        assert_eq!(
            crate::sprite::store::to_json(&sheet),
            crate::sprite::example::EXAMPLE_JSON
        );
        sheet.animations.clear();
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
