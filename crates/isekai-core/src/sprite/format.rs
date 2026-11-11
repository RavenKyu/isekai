use super::{Transform, grid::Grid, timeline::Mode};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SpriteSheet {
    #[serde(serialize_with = "serialize_version")]
    pub version: u32,
    pub image: String,
    pub grid: Grid,
    pub next_id: u64,
    pub tiles: Vec<TileMeta>,
    pub sprites: Vec<Sprite>,
    pub animations: Vec<Animation>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SheetWire {
    version: u32,
    image: String,
    grid: Grid,
    #[serde(default, deserialize_with = "present")]
    next_id: Option<u64>,
    tiles: Vec<TileWire>,
    sprites: Vec<SpriteWire>,
    #[serde(default, deserialize_with = "present")]
    clips: Option<Vec<AnimationWire>>,
    #[serde(default, deserialize_with = "present")]
    animations: Option<Vec<AnimationWire>>,
}
fn serialize_version<S: serde::Serializer>(_: &u32, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_u32(3)
}
impl<'de> Deserialize<'de> for SpriteSheet {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        decode_wire(SheetWire::deserialize(deserializer)?)
            .map(|(sheet, _)| sheet)
            .map_err(serde::de::Error::custom)
    }
}
pub(super) fn decode(text: &str) -> Result<(SpriteSheet, Vec<(u32, u32)>), serde_json::Error> {
    let wire: SheetWire = serde_json::from_str(text)?;
    decode_wire(wire).map_err(serde::de::Error::custom)
}
fn decode_wire(wire: SheetWire) -> Result<(SpriteSheet, Vec<(u32, u32)>), String> {
    let version = wire.version;
    if !matches!(version, 1..=3) {
        return Err("expected sprites.json version 1, 2 or 3".into());
    }
    let legacy = version < 3;
    if legacy && (wire.next_id.is_some() || wire.animations.is_some() || wire.clips.is_none())
        || !legacy && (wire.next_id.is_none() || wire.animations.is_none() || wire.clips.is_some())
    {
        return Err("fields do not match sprites.json version".into());
    }
    let mut dropped = Vec::new();
    let mut tiles = Vec::new();
    for tile in wire.tiles {
        if tile.animation.is_some() {
            if !legacy {
                return Err("version 3 cannot contain tile animation".into());
            }
            dropped.push((tile.col, tile.row));
        }
        if tile.name.is_some()
            || !tile.tags.is_empty()
            || !tile.props.is_empty()
            || tile.animation.is_none()
        {
            tiles.push(TileMeta {
                col: tile.col,
                row: tile.row,
                name: tile.name,
                tags: tile.tags,
                props: tile.props,
            });
        }
    }
    let mut sprites = wire
        .sprites
        .into_iter()
        .map(|s| s.into_sprite(version, wire.grid.tile))
        .collect::<Result<Vec<_>, _>>()?;
    let anim_wire = if legacy {
        wire.clips.unwrap()
    } else {
        wire.animations.unwrap()
    };
    let mut animations = Vec::new();
    if legacy {
        let mut order: Vec<usize> = (0..sprites.len()).collect();
        order.sort_by(|&a, &b| sprites[a].name.cmp(&sprites[b].name));
        for (i, index) in order.into_iter().enumerate() {
            sprites[index].id = i as u64 + 1;
        }
    }
    for a in anim_wire {
        animations.push(a.into_animation(version, &sprites)?);
    }
    let next_id = if legacy {
        let mut order: Vec<usize> = (0..animations.len()).collect();
        order.sort_by(|&a, &b| animations[a].name.cmp(&animations[b].name));
        for (i, index) in order.into_iter().enumerate() {
            animations[index].id = sprites.len() as u64 + i as u64 + 1;
        }
        sprites.len() as u64 + animations.len() as u64 + 1
    } else {
        wire.next_id.unwrap()
    };
    Ok((
        SpriteSheet {
            version,
            image: wire.image,
            grid: wire.grid,
            next_id,
            tiles,
            sprites,
            animations,
        },
        dropped,
    ))
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TileMeta {
    pub col: u32,
    pub row: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub tags: Vec<String>,
    pub props: BTreeMap<String, String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TileWire {
    col: u32,
    row: u32,
    #[serde(default)]
    name: Option<String>,
    tags: Vec<String>,
    props: BTreeMap<String, String>,
    #[serde(default, deserialize_with = "present")]
    animation: Option<LegacyTileAnimation>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyTileAnimation {
    #[allow(dead_code)]
    mode: Mode,
    #[allow(dead_code)]
    frames: Vec<LegacyTileFrame>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyTileFrame {
    #[allow(dead_code)]
    col: u32,
    #[allow(dead_code)]
    row: u32,
    #[allow(dead_code)]
    ms: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Sprite {
    pub id: u64,
    pub name: String,
    #[serde(flatten)]
    pub shape: Shape,
    pub pivot: [f32; 2],
    pub props: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<u64>,
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
        canvas: [u32; 2],
        parts: Vec<Part>,
    },
}
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
        #[serde(default, deserialize_with = "present")]
        id: Option<u64>,
        #[serde(default, deserialize_with = "present")]
        props: Option<BTreeMap<String, String>>,
        #[serde(default, deserialize_with = "present")]
        modified: Option<u64>,
    },
    Set {
        name: String,
        parts: Vec<PartWire>,
        pivot: [f32; 2],
        #[serde(default, deserialize_with = "present")]
        id: Option<u64>,
        #[serde(default, deserialize_with = "present")]
        props: Option<BTreeMap<String, String>>,
        #[serde(default, deserialize_with = "present")]
        modified: Option<u64>,
        #[serde(default, deserialize_with = "present")]
        canvas: Option<[u32; 2]>,
    },
}
impl<'de> Deserialize<'de> for Sprite {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        SpriteWire::deserialize(deserializer)?
            .into_sprite(3, 1)
            .map_err(serde::de::Error::custom)
    }
}
impl SpriteWire {
    fn into_sprite(self, version: u32, tile: u32) -> Result<Sprite, String> {
        let (name, pivot, id, props, modified, shape) = match self {
            Self::Rect {
                name,
                col,
                row,
                cols,
                rows,
                pivot,
                id,
                props,
                modified,
            } => (
                name,
                pivot,
                id,
                props,
                modified,
                Shape::Rect {
                    col,
                    row,
                    cols,
                    rows,
                },
            ),
            Self::Set {
                name,
                parts,
                pivot,
                id,
                props,
                modified,
                canvas,
            } => {
                if version == 1
                    && parts
                        .iter()
                        .any(|p| p.flip_x.is_some() || p.rotation.is_some())
                {
                    return Err("version 1 cannot contain part transforms".into());
                }
                if (version < 3) == canvas.is_some() {
                    return Err("canvas does not match sprites.json version".into());
                }
                let mut parts = parts
                    .into_iter()
                    .map(Part::try_from)
                    .collect::<Result<Vec<_>, _>>()?;
                let canvas = if version < 3 {
                    let min_x = parts.iter().map(|p| p.dx).min().unwrap_or(0);
                    let min_y = parts.iter().map(|p| p.dy).min().unwrap_or(0);
                    let max_x = parts
                        .iter()
                        .map(|p| i64::from(p.dx) - i64::from(min_x) + i64::from(tile))
                        .max()
                        .unwrap_or(i64::from(tile));
                    let max_y = parts
                        .iter()
                        .map(|p| i64::from(p.dy) - i64::from(min_y) + i64::from(tile))
                        .max()
                        .unwrap_or(i64::from(tile));
                    for p in &mut parts {
                        p.dx = i32::try_from(i64::from(p.dx) - i64::from(min_x))
                            .map_err(|_| "invalid legacy bounds")?;
                        p.dy = i32::try_from(i64::from(p.dy) - i64::from(min_y))
                            .map_err(|_| "invalid legacy bounds")?;
                    }
                    let t = i64::from(tile.max(1));
                    [
                        u32::try_from((max_x + t - 1) / t).map_err(|_| "invalid legacy bounds")?,
                        u32::try_from((max_y + t - 1) / t).map_err(|_| "invalid legacy bounds")?,
                    ]
                } else {
                    canvas.unwrap()
                };
                (
                    name,
                    pivot,
                    id,
                    props,
                    modified,
                    Shape::Set { canvas, parts },
                )
            }
        };
        if version < 3 && (id.is_some() || props.is_some() || modified.is_some())
            || version == 3 && (id.is_none() || props.is_none())
        {
            return Err("sprite fields do not match sprites.json version".into());
        }
        Ok(Sprite {
            id: id.unwrap_or(0),
            name,
            shape,
            pivot,
            props: props.unwrap_or_default(),
            modified,
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
#[serde(try_from = "PartWire", deny_unknown_fields)]
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
impl TryFrom<PartWire> for Part {
    type Error = String;

    fn try_from(wire: PartWire) -> Result<Self, Self::Error> {
        let flip_x = wire.flip_x.unwrap_or(false);
        let rotation = wire.rotation.unwrap_or(0);
        Transform::new(flip_x, rotation)?;
        Ok(Self {
            col: wire.col,
            row: wire.row,
            dx: wire.dx,
            dy: wire.dy,
            flip_x,
            rotation,
        })
    }
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
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Animation {
    pub id: u64,
    pub name: String,
    pub mode: Mode,
    pub tags: Vec<String>,
    pub props: BTreeMap<String, String>,
    pub frames: Vec<AnimationFrame>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<u64>,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AnimationFrame {
    pub sprite_id: u64,
    pub ms: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AnimationWire {
    name: String,
    mode: Mode,
    tags: Vec<String>,
    props: BTreeMap<String, String>,
    frames: Vec<AnimationFrameWire>,
    #[serde(default, deserialize_with = "present")]
    id: Option<u64>,
    #[serde(default, deserialize_with = "present")]
    modified: Option<u64>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AnimationFrameWire {
    ms: u32,
    #[serde(default, deserialize_with = "present")]
    sprite: Option<String>,
    #[serde(default, deserialize_with = "present")]
    sprite_id: Option<u64>,
}
impl AnimationWire {
    fn into_animation(self, version: u32, sprites: &[Sprite]) -> Result<Animation, String> {
        let legacy = version < 3;
        if legacy && (self.id.is_some() || self.modified.is_some()) || !legacy && self.id.is_none()
        {
            return Err("animation fields do not match sprites.json version".into());
        }
        let frames = self
            .frames
            .into_iter()
            .map(|f| {
                let sprite_id = if legacy && f.sprite_id.is_none() {
                    let name = f.sprite.ok_or("legacy frame needs sprite")?;
                    sprites
                        .iter()
                        .find(|s| s.name == name)
                        .map(|s| s.id)
                        .ok_or_else(|| format!("animation references missing sprite {name:?}"))?
                } else if !legacy && f.sprite.is_none() {
                    f.sprite_id.ok_or("frame needs sprite_id")?
                } else {
                    return Err("frame fields do not match sprites.json version".into());
                };
                Ok(AnimationFrame {
                    sprite_id,
                    ms: f.ms,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Animation {
            id: self.id.unwrap_or(0),
            name: self.name,
            mode: self.mode,
            tags: self.tags,
            props: self.props,
            frames,
            modified: self.modified,
        })
    }
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
            Shape::Set { parts, .. } => parts.clone(),
        }
    }
    pub fn bounds(&self, grid: &Grid) -> (u32, u32) {
        let [w, h] = match &self.shape {
            Shape::Rect { cols, rows, .. } => [*cols, *rows],
            Shape::Set { canvas, .. } => *canvas,
        };
        (
            (u64::from(w) * u64::from(grid.tile)) as u32,
            (u64::from(h) * u64::from(grid.tile)) as u32,
        )
    }
}
impl SpriteSheet {
    pub fn new_for(image_name: &str, grid: Grid) -> Self {
        Self {
            version: 3,
            next_id: 1,
            image: image_name.into(),
            grid,
            tiles: vec![],
            sprites: vec![],
            animations: vec![],
        }
    }
    pub fn normalize(&mut self) {
        self.tiles.sort_by_key(|t| (t.row, t.col));
        self.sprites.sort_by_key(|a| a.id);
        self.animations.sort_by_key(|a| a.id);
    }
    pub fn allocate_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).expect("id space exhausted");
        id
    }
    pub fn validate(&self) -> Result<(), Vec<String>> {
        self.validate_internal(true)
    }
    fn validate_internal(&self, complete: bool) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if !matches!(self.version, 1..=3) {
            errors.push(format!(
                "unsupported sprites.json version {} (expected 1, 2 or 3)",
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
        }
        let mut names = BTreeSet::new();
        for sprite in &self.sprites {
            if sprite.name.trim().is_empty() || !names.insert(sprite.name.as_str()) {
                errors.push(format!("empty or duplicate sprite name: {:?}", sprite.name));
            }
            self.sprite_shape_errors(sprite, complete, &mut errors);
        }
        let mut ids = BTreeSet::new();
        // The upper bound leaves allocate_id room, so a hand-edited next_id cannot overflow it.
        if self.next_id == 0 || self.next_id > i64::MAX as u64 {
            errors.push(format!(
                "next_id must be between 1 and {}, got {}",
                i64::MAX,
                self.next_id
            ));
        }
        for id in self
            .sprites
            .iter()
            .map(|s| s.id)
            .chain(self.animations.iter().map(|a| a.id))
        {
            if id == 0 || id >= self.next_id {
                errors.push(format!(
                    "id {id} must be positive and below next_id {}",
                    self.next_id
                ));
            }
            if !ids.insert(id) {
                errors.push(format!("duplicate id {id}"));
            }
        }
        let mut names = BTreeSet::new();
        for animation in &self.animations {
            if animation.name.trim().is_empty() || !names.insert(&animation.name) {
                errors.push(format!(
                    "empty or duplicate animation name: {:?}",
                    animation.name
                ));
            }
            if complete && animation.frames.is_empty() {
                errors.push(format!("animation {:?} needs frames", animation.name));
            }
            self.animation_frame_errors(animation, &mut errors);
        }
        into_result(errors)
    }
    fn sprite_shape_errors(&self, sprite: &Sprite, complete: bool, errors: &mut Vec<String>) {
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
            Shape::Set { canvas, parts } => {
                let w = u64::from(canvas[0]) * u64::from(self.grid.tile);
                let h = u64::from(canvas[1]) * u64::from(self.grid.tile);
                if canvas.contains(&0) || w > i32::MAX as u64 || h > i32::MAX as u64 {
                    errors.push(format!("sprite {:?} has invalid canvas", sprite.name));
                }
                if complete && parts.is_empty() {
                    errors.push(format!(
                        "sprite {:?} needs parts with representable bounds",
                        sprite.name
                    ));
                }
                for p in parts {
                    if let Err(error) = Transform::new(p.flip_x, p.rotation) {
                        errors.push(error);
                    }
                    if p.dx < 0
                        || p.dy < 0
                        || i64::from(p.dx) + i64::from(self.grid.tile) > w as i64
                        || i64::from(p.dy) + i64::from(self.grid.tile) > h as i64
                    {
                        errors.push(format!("sprite {:?} part outside its canvas", sprite.name));
                    }
                }
            }
        }
    }
    fn sprite_bounds_errors(
        &self,
        sprite: &Sprite,
        width: u32,
        height: u32,
        errors: &mut Vec<String>,
    ) {
        let cells = self.grid.cells(width, height);
        let valid = match &sprite.shape {
            Shape::Rect {
                col,
                row,
                cols,
                rows,
            } => {
                *cols > 0
                    && *rows > 0
                    && col.checked_add(*cols).is_some_and(|v| v <= cells.cols)
                    && row.checked_add(*rows).is_some_and(|v| v <= cells.rows)
            }
            Shape::Set { parts, .. } => parts
                .iter()
                .all(|p| p.col < cells.cols && p.row < cells.rows),
        };
        if !valid {
            errors.push(format!(
                "sprite {:?} outside {}x{} grid",
                sprite.name, cells.cols, cells.rows
            ));
        }
    }
    fn tile_bounds_errors(
        &self,
        tile: &TileMeta,
        width: u32,
        height: u32,
        errors: &mut Vec<String>,
    ) {
        let cells = self.grid.cells(width, height);
        if tile.col >= cells.cols || tile.row >= cells.rows {
            errors.push(format!(
                "tile ({}, {}) outside {}x{} grid",
                tile.col, tile.row, cells.cols, cells.rows
            ));
        }
    }
    pub fn validate_sprite_edit(
        &self,
        sprite: &Sprite,
        width: u32,
        height: u32,
    ) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if sprite.name.trim().is_empty()
            || self
                .sprites
                .iter()
                .any(|s| s.id != sprite.id && s.name == sprite.name)
        {
            errors.push(format!("empty or duplicate sprite name: {:?}", sprite.name));
        }
        self.sprite_shape_errors(sprite, false, &mut errors);
        self.sprite_bounds_errors(sprite, width, height, &mut errors);
        into_result(errors)
    }
    /// Checks a tile name against the other tiles; `None` clears the name and is always valid.
    pub fn validate_tile_name(&self, cell: (u32, u32), name: Option<&str>) -> Result<(), String> {
        match name {
            Some(name)
                if name.trim().is_empty()
                    || self
                        .tiles
                        .iter()
                        .any(|t| (t.col, t.row) != cell && t.name.as_deref() == Some(name)) =>
            {
                Err(format!("empty or duplicate tile name: {name:?}"))
            }
            _ => Ok(()),
        }
    }
    pub fn validate_tile_edit(
        &self,
        tile: &TileMeta,
        width: u32,
        height: u32,
    ) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if let Err(error) = self.validate_tile_name((tile.col, tile.row), tile.name.as_deref()) {
            errors.push(error);
        }
        self.tile_bounds_errors(tile, width, height, &mut errors);
        into_result(errors)
    }
    pub fn validate_animation_edit(&self, animation: &Animation) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if animation.name.trim().is_empty()
            || self
                .animations
                .iter()
                .any(|a| a.id != animation.id && a.name == animation.name)
        {
            errors.push(format!(
                "empty or duplicate animation name: {:?}",
                animation.name
            ));
        }
        self.animation_frame_errors(animation, &mut errors);
        into_result(errors)
    }
    fn animation_frame_errors(&self, animation: &Animation, errors: &mut Vec<String>) {
        for frame in &animation.frames {
            if frame.ms == 0 {
                errors.push(format!(
                    "animation {:?} frame ms must be >= 1",
                    animation.name
                ));
            }
            if !self.sprites.iter().any(|s| s.id == frame.sprite_id) {
                errors.push(format!(
                    "animation {:?} references missing sprite id {}",
                    animation.name, frame.sprite_id
                ));
            }
        }
    }
    pub fn validate_bounds(&self, width: u32, height: u32) -> Result<(), Vec<String>> {
        self.validate_bounds_internal(width, height, true)
    }
    pub fn validate_edit_bounds(&self, width: u32, height: u32) -> Result<(), Vec<String>> {
        self.validate_bounds_internal(width, height, false)
    }
    fn validate_bounds_internal(
        &self,
        width: u32,
        height: u32,
        complete: bool,
    ) -> Result<(), Vec<String>> {
        let mut errors = self.validate_internal(complete).err().unwrap_or_default();
        for tile in &self.tiles {
            self.tile_bounds_errors(tile, width, height, &mut errors);
        }
        for sprite in &self.sprites {
            self.sprite_bounds_errors(sprite, width, height, &mut errors);
        }
        into_result(errors)
    }
}
fn into_result(errors: Vec<String>) -> Result<(), Vec<String>> {
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
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
            ("version", serde_json::json!(4)),
            ("unknown", serde_json::json!(true)),
        ];
        for (key, value) in mutations {
            let mut v = fixture();
            v[key] = value;
            assert!(store::parse(&v.to_string()).is_err());
        }
        for pointer in [
            "/grid",
            "/sprites/0",
            "/animations/0",
            "/animations/0/frames/0",
        ] {
            let mut v = fixture();
            v.pointer_mut(pointer).unwrap()["unknown"] = true.into();
            assert!(store::parse(&v.to_string()).is_err(), "{pointer}");
        }
        let mut v = fixture();
        v["animations"][0]["frames"][0]["ms"] = 0.into();
        assert!(store::parse(&v.to_string()).is_err());
        let mut v = fixture();
        v["animations"][0]["frames"] = serde_json::json!([]);
        assert!(store::parse(&v.to_string()).is_err());
        let mut v = fixture();
        v["sprites"][1]["name"] = "a".into();
        // Without animations only the duplicate name can reject this.
        v["animations"] = serde_json::json!([]);
        assert!(store::parse(&v.to_string()).is_err());
        let mut v = fixture();
        let clip = v["animations"][0].clone();
        v["animations"].as_array_mut().unwrap().push(clip);
        assert!(store::parse(&v.to_string()).is_err());
        let mut v = fixture();
        v["animations"][0]["frames"][0]["sprite_id"] = 999.into();
        assert!(store::parse(&v.to_string()).is_err());
        let tile = serde_json::json!({"col":0,"row":0,"tags":[],"props":{}});
        let mut v = fixture();
        v["tiles"] = serde_json::json!([tile.clone(), tile.clone()]);
        assert!(store::parse(&v.to_string()).is_err());
        let mut v = fixture();
        v["tiles"] = serde_json::json!([tile]);
        v["tiles"][0]["props"]["number"] = 1.into();
        assert!(store::parse(&v.to_string()).is_err());
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
            id: 1,
            props: Default::default(),
            modified: None,
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
            id: 2,
            props: Default::default(),
            modified: None,
            name: "set".into(),
            pivot: [0.0, 0.0],
            shape: Shape::Set {
                canvas: [3, 2],
                parts: vec![
                    Part {
                        col: 0,
                        row: 0,
                        dx: 0,
                        dy: 0,
                        ..Default::default()
                    },
                    Part {
                        col: 1,
                        row: 0,
                        dx: 32,
                        dy: 16,
                        ..Default::default()
                    },
                ],
            },
        };
        assert_eq!(set.bounds(&sheet.grid), (48, 32));
        let mut sheet = SpriteSheet::new_for("x.png", Grid::DEFAULT);
        for (col, row) in [(16, 0), (0, 9)] {
            sheet.tiles = vec![TileMeta {
                col,
                row,
                name: None,
                tags: vec![],
                props: Default::default(),
            }];
            assert!(sheet.validate_bounds(256, 144).is_err());
        }
        sheet.tiles.clear();
        sheet.next_id = 3;
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
    fn legacy_reads_always_write_v3_and_transform_fields_are_strict() {
        let mut value: serde_json::Value =
            serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/example-v2.sprites.json")))
                .unwrap();
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

#[cfg(test)]
mod direct_read_tests {
    use super::*;

    #[test]
    fn direct_part_and_shape_reads_reject_invalid_transforms() {
        let base = serde_json::json!({"col":0,"row":0,"dx":0,"dy":0});
        for (field, invalid) in [
            ("rotation", serde_json::json!(45)),
            ("rotation", serde_json::json!(-90)),
            ("rotation", serde_json::json!("90")),
            ("rotation", serde_json::Value::Null),
            ("flip_x", serde_json::Value::Null),
            ("unknown", serde_json::json!(true)),
        ] {
            let mut part = base.clone();
            part[field] = invalid;
            assert!(
                serde_json::from_value::<Part>(part.clone()).is_err(),
                "Part: {part}"
            );
            let shape = serde_json::json!({"kind":"set","canvas":[1,1],"parts":[part]});
            assert!(
                serde_json::from_value::<Shape>(shape.clone()).is_err(),
                "Shape: {shape}"
            );
        }
        for transform in Transform::ALL {
            let mut part = base.clone();
            part["flip_x"] = transform.flip_x().into();
            part["rotation"] = transform.rotation().into();
            let decoded: Part = serde_json::from_value(part.clone()).unwrap();
            assert_eq!(decoded.transform(), transform);
            let Shape::Set { parts, .. } = serde_json::from_value::<Shape>(
                serde_json::json!({"kind":"set","canvas":[1,1],"parts":[part]}),
            )
            .unwrap() else {
                panic!("expected set")
            };
            assert_eq!(parts[0].transform(), transform);
        }
        assert_eq!(
            serde_json::from_value::<Part>(base).unwrap().transform(),
            Transform::IDENTITY
        );
    }

    #[test]
    fn direct_sheet_read_rejects_unsupported_version_before_validation() {
        let mut value: serde_json::Value =
            serde_json::from_str(super::super::example::EXAMPLE_JSON).unwrap();
        value["version"] = 4.into();
        assert!(serde_json::from_value::<SpriteSheet>(value).is_err());
    }

    #[test]
    fn in_memory_sheet_validation_rejects_unsupported_version_without_deserializing() {
        let (_, mut sheet) = super::super::example();
        sheet.version = 4;
        assert!(sheet.validate().is_err());
    }
}

#[cfg(test)]
mod v3_contract_tests {
    use super::super::{example, store};
    use super::*;
    fn v3() -> serde_json::Value {
        serde_json::from_str(example::EXAMPLE_JSON).unwrap()
    }
    fn v2() -> serde_json::Value {
        serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/example-v2.sprites.json"))).unwrap()
    }
    #[test]
    fn legacy_ids_are_deterministic_and_v2_is_golden() {
        let old = v2();
        let parsed = store::parse(&old.to_string()).unwrap();
        assert_eq!(store::to_json(&parsed), example::EXAMPLE_JSON);
        assert_eq!(
            parsed.sprites.iter().map(|s| s.id).collect::<Vec<_>>(),
            vec![1, 2, 3, 4]
        );
        assert_eq!(parsed.animations[0].id, 5);
        assert_eq!(parsed.next_id, 6);
        assert_eq!(
            parsed.animations[0]
                .frames
                .iter()
                .map(|f| f.sprite_id)
                .collect::<Vec<_>>(),
            vec![1, 2, 3, 4]
        );
        let mut reordered = old;
        reordered["sprites"].as_array_mut().unwrap().reverse();
        let reordered = store::parse(&reordered.to_string()).unwrap();
        assert_eq!(store::to_json(&reordered), example::EXAMPLE_JSON);
        let mut v1 = v2();
        v1["version"] = 1.into();
        assert_eq!(
            store::to_json(&store::parse(&v1.to_string()).unwrap()),
            example::EXAMPLE_JSON
        );
    }
    #[test]
    fn strict_version_fields_and_ids() {
        let mut cases = Vec::new();
        let mut x = v3();
        x["clips"] = x["animations"].clone();
        cases.push(x);
        let mut x = v2();
        x["animations"] = x["clips"].clone();
        cases.push(x);
        for key in ["id", "props", "modified"] {
            let mut x = v2();
            x["sprites"][0][key] = if key == "props" {
                serde_json::json!({})
            } else {
                1.into()
            };
            cases.push(x);
        }
        for key in ["id", "modified"] {
            let mut x = v2();
            x["clips"][0][key] = 1.into();
            cases.push(x);
        }
        let mut x = v2();
        x["next_id"] = 6.into();
        cases.push(x);
        let mut x = v2();
        x["clips"][0]["frames"][0]["sprite_id"] = 1.into();
        cases.push(x);
        let mut x = v3();
        x["tiles"] = serde_json::json!([{"col":0,"row":0,"tags":[],"props":{},"animation":{"mode":"loop","frames":[]}}]);
        cases.push(x);
        for key in ["id", "props"] {
            let mut x = v3();
            x["sprites"][0].as_object_mut().unwrap().remove(key);
            cases.push(x);
        }
        let mut x = v3();
        x["animations"][0].as_object_mut().unwrap().remove("id");
        cases.push(x);
        let mut x = v3();
        x.as_object_mut().unwrap().remove("next_id");
        cases.push(x);
        // Accepting this would let the next allocate_id overflow and panic.
        let mut x = v3();
        x["next_id"] = u64::MAX.into();
        cases.push(x);
        let mut x = v3();
        x["animations"][0]["frames"][0]
            .as_object_mut()
            .unwrap()
            .remove("sprite_id");
        x["animations"][0]["frames"][0]["sprite"] = "a".into();
        cases.push(x);
        for value in [
            serde_json::json!(-1),
            serde_json::json!("now"),
            serde_json::Value::Null,
        ] {
            let mut x = v3();
            x["sprites"][0]["modified"] = value;
            cases.push(x);
        }
        // An extra sprite no frame points at, so only the id rules can reject it.
        for id in [0, 2, 5, 6, 999] {
            let mut x = v3();
            let mut extra = x["sprites"][0].clone();
            extra["id"] = id.into();
            extra["name"] = "extra".into();
            x["sprites"].as_array_mut().unwrap().push(extra);
            cases.push(x);
        }
        for id in [0, 5, 999] {
            let mut x = v3();
            x["animations"][0]["frames"][0]["sprite_id"] = id.into();
            cases.push(x);
        }
        let mut x = v3();
        x["version"] = 4.into();
        cases.push(x);
        for case in cases {
            assert!(store::parse(&case.to_string()).is_err(), "{case}");
        }
    }
    #[test]
    fn canvas_legacy_conversion_and_canonical_ids() {
        let mut old = v2();
        old["sprites"][0] = serde_json::json!({"name":"a","kind":"set","pivot":[7,9],"parts":[{"col":0,"row":0,"dx":-16,"dy":0},{"col":1,"row":0,"dx":16,"dy":0}]});
        let sheet = store::parse(&old.to_string()).unwrap();
        let Shape::Set { canvas, parts } = &sheet.sprites[0].shape else {
            panic!()
        };
        assert_eq!(*canvas, [3, 1]);
        assert_eq!((parts[0].dx, parts[1].dx), (0, 32));
        assert_eq!(sheet.sprites[0].pivot, [7.0, 9.0]);
        let mut sheet = sheet;
        let Shape::Set { canvas, .. } = &mut sheet.sprites[0].shape else {
            panic!()
        };
        *canvas = [3, 2];
        sheet.sprites[0].props.insert("k".into(), "v".into());
        sheet.sprites[0].modified = Some(123);
        let canonical = store::to_json(&sheet);
        let read = store::parse(&canonical).unwrap();
        assert_eq!(store::to_json(&read), canonical);
        assert_eq!(read.sprites[0].bounds(&read.grid), (48, 32));
        let mut x: serde_json::Value = serde_json::from_str(&canonical).unwrap();
        x["sprites"][0]["canvas"] = serde_json::json!([0, 2]);
        assert!(store::parse(&x.to_string()).is_err());
        x["sprites"][0]["canvas"] = serde_json::json!([3, 2]);
        x["sprites"][0]["parts"][0]["dx"] = (-1).into();
        assert!(store::parse(&x.to_string()).is_err());
        let mut x = v3();
        x["sprites"][0]["canvas"] = serde_json::json!([1, 1]);
        assert!(store::parse(&x.to_string()).is_err());
        let mut x = old;
        x["sprites"][0]["canvas"] = serde_json::json!([3, 1]);
        assert!(store::parse(&x.to_string()).is_err());
        let mut sheet = read;
        let original = serde_json::to_string(&sheet.animations).unwrap();
        let order_before = sheet.sprites.iter().map(|s| s.id).collect::<Vec<_>>();
        sheet.sprites[0].name = "renamed".into();
        assert_eq!(serde_json::to_string(&sheet.animations).unwrap(), original);
        sheet.normalize();
        assert_eq!(
            sheet.sprites.iter().map(|s| s.id).collect::<Vec<_>>(),
            order_before
        );
        sheet.animations.clear();
        sheet.sprites.pop();
        let new_id = sheet.allocate_id();
        assert_eq!(new_id, 6);
    }
    #[test]
    fn legacy_tile_animations_are_dropped_and_reported() {
        let text = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/tile-animation-v2.sprites.json"));
        let (sheet, cells) = decode(text).unwrap();
        sheet.validate_bounds(64, 16).unwrap();
        assert_eq!(cells, vec![(1, 0), (2, 0)]);
        assert_eq!(sheet.tiles.len(), 1);
        assert_eq!(sheet.tiles[0].name.as_deref(), Some("torch"));
        let warning = store::dropped_tile_animation_warning(&cells).unwrap();
        assert!(warning.contains("(1, 0)") && warning.contains("(2, 0)"));
        let mut bad: serde_json::Value = serde_json::from_str(text).unwrap();
        bad["tiles"][0]["animation"]["frames"][0]["extra"] = true.into();
        assert!(store::parse(&bad.to_string()).is_err());
    }
}
