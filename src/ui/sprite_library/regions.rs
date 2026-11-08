use super::{
    Action, OpenSheet, SheetSource, SpriteLibrary, extract,
    view::{self, Overlay},
};
use crate::{
    raster::{self, Background, ObjectBox, RgbaImage},
    sprite::store,
};
use dear_imgui_rs::{TextureId, Ui};
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
};

thread_local! { static STATE: RefCell<RegionsState> = RefCell::new(RegionsState::default()); }
struct RegionsState {
    source: Option<TextureId>,
    background: usize,
    color: String,
    min_size: u32,
    candidates: Vec<ObjectBox>,
    selected: Option<usize>,
    save_path: String,
    confirm_crop: Option<(ObjectBox, String)>,
}
impl Default for RegionsState {
    fn default() -> Self {
        Self {
            source: None,
            background: 0,
            color: "ff00ff".into(),
            min_size: 32,
            candidates: vec![],
            selected: None,
            save_path: String::new(),
            confirm_crop: None,
        }
    }
}
impl RegionsState {
    fn sync(&mut self, sheet: &OpenSheet) {
        if self.source != Some(sheet.imgui_id) {
            self.source = Some(sheet.imgui_id);
            self.candidates.clear();
            self.selected = None;
            self.save_path.clear();
            self.confirm_crop = None;
        }
    }
    fn background(&self) -> Result<Background, String> {
        match self.background {
            0 => Ok(Background::Auto),
            1 => Ok(Background::Transparent),
            _ => raster::parse_hex_color(&self.color)
                .map(|[r, g, b]| Background::Color([r, g, b, 255])),
        }
    }
}
fn region_name(stem: &str, n: usize) -> String {
    format!("{stem}-region-{n}")
}
fn crop_action(image: &RgbaImage, bbox: ObjectBox, name: String) -> Result<Action, String> {
    let image = image
        .crop(bbox.x, bbox.y, bbox.w, bbox.h)
        .ok_or("region is outside the image")?;
    Ok(Action::Replace { image, name })
}
fn default_image_output(source: &SheetSource, checkout: &Path) -> PathBuf {
    let stem = extract::source_stem(source);
    checkout
        .join("assets/splites")
        .join(&stem)
        .join(format!("{stem}.png"))
}
fn save_memory(sheet: &mut OpenSheet, path: &Path, checkout: &Path) -> Result<String, String> {
    if !matches!(sheet.source, SheetSource::Memory { .. }) {
        return Err("only memory images need Save image as".into());
    }
    let output = extract::ensure_output_dir(path, checkout).map_err(|e| e.to_string())?;
    let name = output
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("image file name must be valid UTF-8")?
        .to_owned();
    store::write_atomic(&output, &sheet.image.to_png()?).map_err(|e| e.to_string())?;
    sheet.meta.image = name;
    sheet.source = SheetSource::File(output.clone());
    sheet.dirty = true;
    Ok(format!(
        "Saved image {}; Save writes its metadata{}",
        output.display(),
        extract::path_note(&output, checkout)
    ))
}

