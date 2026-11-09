use super::log;
use super::{
    Action, OpenSheet, SheetEditor, SheetSource, extract,
    view::{self, Overlay},
};
use crate::{
    raster::{self, Background, ObjectBox, RgbaImage},
    sprite::{Grid, store},
};
use dear_imgui_rs::{TextureId, Ui};
use std::path::{Path, PathBuf};

pub(super) struct RegionsState {
    source: Option<TextureId>,
    background: usize,
    color: String,
    min_size: u32,
    candidates: Vec<ObjectBox>,
    selected: Option<usize>,
    save_path: String,
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
fn crop_action(
    image: &RgbaImage,
    grid: Grid,
    bbox: ObjectBox,
    name: String,
) -> Result<Action, String> {
    let image = image
        .crop(bbox.x, bbox.y, bbox.w, bbox.h)
        .ok_or("region is outside the image")?;
    Ok(Action::Replace { image, name, grid })
}
fn default_image_output(source: &SheetSource, checkout: &Path) -> PathBuf {
    let stem = extract::source_stem(source);
    checkout
        .join("assets/splites")
        .join(&stem)
        .join(format!("{stem}.png"))
}
fn save_memory(
    sheet: &mut OpenSheet,
    path: &Path,
    checkout: &Path,
    open_files: &[PathBuf],
) -> Result<String, String> {
    if !matches!(sheet.source, SheetSource::Memory { .. }) {
        return Err("only memory images need Save image as".into());
    }
    let output = extract::absolute_output(path, checkout).map_err(|e| e.to_string())?;
    extract::refuse_open_output(&output, open_files)?;
    // Save would then replace this sidecar with the crop's empty metadata.
    let sidecar = store::sidecar_for(&output);
    if match std::fs::symlink_metadata(&sidecar) {
        Ok(_) => true,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => return Err(e.to_string()),
    } {
        return Err(format!(
            "sidecar {} exists; choose another image path",
            sidecar.display()
        ));
    }
    let name = output
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("image file name must be valid UTF-8")?
        .to_owned();
    let output = extract::ensure_output_dir(&output, checkout).map_err(|e| e.to_string())?;
    store::publish_new(&output, &sheet.image.to_png()?).map_err(|e| match e {
        store::StoreError::Io(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            format!(
                "image {} exists; choose another image path",
                output.display()
            )
            .replace(['\n', '\r'], " ")
        }
        other => other.to_string(),
    })?;
    // Load compares canonical paths, so a later Load of this PNG focuses this editor.
    let output = output.canonicalize().unwrap_or(output);
    sheet.meta.image = name;
    sheet.source = SheetSource::File(output.clone());
    sheet.dirty = true;
    Ok(format!(
        "Saved image {}; Save writes its metadata{}",
        output.display(),
        extract::path_note(&output, checkout)
    ))
}

pub(super) fn memory_header(editor: &mut SheetEditor, ui: &Ui, open_files: &[PathBuf]) {
    let SheetEditor {
        sheet,
        regions: state,
        log,
        saved_path,
        ..
    } = editor;
    if !matches!(sheet.source, SheetSource::Memory { .. }) {
        return;
    }
    let checkout = match std::env::current_dir() {
        Ok(path) => path,
        Err(e) => {
            log::append(log, e.to_string());
            return;
        }
    };
    state.sync(sheet);
    if state.save_path.is_empty() {
        state.save_path = default_image_output(&sheet.source, &checkout)
            .to_string_lossy()
            .into_owned();
    }
    ui.input_text("Image output", &mut state.save_path).build();
    if ui.button("Save image as") {
        match save_memory(sheet, Path::new(&state.save_path), &checkout, open_files) {
            Ok(message) => {
                if let SheetSource::File(path) = &sheet.source {
                    *saved_path = Some(path.clone());
                }
                log::append(log, message);
            }
            Err(error) => log::append(log, error),
        }
    }
}
pub(super) fn sheet_view(editor: &mut SheetEditor, ui: &Ui, mut overlays: Vec<Overlay>) {
    let regions = {
        let state = &mut editor.regions;
        let sheet = &editor.sheet;
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
    };
    overlays.extend(regions);
    editor.sheet_view(ui, &overlays);
}
pub(super) fn tab(editor: &mut SheetEditor, ui: &Ui) {
    let Some(_tab) = ui.tab_item("Regions") else {
        return;
    };
    let SheetEditor {
        sheet,
        regions: state,
        log,
        pending,
        ..
    } = editor;
    state.sync(sheet);
    ui.combo_simple_string(
        "Background",
        &mut state.background,
        &["Auto", "Transparent", "Color"],
    );
    if state.background == 2 {
        ui.input_text("Background RGB hex", &mut state.color)
            .build();
    }
    view::input_u32(ui, "Min size", &mut state.min_size, 1);
    if ui.button("Find regions") {
        match state.background() {
            Ok(background) => {
                // Large images pause the editor; invoke only on this button.
                state.candidates = raster::find_objects(&sheet.image, background, state.min_size);
                state.selected = None;
                log::append(
                    log,
                    format!("{} region candidates (not saved)", state.candidates.len()),
                );
            }
            Err(e) => log::append(log, e),
        }
    }
    ui.child_window("Region candidates")
        .size([0.0, 80.0])
        .build(ui, || {
            for (i, bbox) in state.candidates.clone().iter().enumerate() {
                if ui.selectable(format!(
                    "#{}  {},{}  {}x{}",
                    i + 1,
                    bbox.x,
                    bbox.y,
                    bbox.w,
                    bbox.h
                )) {
                    state.selected = Some(i);
                }
            }
        });
    let selected = state
        .selected
        .and_then(|i| state.candidates.get(i).copied().map(|b| (i, b)));
    {
        let _disabled = ui.begin_disabled_with_cond(selected.is_none());
        if ui.button("Crop to new image")
            && let Some((i, bbox)) = selected
        {
            let name = region_name(&extract::source_stem(&sheet.source), i + 1);
            match crop_action(&sheet.image, sheet.meta.grid, bbox, name) {
                Ok(action) => pending.push(action),
                Err(e) => log::append(log, e),
            }
        }
    }
    ui.text_wrapped("Crops keep the grid and start empty metadata. Save the image, then turn on Key color and Reload image in its window to remove a panel background.");
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
        let grid = Grid {
            tile: 8,
            offset: [3, 5],
            spacing: 0,
        };
        let action = crop_action(&image, grid, candidates[0], region_name("panel", 1)).unwrap();
        let Action::Replace {
            image: crop,
            name,
            grid: kept,
        } = action
        else {
            panic!("expected queued replacement")
        };
        assert_eq!((crop.width, crop.height), (40, 70));
        assert_eq!(name, "panel-region-1");
        assert_eq!(kept, grid);
        assert_eq!(crop.pixel(7, 5), [1, 2, 3, 255]);
        assert!(
            crop_action(
                &image,
                grid,
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
    fn memory_save_button_in_its_own_process() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "ui::sprite_library::regions::tests::memory_save_button_preserves_state_on_refusal",
                "--ignored",
                "--nocapture",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
    }
    #[test]
    #[ignore = "run by memory_save_button_in_its_own_process"]
    fn memory_save_button_preserves_state_on_refusal() {
        use super::super::tests::{context, item_center, vertices};
        use std::cell::{Cell, RefCell};
        let temp = super::super::extract::tests::Temp::new();
        let path = temp.0.join("existing.png");
        std::fs::write(&path, b"existing PNG without metadata").unwrap();
        let mut editor = SheetEditor::new(1, super::super::extract::tests::memory_sheet());
        editor.sheet.dirty = false;
        editor.regions.source = Some(editor.sheet.imgui_id);
        editor.regions.save_path = path.to_string_lossy().into_owned();
        let meta = editor.sheet.meta.clone();
        let editor = RefCell::new(editor);
        let center = Cell::new([0.0; 2]);
        let (mut context, consumer) = context();
        let render = |context: &mut dear_imgui_rs::Context| {
            vertices(context, &consumer, |ui| {
                ui.window("Save memory test")
                    .size([600.0, 160.0], dear_imgui_rs::Condition::Always)
                    .build(|| {
                        memory_header(&mut editor.borrow_mut(), ui, &[]);
                        center.set(item_center(ui));
                    });
            });
        };
        render(&mut context);
        context.io_mut().add_mouse_pos_event(center.get());
        render(&mut context);
        context
            .io_mut()
            .add_mouse_button_event(dear_imgui_rs::MouseButton::Left, true);
        render(&mut context);
        context
            .io_mut()
            .add_mouse_button_event(dear_imgui_rs::MouseButton::Left, false);
        render(&mut context);
        let editor = editor.borrow();
        assert!(editor.saved_path.is_none());
        assert!(matches!(editor.sheet.source, SheetSource::Memory { .. }));
        assert!(!editor.sheet.dirty);
        assert_eq!(editor.sheet.meta, meta);
        assert_eq!(editor.log.len(), 1, "the button must actually have run");
        assert!(editor.log[0].contains("exists; choose another image path"));
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"existing PNG without metadata"
        );
        assert_eq!(std::fs::read_dir(&temp.0).unwrap().count(), 1);
    }
    #[cfg(unix)]
    #[test]
    fn save_memory_refuses_existing_symlink_names() {
        let temp = super::super::extract::tests::Temp::new();
        let outside = super::super::extract::tests::Temp::new();
        let target = outside.0.join("original.png");
        std::fs::write(&target, b"original PNG").unwrap();
        for (name, target) in [
            ("alias.png", target.clone()),
            ("dangling.png", outside.0.join("missing.png")),
        ] {
            let output = temp.0.join(name);
            std::os::unix::fs::symlink(&target, &output).unwrap();
            let mut sheet = super::super::extract::tests::memory_sheet();
            sheet.dirty = false;
            let meta = sheet.meta.clone();
            assert!(
                save_memory(&mut sheet, &output, &temp.0, &[])
                    .unwrap_err()
                    .contains("exists; choose another image path")
            );
            assert!(matches!(sheet.source, SheetSource::Memory { .. }));
            assert!(!sheet.dirty);
            assert_eq!(sheet.meta, meta);
            assert_eq!(std::fs::read_link(&output).unwrap(), target);
            assert!(!store::sidecar_for(&output).exists());
        }
        assert_eq!(std::fs::read(&target).unwrap(), b"original PNG");
        assert_eq!(std::fs::read_dir(&temp.0).unwrap().count(), 2);
        assert_eq!(std::fs::read_dir(&outside.0).unwrap().count(), 1);
        // A dangling sidecar still reserves its name for metadata.
        let output = temp.0.join("orphan.png");
        let sidecar = store::sidecar_for(&output);
        std::os::unix::fs::symlink("absent.json", &sidecar).unwrap();
        assert!(
            save_memory(
                &mut super::super::extract::tests::memory_sheet(),
                &output,
                &temp.0,
                &[]
            )
            .unwrap_err()
            .contains("sidecar")
        );
        assert!(!output.exists());
        assert!(sidecar.is_symlink());
        assert_eq!(std::fs::read_dir(&temp.0).unwrap().count(), 3);
    }
    #[test]
    fn save_memory_uses_png_and_changes_source_only_after_success() {
        let temp = super::super::extract::tests::Temp::new();
        let mut sheet = super::super::extract::tests::memory_sheet();
        let output = default_image_output(&sheet.source, &temp.0);
        assert_eq!(output, temp.0.join("assets/splites/map/map.png"));
        assert!(save_memory(&mut sheet, &output.with_extension("jpg"), &temp.0, &[]).is_err());
        assert!(matches!(sheet.source, SheetSource::Memory { .. }));
        std::fs::create_dir_all(output.parent().unwrap()).unwrap();
        std::fs::write(&output, b"open sibling PNG").unwrap();
        let canonical = output.canonicalize().unwrap();
        assert!(
            save_memory(
                &mut sheet,
                &output,
                &temp.0,
                std::slice::from_ref(&canonical)
            )
            .unwrap_err()
            .contains("already open")
        );
        assert_eq!(std::fs::read(&output).unwrap(), b"open sibling PNG");
        #[cfg(unix)]
        {
            // A link to the open PNG names the same file.
            let link = output.with_file_name("alias.png");
            std::os::unix::fs::symlink(&output, &link).unwrap();
            assert!(
                save_memory(&mut sheet, &link, &temp.0, std::slice::from_ref(&canonical))
                    .unwrap_err()
                    .contains("already open")
            );
            assert!(link.is_symlink());
            std::fs::remove_file(&link).unwrap();
        }
        assert!(matches!(sheet.source, SheetSource::Memory { .. }));
        let dirty = sheet.dirty;
        let meta = sheet.meta.clone();
        assert!(
            save_memory(&mut sheet, &output, &temp.0, &[])
                .unwrap_err()
                .contains("exists; choose another image path")
        );
        assert_eq!(std::fs::read(&output).unwrap(), b"open sibling PNG");
        assert_eq!(sheet.meta, meta);
        assert_eq!(sheet.dirty, dirty);
        assert!(matches!(sheet.source, SheetSource::Memory { .. }));
        assert!(!store::sidecar_for(&output).exists());
        std::fs::remove_file(&output).unwrap();
        save_memory(&mut sheet, &output, &temp.0, &[]).unwrap();
        assert!(matches!(&sheet.source,SheetSource::File(path) if path==&output));
        assert_eq!(sheet.meta.image, "map.png");
        assert!(sheet.dirty);
        assert_eq!(
            RgbaImage::from_png(&std::fs::read(&output).unwrap()).unwrap(),
            sheet.image
        );
        assert!(!store::sidecar_for(&output).exists());
        // The next crop's unchanged default output cannot replace the first crop.
        let png = std::fs::read(&output).unwrap();
        let mut other = super::super::extract::tests::memory_sheet();
        other.dirty = false;
        other.image.set_pixel(0, 0, [1, 2, 3, 255]);
        assert_eq!(default_image_output(&other.source, &temp.0), output);
        assert!(save_memory(&mut other, &output, &temp.0, &[]).is_err());
        assert!(matches!(other.source, SheetSource::Memory { .. }));
        assert!(!other.dirty);
        assert_eq!(std::fs::read(&output).unwrap(), png);
        assert!(!store::sidecar_for(&output).exists());
        assert_eq!(
            std::fs::read_dir(output.parent().unwrap()).unwrap().count(),
            1
        );
        // An existing sidecar holds someone's metadata: refuse before writing the PNG.
        std::fs::write(store::sidecar_for(&output), b"kept").unwrap();
        let png = std::fs::read(&output).unwrap();
        let mut other = super::super::extract::tests::memory_sheet();
        other.image.set_pixel(0, 0, [1, 2, 3, 255]);
        assert!(
            save_memory(&mut other, &output, &temp.0, &[])
                .unwrap_err()
                .contains("exists")
        );
        assert!(matches!(other.source, SheetSource::Memory { .. }));
        assert_eq!(std::fs::read(&output).unwrap(), png);
        assert_eq!(std::fs::read(store::sidecar_for(&output)).unwrap(), b"kept");
        // Orphan metadata alone still reserves the output name.
        std::fs::remove_file(&output).unwrap();
        assert!(save_memory(&mut other, &output, &temp.0, &[]).is_err());
        assert!(!output.exists());
        assert_eq!(std::fs::read(store::sidecar_for(&output)).unwrap(), b"kept");
        assert_eq!(
            std::fs::read_dir(output.parent().unwrap()).unwrap().count(),
            1
        );
    }
}
