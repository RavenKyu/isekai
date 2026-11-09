use super::{
    format::{Clip, ClipFrame, Part, Shape, Sprite, SpriteSheet},
    grid::Grid,
    store::{self, StoreError},
    timeline::Mode,
};
use crate::raster::RgbaImage;
use std::path::{Path, PathBuf};

pub const EXAMPLE_JSON: &str = include_str!("../../tests/fixtures/example.sprites.json");
pub fn example() -> (RgbaImage, SpriteSheet) {
    let mut image = RgbaImage::new(64, 16);
    let mut sheet = SpriteSheet::new_for("example.png", Grid::DEFAULT);
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
pub const EXAMPLE_TURNED_JSON: &str =
    include_str!("../../tests/fixtures/example-turned.sprites.json");
pub fn example_turned() -> (RgbaImage, SpriteSheet) {
    let (image, mut sheet) = example();
    sheet.sprites = vec![Sprite {
        name: "turned".into(),
        pivot: [16.0, 16.0],
        shape: Shape::Set {
            parts: [(false, 0), (false, 90), (true, 0), (true, 270)]
                .into_iter()
                .enumerate()
                .map(|(i, (flip_x, rotation))| Part {
                    col: i as u32,
                    row: 0,
                    dx: (i % 2) as i32 * 16,
                    dy: (i / 2) as i32 * 16,
                    flip_x,
                    rotation,
                })
                .collect(),
        },
    }];
    sheet.clips = vec![Clip {
        name: "turned".into(),
        mode: Mode::Loop,
        tags: vec![],
        props: Default::default(),
        frames: vec![ClipFrame {
            sprite: "turned".into(),
            ms: 150,
        }],
    }];
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
    fn turned_fixture_is_current() {
        let (image, sheet) = example_turned();
        assert_eq!(store::to_json(&sheet), EXAMPLE_TURNED_JSON);
        assert_eq!(store::parse(EXAMPLE_TURNED_JSON).unwrap(), sheet);
        assert_eq!(image, example().0);
        sheet.validate_bounds(image.width, image.height).unwrap();
    }
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
