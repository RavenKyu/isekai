use crate::{
    raster::RgbaImage,
    sprite::{Grid, SpriteSheet},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub cells: BTreeSet<(u32, u32)>,
    pub anchor: Option<(u32, u32)>,
}
impl Selection {
    pub fn apply(&mut self, cell: Option<(u32, u32)>, shift: bool, ctrl: bool) {
        let Some((col, row)) = cell else {
            self.cells.clear();
            self.anchor = None;
            return;
        };
        if shift {
            let (x, y) = self.anchor.unwrap_or((col, row));
            self.cells.clear();
            for r in y.min(row)..=y.max(row) {
                for c in x.min(col)..=x.max(col) {
                    self.cells.insert((c, r));
                }
            }
        } else if ctrl {
            if !self.cells.remove(&(col, row)) {
                self.cells.insert((col, row));
            }
            self.anchor = Some((col, row));
        } else {
            self.cells.clear();
            self.cells.insert((col, row));
            self.anchor = Some((col, row));
        }
    }
    pub fn single(&self) -> Option<(u32, u32)> {
        if self.cells.len() == 1 {
            self.cells.first().copied()
        } else {
            None
        }
    }
}
pub fn cell_under(
    grid: &Grid,
    zoom: u32,
    origin: [f32; 2],
    mouse: [f32; 2],
    width: u32,
    height: u32,
) -> Option<(u32, u32)> {
    if zoom == 0 || mouse.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let x = (mouse[0] - origin[0]) / zoom as f32;
    let y = (mouse[1] - origin[1]) / zoom as f32;
    if x < 0.0 || y < 0.0 {
        return None;
    }
    grid.cell_at(x.floor() as u32, y.floor() as u32, width, height)
}
pub fn apply_guess(grid: Grid, pick: (u32, (u32, u32))) -> Grid {
    Grid {
        tile: pick.0,
        offset: [pick.1.0, pick.1.1],
        ..grid
    }
}
pub fn parse_tags(text: &str) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}
pub fn format_tags(tags: &[String]) -> String {
    tags.join(", ")
}
pub fn parse_props(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut props = BTreeMap::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("property line {} needs key=value", i + 1))?;
        let key = key.trim();
        if key.is_empty() || props.contains_key(key) {
            return Err(format!(
                "property line {} has an empty or duplicate key",
                i + 1
            ));
        }
        props.insert(key.into(), value.into());
    }
    Ok(props)
}
pub fn format_props(props: &BTreeMap<String, String>) -> String {
    props
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("\n")
}
pub fn list_assets(root: &Path) -> Result<Vec<PathBuf>, String> {
    fn visit(root: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
        if root.symlink_metadata()?.file_type().is_symlink() {
            return Ok(());
        }
        for entry in std::fs::read_dir(root)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_symlink() || entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            if kind.is_dir() {
                visit(&entry.path(), out)?;
            } else if kind.is_file() && is_png(&entry.path()) {
                out.push(entry.path());
            }
        }
        Ok(())
    }
    // A checkout without local assets (assets/splites is ignored) lists nothing.
    if matches!(root.symlink_metadata(), Err(e) if e.kind() == std::io::ErrorKind::NotFound) {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    visit(root, &mut files).map_err(|e| format!("assets {}: {e}", root.display()))?;
    files.sort();
    Ok(files)
}
pub fn is_png(path: &Path) -> bool {
    path.extension()
        .is_some_and(|s| s.eq_ignore_ascii_case("png"))
}

pub fn open_sheet(
    path: &Path,
    bytes: &[u8],
    key: Option<[u8; 3]>,
    sidecar: Option<SpriteSheet>,
) -> Result<(RgbaImage, SpriteSheet, bool), String> {
    if !is_png(path) {
        return Err("only PNG images are supported".into());
    }
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("image file name must be valid UTF-8")?;
    let mut image = RgbaImage::from_png(bytes)?;
    image.normalize_alpha();
    if let Some(key) = key {
        image.apply_key_color(key);
    }
    let dirty = sidecar.is_none();
    let meta = sidecar.unwrap_or_else(|| {
        SpriteSheet::new_for(
            name,
            Grid {
                tile: 16,
                offset: [0, 0],
                spacing: 0,
            },
        )
    });
    if meta.image != name {
        return Err(format!(
            "sidecar image {:?} does not match opened file {name:?}",
            meta.image
        ));
    }
    meta.validate_bounds(image.width, image.height)
        .map_err(|errors| errors.join("; "))?;
    Ok((image, meta, dirty))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn picking_selection_guess_and_text_roundtrips() {
        let grid = Grid {
            tile: 16,
            offset: [2, 5],
            spacing: 1,
        };
        assert_eq!(
            cell_under(&grid, 3, [0.0; 2], [57.0, 15.0], 100, 50),
            Some((1, 0))
        );
        assert_eq!(cell_under(&grid, 3, [0.0; 2], [54.0, 15.0], 100, 50), None);
        assert_eq!(cell_under(&grid, 3, [0.0; 2], [57.0, 12.0], 100, 50), None);
        let mut selection = Selection::default();
        selection.apply(Some((1, 2)), false, false);
        assert_eq!(selection.single(), Some((1, 2)));
        selection.apply(Some((2, 3)), true, false);
        assert_eq!(selection.cells.len(), 4);
        selection.apply(Some((1, 2)), false, true);
        assert_eq!(selection.cells.len(), 3);
        selection.apply(None, false, false);
        assert!(selection.cells.is_empty());
        assert_eq!(
            apply_guess(grid, (16, (3, 5))),
            Grid {
                tile: 16,
                offset: [3, 5],
                spacing: 1
            }
        );
        assert_eq!(parse_tags("a, b ,c"), vec!["a", "b", "c"]);
        assert_eq!(
            parse_tags(&format_tags(&parse_tags("a, b ,c"))),
            vec!["a", "b", "c"]
        );
        let props = parse_props("k=v\nx=1").unwrap();
        assert_eq!(format_props(&props), "k=v\nx=1");
        assert!(parse_props("broken").is_err());
        assert!(parse_props("x=1\nx=2").is_err());
    }
    #[test]
    fn import_requires_png_matching_valid_sidecar() {
        let (image, meta) = crate::sprite::example();
        let bytes = image.to_png().unwrap();
        assert!(open_sheet(Path::new("example.jpg"), &bytes, None, None).is_err());
        assert!(open_sheet(Path::new("example.png"), b"broken", None, None).is_err());
        assert!(crate::sprite::store::parse("broken sidecar").is_err());
        assert!(open_sheet(Path::new("other.png"), &bytes, None, Some(meta.clone())).is_err());
        let (_, new, dirty) = open_sheet(Path::new("example.PNG"), &bytes, None, None).unwrap();
        assert!(dirty);
        assert_eq!(new.image, "example.PNG");
        assert_eq!(new.grid.tile, 16);
        assert!(
            !open_sheet(Path::new("example.png"), &bytes, None, Some(meta))
                .unwrap()
                .2
        );
    }
    #[cfg(unix)]
    #[test]
    fn assets_skip_hidden_entries_and_symlinks() {
        let temp = crate::test_support::TempDir::new("assets");
        let root = &temp.0;
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("sub/a.PNG"), b"").unwrap();
        std::fs::write(root.join(".hidden.png"), b"").unwrap();
        std::os::unix::fs::symlink(root.join("sub"), root.join("loop")).unwrap();
        assert_eq!(list_assets(root).unwrap(), vec![root.join("sub/a.PNG")]);
        assert_eq!(
            list_assets(&root.join("missing")).unwrap(),
            Vec::<PathBuf>::new()
        );
    }
}
