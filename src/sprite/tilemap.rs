use crate::{
    raster::RgbaImage,
    sprite::{Transform, store::image_path_is_safe},
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cell {
    pub index: usize,
    #[serde(default, skip_serializing_if = "is_false")]
    pub flip_x: bool,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rotation: u16,
}
fn is_false(value: &bool) -> bool {
    !value
}
fn is_zero(value: &u16) -> bool {
    *value == 0
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TileMap {
    pub version: u32,
    pub tileset: String,
    pub tile: u32,
    pub cols: u32,
    pub rows: u32,
    pub cells: Vec<Cell>,
}
impl TileMap {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || self.tile == 0
            || !image_path_is_safe(&self.tileset)
            || Path::new(&self.tileset).components().count() != 1
        {
            return Err("invalid version, tile, or tileset filename".into());
        }
        if u64::from(self.cols) * u64::from(self.rows) != self.cells.len() as u64 {
            return Err("cell count does not match dimensions".into());
        }
        for cell in &self.cells {
            Transform::new(cell.flip_x, cell.rotation)?;
        }
        Ok(())
    }
    pub fn validate_against(&self, width: u32, height: u32) -> Result<(), String> {
        self.validate()?;
        let count = u64::from(width / self.tile) * u64::from(height / self.tile);
        if self.cells.iter().any(|cell| cell.index as u64 >= count) {
            return Err("tile index outside tileset".into());
        }
        Ok(())
    }
    pub fn rebuild(&self, tileset: &RgbaImage) -> Result<RgbaImage, String> {
        self.validate_against(tileset.width, tileset.height)?;
        let width = self
            .cols
            .checked_mul(self.tile)
            .ok_or("map width overflow")?;
        let height = self
            .rows
            .checked_mul(self.tile)
            .ok_or("map height overflow")?;
        let mut image = RgbaImage::new(width, height);
        let columns = tileset.width / self.tile;
        for (position, cell) in self.cells.iter().enumerate() {
            let index = u32::try_from(cell.index).map_err(|_| "tile index overflow")?;
            let tile = tileset
                .crop(
                    (index % columns) * self.tile,
                    (index / columns) * self.tile,
                    self.tile,
                    self.tile,
                )
                .ok_or("invalid tile")?
                .transformed(Transform::new(cell.flip_x, cell.rotation)?);
            let x = (position as u32 % self.cols) * self.tile;
            let y = (position as u32 / self.cols) * self.tile;
            for dy in 0..self.tile {
                for dx in 0..self.tile {
                    image.set_pixel(x + dx, y + dy, tile.pixel(dx, dy));
                }
            }
        }
        Ok(image)
    }
}
pub fn parse(text: &str) -> Result<TileMap, String> {
    let map: TileMap = serde_json::from_str(text).map_err(|e| e.to_string())?;
    map.validate()?;
    Ok(map)
}
pub fn to_json(map: &TileMap) -> String {
    format!(
        "{}\n",
        serde_json::to_string_pretty(map).expect("serialize tile map")
    )
}
pub fn map_path_for(output: &Path, source_stem: &str) -> PathBuf {
    output.with_file_name(format!("{source_stem}.map.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_roundtrip_and_rebuild() {
        let mut sheet = RgbaImage::new(2, 2);
        for y in 0..2 {
            for x in 0..2 {
                sheet.set_pixel(x, y, [(x + 2 * y) as u8, 0, 0, 255]);
            }
        }
        let mut map = TileMap {
            version: 1,
            tileset: "tiles.png".into(),
            tile: 2,
            cols: 2,
            rows: 1,
            cells: vec![
                Cell {
                    index: 0,
                    flip_x: false,
                    rotation: 0,
                },
                Cell {
                    index: 0,
                    flip_x: true,
                    rotation: 90,
                },
            ],
        };
        let json = to_json(&map);
        assert_eq!(to_json(&parse(&json).unwrap()), json);
        let rebuilt = map.rebuild(&sheet).unwrap();
        let rotated = sheet.transformed(Transform::new(true, 90).unwrap());
        assert_eq!(rebuilt.crop(2, 0, 2, 2), Some(rotated));
        map.cells[1].rotation = 45;
        assert!(map.validate().is_err());
        map.cells[1].rotation = 0;
        map.cells[1].index = 1;
        assert!(map.validate_against(2, 2).is_err());
        map.cells.pop();
        assert!(map.validate().is_err());
        assert!(parse(&json.replace("tiles.png", "../tiles.png")).is_err());
        assert!(parse(&json.replace("\"version\": 1", "\"unknown\": 1, \"version\": 1")).is_err());
    }
}
