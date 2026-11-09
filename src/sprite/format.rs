use super::{Transform, grid::Grid, timeline::Mode};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SpriteSheet {
    #[serde(serialize_with = "serialize_version")]
    pub version: u32,
    pub image: String,
    pub grid: Grid,
    pub tiles: Vec<TileMeta>,
    pub sprites: Vec<Sprite>,
    pub clips: Vec<Clip>,
}
// Keep field presence while decoding: even explicit identity fields are illegal in v1.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SheetWire {
    version: u32,
    image: String,
    grid: Grid,
    tiles: Vec<TileMeta>,
    sprites: Vec<SpriteWire>,
    clips: Vec<Clip>,
}
fn serialize_version<S: serde::Serializer>(_: &u32, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_u32(2)
}
impl<'de> Deserialize<'de> for SpriteSheet {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = SheetWire::deserialize(deserializer)?;
        if !matches!(wire.version, 1 | 2) {
            return Err(serde::de::Error::custom(
                "expected sprites.json version 1 or 2",
            ));
        }
        let sprites = wire
            .sprites
            .into_iter()
            .map(|sprite| {
                if wire.version == 1
                    && matches!(&sprite, SpriteWire::Set { parts, .. }
                if parts.iter().any(|p| p.flip_x.is_some() || p.rotation.is_some()))
                {
                    return Err(serde::de::Error::custom(
                        "version 1 cannot contain part transforms",
                    ));
                }
                sprite.into_sprite().map_err(serde::de::Error::custom)
            })
            .collect::<Result<_, D::Error>>()?;
        Ok(Self {
            version: wire.version,
            image: wire.image,
            grid: wire.grid,
            tiles: wire.tiles,
            sprites,
            clips: wire.clips,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TileMeta {
    pub col: u32,
    pub row: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub tags: Vec<String>,
    pub props: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation: Option<TileAnimation>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TileAnimation {
    pub mode: Mode,
    pub frames: Vec<TileFrame>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TileFrame {
    pub col: u32,
    pub row: u32,
    pub ms: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Sprite {
    pub name: String,
    #[serde(flatten)]
    pub shape: Shape,
    pub pivot: [f32; 2],
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum Shape {
    Rect {
        col: u32,
        row: u32,
        cols: u32,
        rows: u32,
    },
    Set {
        parts: Vec<Part>,
    },
}
// A dedicated wire enum rejects unknown fields without combining deserialize's
// flatten with deny_unknown_fields (an unsupported serde combination).
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
enum SpriteWire {
    Rect {
        name: String,
        col: u32,
        row: u32,
        cols: u32,
        rows: u32,
        pivot: [f32; 2],
    },
    Set {
        name: String,
        parts: Vec<PartWire>,
        pivot: [f32; 2],
    },
}
impl<'de> Deserialize<'de> for Sprite {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        SpriteWire::deserialize(deserializer)?
            .into_sprite()
            .map_err(serde::de::Error::custom)
    }
}
impl SpriteWire {
    fn into_sprite(self) -> Result<Sprite, String> {
        Ok(match self {
            Self::Rect {
                name,
                col,
                row,
                cols,
                rows,
                pivot,
            } => Sprite {
                name,
                pivot,
                shape: Shape::Rect {
                    col,
                    row,
                    cols,
                    rows,
                },
            },
            Self::Set { name, parts, pivot } => Sprite {
                name,
                pivot,
                shape: Shape::Set {
                    parts: parts
                        .into_iter()
                        .map(|p| {
                            let flip_x = p.flip_x.unwrap_or(false);
                            let rotation = p.rotation.unwrap_or(0);
                            Transform::new(flip_x, rotation)?;
                            Ok(Part {
                                col: p.col,
                                row: p.row,
                                dx: p.dx,
                                dy: p.dy,
                                flip_x,
                                rotation,
                            })
                        })
                        .collect::<Result<_, String>>()?,
                },
            },
        })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PartWire {
    col: u32,
    row: u32,
    dx: i32,
    dy: i32,
    #[serde(default, deserialize_with = "present")]
    flip_x: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    rotation: Option<u16>,
}
fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Part {
    pub col: u32,
    pub row: u32,
    pub dx: i32,
    pub dy: i32,
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
impl Part {
    pub fn transform(&self) -> Transform {
        Transform::new(self.flip_x, self.rotation).expect("validated part transform")
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clip {
    pub name: String,
    pub mode: Mode,
    pub tags: Vec<String>,
    pub props: BTreeMap<String, String>,
    pub frames: Vec<ClipFrame>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClipFrame {
    pub sprite: String,
    pub ms: u32,
}

impl Sprite {
    /// Expand a validated sprite. Source spacing never appears in object offsets.
    pub fn parts(&self, grid: &Grid) -> Vec<Part> {
        match &self.shape {
            Shape::Rect {
                col,
                row,
                cols,
                rows,
            } => (0..*rows)
                .flat_map(|r| {
                    (0..*cols).map(move |c| Part {
                        col: col + c,
                        row: row + r,
                        dx: (u64::from(c) * u64::from(grid.tile)) as i32,
                        dy: (u64::from(r) * u64::from(grid.tile)) as i32,
                        ..Default::default()
                    })
                })
                .collect(),
            Shape::Set { parts } => parts.clone(),
        }
    }
    /// Top-left object offset; pivots are measured from the bounding box here.
    pub fn bounds_origin(&self) -> (i32, i32) {
        match &self.shape {
            Shape::Rect { .. } => (0, 0),
            Shape::Set { parts } => (
                parts.iter().map(|p| p.dx).min().unwrap_or(0),
                parts.iter().map(|p| p.dy).min().unwrap_or(0),
            ),
        }
    }
    pub fn bounds(&self, grid: &Grid) -> (u32, u32) {
        match &self.shape {
            Shape::Rect { cols, rows, .. } => (
                (u64::from(*cols) * u64::from(grid.tile)) as u32,
                (u64::from(*rows) * u64::from(grid.tile)) as u32,
            ),
            Shape::Set { parts } => {
                let (x, y) = self.bounds_origin();
                (
                    parts
                        .iter()
                        .map(|p| i64::from(p.dx) - i64::from(x) + i64::from(grid.tile))
                        .max()
                        .unwrap_or(0) as u32,
                    parts
                        .iter()
                        .map(|p| i64::from(p.dy) - i64::from(y) + i64::from(grid.tile))
                        .max()
                        .unwrap_or(0) as u32,
                )
            }
        }
    }
}
impl SpriteSheet {
    pub fn new_for(image_name: &str, grid: Grid) -> Self {
        Self {
            version: 2,
            image: image_name.into(),
            grid,
            tiles: vec![],
            sprites: vec![],
            clips: vec![],
        }
    }
    pub fn normalize(&mut self) {
        self.tiles.sort_by_key(|t| (t.row, t.col));
        self.sprites.sort_by(|a, b| a.name.cmp(&b.name));
        self.clips.sort_by(|a, b| a.name.cmp(&b.name));
    }
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if !matches!(self.version, 1 | 2) {
            errors.push(format!(
                "unsupported sprites.json version {} (expected 1 or 2)",
                self.version
            ));
        }
        if !super::store::image_path_is_safe(&self.image) {
            errors.push(format!("unsafe image path: {:?}", self.image));
        }
        if let Err(e) = self.grid.validate() {
            errors.push(e);
        }
        let mut cells = BTreeSet::new();
        let mut tile_names = BTreeSet::new();
        for tile in &self.tiles {
            if !cells.insert((tile.col, tile.row)) {
                errors.push(format!("duplicate tile ({}, {})", tile.col, tile.row));
            }
            if let Some(name) = &tile.name
                && (name.trim().is_empty() || !tile_names.insert(name))
            {
                errors.push(format!("empty or duplicate tile name: {name:?}"));
            }
            if let Some(animation) = &tile.animation
                && (animation.frames.is_empty() || animation.frames.iter().any(|f| f.ms == 0))
            {
                errors.push(format!(
                    "tile ({}, {}) needs animation frames with ms >= 1",
                    tile.col, tile.row
                ));
            }
        }
        let mut names = BTreeSet::new();
        for sprite in &self.sprites {
            if sprite.name.trim().is_empty() || !names.insert(sprite.name.as_str()) {
                errors.push(format!("empty or duplicate sprite name: {:?}", sprite.name));
            }
            if sprite.pivot.iter().any(|p| !p.is_finite()) {
                errors.push(format!("sprite {:?} pivot must be finite", sprite.name));
            }
            match &sprite.shape {
                Shape::Rect {
                    col,
                    row,
                    cols,
                    rows,
                } => {
                    if *cols == 0
                        || *rows == 0
                        || col.checked_add(*cols).is_none()
                        || row.checked_add(*rows).is_none()
                        || u64::from(*cols) * u64::from(self.grid.tile) > i32::MAX as u64
                        || u64::from(*rows) * u64::from(self.grid.tile) > i32::MAX as u64
                    {
                        errors.push(format!(
                            "sprite {:?} has invalid rectangle dimensions",
                            sprite.name
                        ));
                    }
                }
                Shape::Set { parts } => {
                    for part in parts {
                        if let Err(error) = Transform::new(part.flip_x, part.rotation) {
                            errors.push(error);
                        }
                    }
                    let (x, y) = sprite.bounds_origin();
                    if parts.is_empty()
                        || parts.iter().any(|p| {
                            i64::from(p.dx) - i64::from(x) + i64::from(self.grid.tile)
                                > i64::from(i32::MAX)
                                || i64::from(p.dy) - i64::from(y) + i64::from(self.grid.tile)
                                    > i64::from(i32::MAX)
                        })
                    {
                        errors.push(format!(
                            "sprite {:?} needs parts with representable bounds",
                            sprite.name
                        ));
                    }
                }
            }
        }
        let mut clips = BTreeSet::new();
        for clip in &self.clips {
            if clip.name.trim().is_empty() || !clips.insert(&clip.name) {
                errors.push(format!("empty or duplicate clip name: {:?}", clip.name));
            }
            if clip.frames.is_empty() {
                errors.push(format!("clip {:?} needs frames", clip.name));
            }
            for frame in &clip.frames {
                if frame.ms == 0 {
                    errors.push(format!("clip {:?} frame ms must be >= 1", clip.name));
                }
                if !names.contains(frame.sprite.as_str()) {
                    errors.push(format!(
                        "clip {:?} references missing sprite {:?}",
                        clip.name, frame.sprite
                    ));
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    pub fn validate_bounds(&self, width: u32, height: u32) -> Result<(), Vec<String>> {
        let mut errors = self.validate().err().unwrap_or_default();
        let cells = self.grid.cells(width, height);
        let inside = |col: u32, row: u32| col < cells.cols && row < cells.rows;
        for t in &self.tiles {
            if !inside(t.col, t.row) {
                errors.push(format!(
                    "tile ({}, {}) outside {}x{} grid",
                    t.col, t.row, cells.cols, cells.rows
                ));
            }
            if let Some(animation) = &t.animation {
                for f in &animation.frames {
                    if !inside(f.col, f.row) {
                        errors.push(format!(
                            "animation tile ({}, {}) outside grid",
                            f.col, f.row
                        ));
                    }
                }
            }
        }
        for sprite in &self.sprites {
            let valid = match &sprite.shape {
                Shape::Rect {
                    col,
                    row,
                    cols,
                    rows,
                } => {
                    *cols > 0
                        && *rows > 0
                        && col.checked_add(*cols).is_some_and(|end| end <= cells.cols)
                        && row.checked_add(*rows).is_some_and(|end| end <= cells.rows)
                }
                Shape::Set { parts } => parts.iter().all(|p| inside(p.col, p.row)),
            };
            if !valid {
                errors.push(format!(
                    "sprite {:?} outside {}x{} grid",
                    sprite.name, cells.cols, cells.rows
                ));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{example, store};
    use super::*;
    fn fixture() -> serde_json::Value {
        serde_json::from_str(super::super::example::EXAMPLE_JSON).unwrap()
    }
    #[test]
    fn rejects_invalid_metadata_at_read_time() {
        let mutations: Vec<(&str, serde_json::Value)> = vec![
            ("version", serde_json::json!(3)),
            ("unknown", serde_json::json!(true)),
        ];
        for (key, value) in mutations {
            let mut v = fixture();
            v[key] = value;
            assert!(store::parse(&v.to_string()).is_err());
        }
        for pointer in ["/grid", "/sprites/0", "/clips/0", "/clips/0/frames/0"] {
            let mut v = fixture();
            v.pointer_mut(pointer).unwrap()["unknown"] = true.into();
            assert!(store::parse(&v.to_string()).is_err(), "{pointer}");
        }
        let mut v = fixture();
        v["clips"][0]["frames"][0]["ms"] = 0.into();
        assert!(store::parse(&v.to_string()).is_err());
        let mut v = fixture();
        v["clips"][0]["frames"] = serde_json::json!([]);
        assert!(store::parse(&v.to_string()).is_err());
        let mut v = fixture();
        v["sprites"][1]["name"] = "a".into();
        // Without clips only the duplicate name, not cycle's frame "b", can reject this.
        v["clips"] = serde_json::json!([]);
        assert!(store::parse(&v.to_string()).is_err());
        let mut v = fixture();
        let clip = v["clips"][0].clone();
        v["clips"].as_array_mut().unwrap().push(clip);
        assert!(store::parse(&v.to_string()).is_err());
        let mut v = fixture();
        v["clips"][0]["frames"][0]["sprite"] = "missing".into();
        assert!(store::parse(&v.to_string()).is_err());
        let tile = serde_json::json!({"col":0,"row":0,"tags":[],"props":{},"animation":{"mode":"pingpong","frames":[{"col":1,"row":0,"ms":10}]}});
        let mut v = fixture();
        v["tiles"] = serde_json::json!([tile.clone(), tile.clone()]);
        assert!(store::parse(&v.to_string()).is_err());
        for (pointer, value) in [
            ("/tiles/0/animation/frames/0/ms", serde_json::json!(0)),
            ("/tiles/0/animation/frames", serde_json::json!([])),
            ("/tiles/0/animation/frames/0/extra", serde_json::json!(1)),
            ("/tiles/0/props/number", serde_json::json!(1)),
        ] {
            let mut v = fixture();
            v["tiles"] = serde_json::json!([tile.clone()]);
            let (parent, key) = pointer.rsplit_once('/').unwrap();
            v.pointer_mut(parent).unwrap()[key] = value;
            assert!(store::parse(&v.to_string()).is_err(), "{pointer}");
        }
        let mut v = fixture();
        v["sprites"][0] = serde_json::json!({"name":"a","kind":"set","parts":[{"col":0,"row":0,"dx":0,"dy":0,"extra":1}],"pivot":[0.0,0.0]});
        assert!(store::parse(&v.to_string()).is_err());
    }
    #[test]
    fn bounds_normalization_and_sprite_parts() {
        let (_, mut sheet) = example();
        sheet.sprites.reverse();
        let json = store::to_json(&sheet);
        assert_eq!(json, super::super::example::EXAMPLE_JSON);
        // Input order is retained until explicitly normalized for saving.
        let shuffled = serde_json::to_string(&sheet).unwrap();
        assert_eq!(store::parse(&shuffled).unwrap(), sheet);
        sheet.grid = Grid {
            tile: 16,
            offset: [0, 0],
            spacing: 1,
        };
        let rect = Sprite {
            name: "rect".into(),
            pivot: [16.0, 32.0],
            shape: Shape::Rect {
                col: 1,
                row: 2,
                cols: 2,
                rows: 2,
            },
        };
        assert_eq!(
            rect.parts(&sheet.grid),
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
                    row: 2,
                    dx: 16,
                    dy: 0,
                    ..Default::default()
                },
                Part {
                    col: 1,
                    row: 3,
                    dx: 0,
                    dy: 16,
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
        assert_eq!(rect.bounds(&sheet.grid), (32, 32));
        let set = Sprite {
            name: "set".into(),
            pivot: [0.0, 0.0],
            shape: Shape::Set {
                parts: vec![
                    Part {
                        col: 0,
                        row: 0,
                        dx: -16,
                        dy: -8,
                        ..Default::default()
                    },
                    Part {
                        col: 1,
                        row: 0,
                        dx: 16,
                        dy: 8,
                        ..Default::default()
                    },
                ],
            },
        };
        assert_eq!(set.bounds(&sheet.grid), (48, 32));
        assert_eq!(set.bounds_origin(), (-16, -8));
        let mut sheet = SpriteSheet::new_for(
            "x.png",
            Grid {
                tile: 16,
                offset: [0, 0],
                spacing: 0,
            },
        );
        for (col, row) in [(16, 0), (0, 9)] {
            sheet.tiles = vec![TileMeta {
                col,
                row,
                name: None,
                tags: vec![],
                props: Default::default(),
                animation: None,
            }];
            assert!(sheet.validate_bounds(256, 144).is_err());
        }
        sheet.tiles.clear();
        sheet.sprites.push(rect);
        sheet.sprites.push(set);
        sheet.validate_bounds(256, 144).unwrap();
        let parsed = store::parse(&store::to_json(&sheet)).unwrap();
        assert_eq!(parsed, sheet);
        sheet.sprites[0].shape = Shape::Rect {
            col: 15,
            row: 8,
            cols: 2,
            rows: 1,
        };
        assert!(sheet.validate_bounds(256, 144).is_err());
    }
}

#[cfg(test)]
mod version_tests {
    use super::super::{example, store};
    #[test]
    fn legacy_reads_always_write_v2_and_transform_fields_are_strict() {
        let mut value: serde_json::Value = serde_json::from_str(example::EXAMPLE_JSON).unwrap();
        value["version"] = 1.into();
        let legacy = store::parse(&value.to_string()).unwrap();
        assert_eq!(legacy.version, 1);
        assert_eq!(store::to_json(&legacy), example::EXAMPLE_JSON);
        value["sprites"][0] = serde_json::json!({"name":"a", "kind":"set",
            "parts":[{"col":0,"row":0,"dx":0,"dy":0}],"pivot":[0.0,0.0]});
        for (field, identity) in [
            ("flip_x", serde_json::json!(false)),
            ("rotation", serde_json::json!(0)),
        ] {
            let mut v = value.clone();
            v["sprites"][0]["parts"][0][field] = identity;
            assert!(store::parse(&v.to_string()).is_err(), "v1 {field}");
        }
        value["version"] = 2.into();
        for (field, invalid) in [
            ("rotation", serde_json::json!(45)),
            ("rotation", serde_json::json!(-90)),
            ("rotation", serde_json::json!("90")),
            ("rotation", serde_json::Value::Null),
            ("flip_x", serde_json::Value::Null),
            ("unknown", serde_json::json!(true)),
        ] {
            let mut v = value.clone();
            v["sprites"][0]["parts"][0][field] = invalid;
            assert!(store::parse(&v.to_string()).is_err(), "{field}: {v}");
        }
        value["sprites"][0]["parts"][0]["flip_x"] = true.into();
        value["sprites"][0]["parts"][0]["rotation"] = 270.into();
        let sheet = store::parse(&value.to_string()).unwrap();
        let canonical = store::to_json(&sheet);
        assert_eq!(
            store::to_json(&store::parse(&canonical).unwrap()),
            canonical
        );
        assert!(canonical.contains("\"flip_x\": true"));
        assert!(canonical.contains("\"rotation\": 270"));
    }
}