pub(super) fn memory_header(library: &mut SpriteLibrary, ui: &Ui) {
    let Some(sheet) = &mut library.sheet else {
        return;
    };
    if !matches!(sheet.source, SheetSource::Memory { .. }) {
        return;
    }
    let checkout = match std::env::current_dir() {
        Ok(path) => path,
        Err(e) => {
            library.status = e.to_string();
            return;
        }
    };
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.sync(sheet);
        if state.save_path.is_empty() {
            state.save_path = default_image_output(&sheet.source, &checkout)
                .to_string_lossy()
                .into_owned();
        }
        ui.input_text("Image output", &mut state.save_path).build();
        if ui.button("Save image as") {
            match save_memory(sheet, Path::new(&state.save_path), &checkout) {
                Ok(message) => {
                    if let SheetSource::File(path) = &sheet.source {
                        library.path_input = path.to_string_lossy().into_owned();
                    }
                    library.status = message;
                }
                Err(error) => library.status = error,
            }
        }
    });
}
pub(super) fn sheet_view(library: &mut SpriteLibrary, ui: &Ui) {
    let overlays = STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(sheet) = &library.sheet else {
            return vec![];
        };
        state.sync(sheet);
        state
            .selected
            .and_then(|i| state.candidates.get(i))
            .and_then(|bbox| bbox.snap(&sheet.meta.grid, sheet.image.width, sheet.image.height))
            .map(|rect| {
                vec![Overlay {
                    col: rect.col,
                    row: rect.row,
                    cols: rect.cols,
                    rows: rect.rows,
                    color: [1.0, 0.75, 0.1, 1.0],
                }]
            })
            .unwrap_or_default()
    });
    library.sheet_view(ui, &overlays);
}
pub(super) fn tab(library: &mut SpriteLibrary, ui: &Ui) {
    let Some(_tab) = ui.tab_item("Regions") else {
        return;
    };
    let Some(sheet) = &library.sheet else { return };
    STATE.with(|state| {
        let mut state=state.borrow_mut();state.sync(sheet);
        ui.combo_simple_string("Background",&mut state.background,&["Auto","Transparent","Color"]);
        if state.background==2 {ui.input_text("Background RGB hex",&mut state.color).build();}
        view::input_u32(ui,"Min size",&mut state.min_size,1);
        if ui.button("Find regions") {
            match state.background() {
                Ok(background)=>{
                    // Large images pause the editor; invoke only on this button.
                    state.candidates=raster::find_objects(&sheet.image,background,state.min_size);
                    state.selected=None;state.confirm_crop=None;
                    library.status=format!("{} region candidates (not saved)",state.candidates.len());
                },
                Err(e)=>library.status=e,
            }
        }
        ui.child_window("Region candidates").size([0.0,80.0]).build(ui,|| {
            for (i,bbox) in state.candidates.clone().iter().enumerate() {
                if ui.selectable(format!("#{}  {},{}  {}x{}",i+1,bbox.x,bbox.y,bbox.w,bbox.h)) {state.selected=Some(i);}
            }
        });
        let selected=state.selected.and_then(|i|state.candidates.get(i).copied().map(|b|(i,b)));
        {let _disabled=ui.begin_disabled_with_cond(selected.is_none());
            if ui.button("Crop to new image") && let Some((i,bbox))=selected {
                let name=region_name(&extract::source_stem(&sheet.source),i+1);
                if sheet.dirty {
                    state.confirm_crop=Some((bbox,name));ui.open_popup("Discard metadata for crop?");
                } else {
                    match crop_action(&sheet.image,bbox,name) {Ok(action)=>library.pending.push(action),Err(e)=>library.status=e}
                }
            }
        }
        ui.modal_popup("Discard metadata for crop?",|| {
            ui.text("Discard unsaved changes and start a new image?");
            if ui.button("Discard and crop") {
                if let Some((bbox,name))=state.confirm_crop.take() {
                    match crop_action(&sheet.image,bbox,name) {Ok(action)=>library.pending.push(action),Err(e)=>library.status=e}
                }
                ui.close_current_popup();
            }
            ui.same_line();
            if ui.button("Cancel crop") {state.confirm_crop=None;ui.close_current_popup();}
        });
        ui.text_wrapped("Crops keep the grid and start empty metadata. Save the image, then Load with Key color to remove a panel background.");
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn regions_are_proposals_and_crop_is_a_queued_image() {
        let mut image = RgbaImage::new(200, 160);
        image.fill_rect(0, 0, 200, 160, [255, 190, 220, 255]);
        image.fill_rect(3, 5, 40, 70, [255, 0, 255, 255]);
        image.fill_rect(100, 80, 80, 70, [255, 0, 255, 255]);
        image.set_pixel(10, 10, [1, 2, 3, 255]);
        let candidates = raster::find_objects(&image, Background::Auto, 32);
        assert_eq!(
            candidates,
            vec![
                ObjectBox {
                    x: 3,
                    y: 5,
                    w: 40,
                    h: 70
                },
                ObjectBox {
                    x: 100,
                    y: 80,
                    w: 80,
                    h: 70
                }
            ]
        );
        let action = crop_action(&image, candidates[0], region_name("panel", 1)).unwrap();
        let Action::Replace { image: crop, name } = action else {
            panic!("expected queued replacement")
        };
        assert_eq!((crop.width, crop.height), (40, 70));
        assert_eq!(name, "panel-region-1");
        assert_eq!(crop.pixel(7, 5), [1, 2, 3, 255]);
        assert!(
            crop_action(
                &image,
                ObjectBox {
                    x: 199,
                    y: 0,
                    w: 2,
                    h: 2
                },
                "bad".into()
            )
            .is_err()
        );
    }
    #[test]
    fn save_memory_uses_png_and_changes_source_only_after_success() {
        let temp = super::super::extract::tests::Temp::new();
        let mut sheet = super::super::extract::tests::memory_sheet();
        let output = default_image_output(&sheet.source, &temp.0);
        assert_eq!(output, temp.0.join("assets/splites/map/map.png"));
        assert!(save_memory(&mut sheet, &output.with_extension("jpg"), &temp.0).is_err());
        assert!(matches!(sheet.source, SheetSource::Memory { .. }));
        save_memory(&mut sheet, &output, &temp.0).unwrap();
        assert!(matches!(&sheet.source,SheetSource::File(path) if path==&output));
        assert_eq!(sheet.meta.image, "map.png");
        assert!(sheet.dirty);
        assert_eq!(
            RgbaImage::from_png(&std::fs::read(&output).unwrap()).unwrap(),
            sheet.image
        );
        assert!(!store::sidecar_for(&output).exists());
    }
}
