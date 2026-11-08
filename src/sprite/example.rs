use super::{
    format::{Clip, ClipFrame, Shape, Sprite, SpriteSheet},
    grid::Grid,
    store::{self, StoreError},
    timeline::Mode,
};
use crate::raster::RgbaImage;
use std::path::{Path, PathBuf};

pub const EXAMPLE_JSON: &str = include_str!("../../tests/fixtures/example.sprites.json");
pub fn example() -> (RgbaImage, SpriteSheet) {
    let mut image = RgbaImage::new(64, 16);
    let mut sheet = SpriteSheet::new_for(
        "example.png",
        Grid {
            tile: 16,
            offset: [0, 0],
            spacing: 0,
        },
    );
    for (col, (name, color)) in [
        ("a", [255, 64, 64, 255]),
        ("b", [64, 255, 64, 255]),
        ("c", [64, 64, 255, 255]),
        ("d", [255, 224, 64, 255]),
    ]
    .into_iter()
    .enumerate()
    {
        image.fill_rect(col as u32 * 16, 0, 16, 16, color);
        image.fill_rect(col as u32 * 16 + 6, 6, 4, 4, [0; 4]);
        sheet.sprites.push(Sprite {
            name: name.into(),
            shape: Shape::Rect {
                col: col as u32,
                row: 0,
                cols: 1,
                rows: 1,
            },
            pivot: [8.0, 8.0],
        });
    }
    sheet.clips.push(Clip {
        name: "cycle".into(),
        mode: Mode::Loop,
        tags: vec![],
        props: Default::default(),
        frames: ["a", "b", "c", "d"]
            .into_iter()
            .map(|name| ClipFrame {
                sprite: name.into(),
                ms: 150,
            })
            .collect(),
    });
    (image, sheet)
}
pub fn write_example(dir: &Path) -> Result<PathBuf, StoreError> {
    let (image, _) = example();
    let bytes = image.to_png().map_err(|e| StoreError::Invalid(vec![e]))?;
    store::write_atomic(&dir.join("example.png"), &bytes)?;
    let sidecar = dir.join("example.sprites.json");
    store::write_atomic(&sidecar, EXAMPLE_JSON.as_bytes())?;
    Ok(sidecar)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn code_generated_fixtures_are_current() {
        let (image, sheet) = example();
        assert_eq!(store::to_json(&sheet), EXAMPLE_JSON);
        assert_eq!(
            image.to_png().unwrap(),
            include_bytes!("../../tests/fixtures/example.png")
        );
        assert_eq!(store::parse(EXAMPLE_JSON).unwrap(), sheet);
        sheet.validate_bounds(64, 16).unwrap();
        for col in 0..4 {
            let pixel = image.pixel(col * 16, 0);
            assert_ne!(pixel[..3], [0; 3]);
            assert_eq!(pixel[3], 255);
            assert_eq!(image.pixel(col * 16 + 6, 6), [0; 4]);
        }
    }
}
