use super::SpriteSheet;
use crate::raster::RgbaImage;

/// Only sheets drawn in code belong here: never embed the person's local images.
/// scripts/smoke-web.sh --play uses the same example name.
pub fn lookup(name: &str) -> Option<(RgbaImage, SpriteSheet)> {
    match name {
        "example" => Some(super::example()),
        _ => None,
    }
}
