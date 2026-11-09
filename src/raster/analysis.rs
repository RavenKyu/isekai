use super::RgbaImage;
use crate::sprite::grid::Grid;
use std::{
    collections::{HashMap, HashSet},
    sync::atomic::{AtomicBool, Ordering},
};

pub const REPEAT_RATIO: f64 = 0.25;
pub const DEFAULT_GUESS_CANDIDATES: [u32; 7] = [8, 12, 16, 24, 32, 48, 64];
pub const DEFAULT_MIN_OBJECT_SIZE: u32 = 4;
pub const DEFAULT_SHEET_COLUMNS: u32 = 16;

impl RgbaImage {
    pub fn normalize_alpha(&mut self) {
        for pixel in self.pixels.as_chunks_mut::<4>().0 {
            if pixel[3] == 0 {
                *pixel = [0; 4];
            }
        }
    }

    /// Keying makes matching pixels empty for tile_is_empty, regardless of their old alpha.
    pub fn apply_key_color(&mut self, key: [u8; 3]) {
        for pixel in self.pixels.as_chunks_mut::<4>().0 {
            if pixel[..3] == key {
                *pixel = [0; 4];
            }
        }
    }

    pub fn tile(&self, grid: &Grid, col: u32, row: u32) -> Option<Self> {
        let cells = grid.cells(self.width, self.height);
        if col >= cells.cols || row >= cells.rows {
            return None;
        }
        let (x, y) = grid.origin(col, row);
        self.crop(x, y, grid.tile, grid.tile)
    }

    pub fn tile_is_empty(&self, grid: &Grid, col: u32, row: u32) -> bool {
        self.tile(grid, col, row)
            .is_none_or(|tile| tile.pixels.as_chunks::<4>().0.iter().all(|p| p[3] == 0))
    }

    fn normalized_tile_bytes(&self, grid: &Grid, col: u32, row: u32) -> Vec<u8> {
        let mut tile = self.tile(grid, col, row).expect("cell inside grid");
        tile.normalize_alpha();
        tile.pixels
    }
}

pub fn parse_hex_color(text: &str) -> Result<[u8; 3], String> {
    let text = text.strip_prefix('#').unwrap_or(text);
    if text.len() != 6 || !text.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err("color must contain exactly six hexadecimal digits".into());
    }
    let value = u32::from_str_radix(text, 16).map_err(|e| e.to_string())?;
    Ok([(value >> 16) as u8, (value >> 8) as u8, value as u8])
}

