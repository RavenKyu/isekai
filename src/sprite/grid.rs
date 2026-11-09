use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grid {
    pub tile: u32,
    pub offset: [u32; 2],
    pub spacing: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cells {
    pub cols: u32,
    pub rows: u32,
    pub leftover_right: u32,
    pub leftover_bottom: u32,
}

impl Grid {
    pub const DEFAULT: Self = Self {
        tile: 16,
        offset: [0, 0],
        spacing: 0,
    };
    pub fn validate(&self) -> Result<(), String> {
        if self.tile == 0 || self.tile.checked_add(self.spacing).is_none() {
            Err("grid tile must be positive and tile + spacing must fit u32".into())
        } else {
            Ok(())
        }
    }

    pub fn cells(&self, width: u32, height: u32) -> Cells {
        let axis = |size: u32, offset: u32| {
            let available = size.saturating_sub(offset);
            if self.validate().is_err() || offset > size {
                return (0, available);
            }
            let step = u64::from(self.tile) + u64::from(self.spacing);
            let count = ((u64::from(available) + u64::from(self.spacing)) / step) as u32;
            let used = u64::from(count) * u64::from(self.tile)
                + u64::from(count.saturating_sub(1)) * u64::from(self.spacing);
            (count, available - used as u32)
        };
        let (cols, leftover_right) = axis(width, self.offset[0]);
        let (rows, leftover_bottom) = axis(height, self.offset[1]);
        Cells {
            cols,
            rows,
            leftover_right,
            leftover_bottom,
        }
    }

    /// Callers pass a validated grid and a cell inside its image bounds.
    pub fn origin(&self, col: u32, row: u32) -> (u32, u32) {
        let step = u64::from(self.tile) + u64::from(self.spacing);
        (
            (u64::from(self.offset[0]) + u64::from(col) * step) as u32,
            (u64::from(self.offset[1]) + u64::from(row) * step) as u32,
        )
    }

    pub fn cell_at(&self, px: u32, py: u32, width: u32, height: u32) -> Option<(u32, u32)> {
        self.validate().ok()?;
        if px >= width || py >= height {
            return None;
        }
        let x = px.checked_sub(self.offset[0])?;
        let y = py.checked_sub(self.offset[1])?;
        let step = self.tile + self.spacing;
        let cells = self.cells(width, height);
        let (col, row) = (x / step, y / step);
        (col < cells.cols && row < cells.rows && x % step < self.tile && y % step < self.tile)
            .then_some((col, row))
    }

    pub fn uv(&self, col: u32, row: u32, width: u32, height: u32) -> ([f32; 2], [f32; 2]) {
        let (x, y) = self.origin(col, row);
        (
            [x as f32 / width as f32, y as f32 / height as f32],
            [
                (u64::from(x) + u64::from(self.tile)) as f32 / width as f32,
                (u64::from(y) + u64::from(self.tile)) as f32 / height as f32,
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dimensions_spacing_and_independent_offsets() {
        let grid = Grid::DEFAULT;
        assert_eq!(
            (grid.cells(256, 64).cols, grid.cells(256, 64).rows),
            (16, 4)
        );
        assert_eq!(
            (grid.cells(256, 144).cols, grid.cells(256, 144).rows),
            (16, 9)
        );
        let grid = Grid {
            offset: [2, 0],
            spacing: 1,
            ..grid
        };
        assert_eq!(
            grid.cells(100, 50),
            Cells {
                cols: 5,
                rows: 3,
                leftover_right: 14,
                leftover_bottom: 0
            }
        );
        assert_eq!(grid.cell_at(1, 0, 100, 50), None);
        assert_eq!(grid.cell_at(18, 0, 100, 50), None);
        assert_eq!(grid.cell_at(19, 0, 100, 50), Some((1, 0)));
        assert_eq!(grid.cell_at(86, 0, 100, 50), None);
        assert_eq!(grid.origin(1, 1), (19, 17));
        let grid = Grid {
            tile: 16,
            offset: [3, 5],
            spacing: 0,
        };
        assert_eq!((grid.cells(100, 50).cols, grid.cells(100, 50).rows), (6, 2));
        assert_eq!(grid.cell_at(3, 4, 100, 50), None);
        assert_eq!(grid.uv(0, 0, 100, 50), ([0.03, 0.1], [0.19, 0.42]));
        assert_eq!(grid.cells(1, 1).cols, 0);
        assert!(Grid { tile: 0, ..grid }.validate().is_err());
        assert!(
            Grid {
                tile: 1,
                spacing: u32::MAX,
                ..grid
            }
            .validate()
            .is_err()
        );
        assert_eq!(
            Grid {
                tile: 1,
                offset: [0, 0],
                spacing: 1
            }
            .cells(u32::MAX, 1)
            .cols,
            2147483648
        );
    }
}