#[derive(Clone, Debug, PartialEq)]
pub struct GridScore {
    pub tile: u32,
    pub offset: (u32, u32),
    pub unique: usize,
    pub total: usize,
    pub ratio: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GridGuess {
    pub pick: Option<(u32, (u32, u32))>,
    pub scores: Vec<GridScore>,
}

/// Number of offsets that contain at least two rows and columns of cells.
/// A partial edge skips offsets exactly as the scoring loop does.
pub fn guess_grid_work(image: &RgbaImage, candidates: &[u32], search_offset: bool) -> u64 {
    candidates.iter().fold(0u64, |total, &tile| {
        if tile == 0 {
            return total;
        }
        let limit = if search_offset { u64::from(tile) } else { 1 };
        let offsets = |size: u32| {
            (u64::from(size) + 1)
                .saturating_sub(2 * u64::from(tile))
                .min(limit)
        };
        total.saturating_add(offsets(image.width) * offsets(image.height))
    })
}

/// Expensive suggestion: invoke from an explicit editor button, never on every frame.
/// Full normalized byte keys preserve equality even when hash values collide.
pub fn guess_grid(image: &RgbaImage, candidates: &[u32], search_offset: bool) -> GridGuess {
    guess_grid_with(
        image,
        candidates,
        search_offset,
        &|_, _| {},
        &AtomicBool::new(false),
    )
    .expect("uncancelled grid guess")
}

/// The synchronous algorithm with progress after each scored offset. Cancellation
/// is checked between cells as well, so a large offset cannot hold up cancellation.
/// Cancelled work returns no partial suggestion.
pub fn guess_grid_with(
    image: &RgbaImage,
    candidates: &[u32],
    search_offset: bool,
    progress: &dyn Fn(u64, u64),
    cancel: &AtomicBool,
) -> Option<GridGuess> {
    let total_work = guess_grid_work(image, candidates, search_offset);
    let mut done = 0;
    progress(done, total_work);
    if cancel.load(Ordering::Relaxed) {
        return None;
    }
    let mut scores = Vec::new();
    for &tile in candidates {
        let bounds = Grid {
            tile,
            offset: [0, 0],
            spacing: 0,
        }
        .cells(image.width, image.height);
        if bounds.cols < 2 || bounds.rows < 2 {
            continue;
        }
        let mut best: Option<GridScore> = None;
        let limit = if search_offset { tile } else { 1 };
        for oy in 0..limit {
            for ox in 0..limit {
                if cancel.load(Ordering::Relaxed) {
                    return None;
                }
                let grid = Grid {
                    tile,
                    offset: [ox, oy],
                    spacing: 0,
                };
                let cells = grid.cells(image.width, image.height);
                if cells.cols < 2 || cells.rows < 2 {
                    continue;
                }
                let mut unique = HashSet::new();
                for row in 0..cells.rows {
                    for col in 0..cells.cols {
                        if cancel.load(Ordering::Relaxed) {
                            return None;
                        }
                        unique.insert(image.normalized_tile_bytes(&grid, col, row));
                    }
                }
                let total = cells.cols as usize * cells.rows as usize;
                let score = GridScore {
                    tile,
                    offset: (ox, oy),
                    unique: unique.len(),
                    total,
                    ratio: unique.len() as f64 / total as f64,
                };
                if best.as_ref().is_none_or(|old| score.ratio < old.ratio) {
                    best = Some(score);
                }
                done += 1;
                progress(done, total_work);
            }
        }
        if let Some(score) = best {
            scores.push(score);
        }
    }
    let pick = scores
        .iter()
        .filter(|s| s.ratio <= REPEAT_RATIO)
        .max_by_key(|s| s.tile)
        .or_else(|| {
            scores
                .iter()
                .min_by_key(|s| (s.unique as u128 * u128::from(s.tile).pow(2), s.tile))
        })
        .map(|s| (s.tile, s.offset));
    if cancel.load(Ordering::Relaxed) {
        None
    } else {
        Some(GridGuess { pick, scores })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dedupe {
    pub unique: Vec<RgbaImage>,
    pub map: Vec<usize>,
    pub cols: u32,
    pub rows: u32,
    pub counts: Vec<usize>,
}

pub fn dedupe(image: &RgbaImage, grid: &Grid) -> Dedupe {
    let cells = grid.cells(image.width, image.height);
    let mut result = Dedupe {
        unique: Vec::new(),
        map: Vec::new(),
        cols: cells.cols,
        rows: cells.rows,
        counts: Vec::new(),
    };
    let mut indices = HashMap::<Vec<u8>, usize>::new();
    for row in 0..cells.rows {
        for col in 0..cells.cols {
            let bytes = image.normalized_tile_bytes(grid, col, row);
            let next = result.unique.len();
            let index = *indices.entry(bytes.clone()).or_insert_with(|| {
                result.unique.push(RgbaImage {
                    width: grid.tile,
                    height: grid.tile,
                    pixels: bytes,
                });
                result.counts.push(0);
                next
            });
            result.map.push(index);
            result.counts[index] += 1;
        }
    }
    result
}

pub fn build_sheet(unique: &[RgbaImage], tile: u32, columns: u32) -> RgbaImage {
    assert!(tile > 0, "sheet tile must be positive");
    let count = u32::try_from(unique.len()).expect("too many unique tiles");
    let cols = columns.min(count).max(1);
    let rows = count.div_ceil(cols).max(1);
    let mut sheet = RgbaImage::new(
        cols.checked_mul(tile).expect("sheet width overflow"),
        rows.checked_mul(tile).expect("sheet height overflow"),
    );
    for (index, image) in unique.iter().enumerate() {
        assert_eq!((image.width, image.height), (tile, tile));
        let (x, y) = (index as u32 % cols * tile, index as u32 / cols * tile);
        for dy in 0..tile {
            for dx in 0..tile {
                sheet.set_pixel(x + dx, y + dy, image.pixel(dx, dy));
            }
        }
    }
    sheet
}

pub fn rebuild_matches(
    image: &RgbaImage,
    grid: &Grid,
    dedupe: &Dedupe,
) -> Result<(), Vec<(u32, u32)>> {
    let cells = grid.cells(image.width, image.height);
    let mut different = Vec::new();
    for row in 0..cells.rows {
        for col in 0..cells.cols {
            let index = row as usize * cells.cols as usize + col as usize;
            let expected = image.normalized_tile_bytes(grid, col, row);
            let actual = dedupe.map.get(index).and_then(|&i| dedupe.unique.get(i));
            if dedupe.cols != cells.cols
                || dedupe.rows != cells.rows
                || actual.is_none_or(|tile| {
                    tile.width != grid.tile || tile.height != grid.tile || tile.pixels != expected
                })
            {
                different.push((col, row));
            }
        }
    }
    if different.is_empty() {
        Ok(())
    } else {
        Err(different)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Background {
    Transparent,
    Color([u8; 4]),
    Auto,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectBox {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellRect {
    pub col: u32,
    pub row: u32,
    pub cols: u32,
    pub rows: u32,
}

/// Linear flood fill with an explicit stack; call on demand, not during every draw.
pub fn find_objects(image: &RgbaImage, background: Background, min_size: u32) -> Vec<ObjectBox> {
    let Some(len) = (image.width as usize).checked_mul(image.height as usize) else {
        return Vec::new();
    };
    if len == 0 {
        return Vec::new();
    }
    let background = match background {
        Background::Auto if image.pixels.as_chunks::<4>().0.iter().any(|p| p[3] == 0) => {
            Background::Transparent
        }
        Background::Auto => Background::Color(image.pixel(0, 0)),
        other => other,
    };
    let foreground = |x, y| match background {
        Background::Transparent => image.pixel(x, y)[3] > 0,
        Background::Color(color) => image.pixel(x, y) != color,
        Background::Auto => unreachable!(),
    };
    let mut visited = vec![false; len];
    let mut objects = Vec::new();
    let mut stack = Vec::new();
    for y in 0..image.height {
        for x in 0..image.width {
            let i = y as usize * image.width as usize + x as usize;
            if visited[i] {
                continue;
            }
            visited[i] = true;
            if !foreground(x, y) {
                continue;
            }
            let (mut left, mut top, mut right, mut bottom) = (x, y, x, y);
            stack.push((x, y));
            while let Some((px, py)) = stack.pop() {
                left = left.min(px);
                top = top.min(py);
                right = right.max(px);
                bottom = bottom.max(py);
                for ny in py.saturating_sub(1)..=py.saturating_add(1).min(image.height - 1) {
                    for nx in px.saturating_sub(1)..=px.saturating_add(1).min(image.width - 1) {
                        let ni = ny as usize * image.width as usize + nx as usize;
                        if !visited[ni] {
                            visited[ni] = true;
                            if foreground(nx, ny) {
                                stack.push((nx, ny));
                            }
                        }
                    }
                }
            }
            let object = ObjectBox {
                x: left,
                y: top,
                w: right - left + 1,
                h: bottom - top + 1,
            };
            if object.w >= min_size && object.h >= min_size {
                objects.push(object);
            }
        }
    }
    objects.sort_by_key(|b| (b.y, b.x));
    objects
}

impl ObjectBox {
    pub fn snap(&self, grid: &Grid, width: u32, height: u32) -> Option<CellRect> {
        grid.validate().ok()?;
        if self.w == 0 || self.h == 0 {
            return None;
        }
        let cells = grid.cells(width, height);
        // Look up actual cell origins: no second copy of Grid's stride/offset formula.
        // A start in a gap selects the next cell; an end in a gap selects the previous.
        let axis = |start: u32, len: u32, count: u32, horizontal: bool| {
            let end = u64::from(start) + u64::from(len);
            // Keep axes independent even when the other axis has a nonzero offset.
            let origin = |i| {
                if horizontal {
                    grid.origin(i, 0).0
                } else {
                    grid.origin(0, i).1
                }
            };
            let first = (0..count)
                .find(|&i| u64::from(origin(i)) + u64::from(grid.tile) > u64::from(start))?;
            let last = (first..count)
                .take_while(|&i| u64::from(origin(i)) < end)
                .last()?;
            Some((first, last - first + 1))
        };
        let (col, cols) = axis(self.x, self.w, cells.cols, true)?;
        let (row, rows) = axis(self.y, self.h, cells.rows, false)?;
        Some(CellRect {
            col,
            row,
            cols,
            rows,
        })
    }
}

#[cfg(test)]
mod tests;
