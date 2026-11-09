use super::log;
use super::{Action, OpenSheet, SheetEditor, SheetSource, view};
use crate::{
    raster::{self, Dedupe, RgbaImage},
    sprite::{
        Grid, SpriteSheet,
        store::{self, StoreError},
        tilemap::{self, Cell, TileMap},
    },
};
use dear_imgui_rs::{TextureId, Ui};
use std::{
    fs, io,
    path::{Component, Path, PathBuf},
};

struct Analysis {
    dedupe: Dedupe,
    rebuild: Result<(), Vec<(u32, u32)>>,
    grouped: bool,
}
impl Analysis {
    fn new(image: &RgbaImage, grid: &Grid, grouped: bool) -> Self {
        let dedupe = raster::dedupe_with(image, grid, grouped);
        let rebuild = raster::rebuild_matches(image, grid, &dedupe);
        Self {
            dedupe,
            rebuild,
            grouped,
        }
    }
    fn report(&self) -> String {
        let d = &self.dedupe;
        let comparison = match &self.rebuild {
            Ok(()) => "identical".into(),
            Err(cells) => format!(
                "{} cells differ (first {},{})",
                cells.len(),
                cells[0].0,
                cells[0].1
            ),
        };
        format!(
            "cells {} x {} = {}, unique {}, rebuild: {comparison}",
            d.cols,
            d.rows,
            u64::from(d.cols) * u64::from(d.rows),
            d.unique.len()
        ) + if self.grouped {
            " (rotations/flips grouped)"
        } else {
            ""
        }
    }
}
pub(super) struct ExtractState {
    source: Option<(TextureId, String)>,
    grid: Option<Grid>,
    columns: u32,
    group_transforms: bool,
    output: String,
    analysis: Option<Analysis>,
    written: Option<PathBuf>,
}
impl Default for ExtractState {
    fn default() -> Self {
        Self {
            source: None,
            grid: None,
            columns: raster::DEFAULT_SHEET_COLUMNS,
            group_transforms: false,
            output: String::new(),
            analysis: None,
            written: None,
        }
    }
}
impl ExtractState {
    fn sync(&mut self, sheet: &OpenSheet, checkout: &Path) {
        let source = (sheet.imgui_id, source_stem(&sheet.source));
        if self.source.as_ref() != Some(&source) {
            self.output = default_output(&sheet.source, checkout)
                .to_string_lossy()
                .into_owned();
            self.analysis = None;
            self.written = None;
            self.source = Some(source);
        }
        if self.grid != Some(sheet.meta.grid) {
            self.analysis = None;
            self.written = None;
            self.grid = Some(sheet.meta.grid);
        }
    }
    fn set_group_transforms(&mut self, grouped: bool) {
        if self.group_transforms != grouped {
            self.group_transforms = grouped;
            self.invalidate();
        }
    }
    pub(super) fn invalidate(&mut self) {
        self.analysis = None;
        self.written = None;
    }
}

pub(super) fn source_stem(source: &SheetSource) -> String {
    match source {
        SheetSource::File(path) => path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "image".into()),
        SheetSource::Memory { name } => name.clone(),
    }
}
pub(super) fn default_output(source: &SheetSource, checkout: &Path) -> PathBuf {
    let stem = source_stem(source);
    checkout
        .join("assets/splites")
        .join(&stem)
        .join(format!("{stem}-tileset.png"))
}
pub(super) fn absolute_output(path: &Path, checkout: &Path) -> Result<PathBuf, StoreError> {
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "output path must not contain ..",
        )
        .into());
    }
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        checkout.join(path)
    };
    if !super::model::is_png(&path) || path.file_stem().is_none() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "output must have a .png extension",
        )
        .into());
    }
    Ok(path)
}
/// Refuses a write over a PNG any editor holds: its window would keep the old
/// image, and Load of that path only focuses it. A symlink or alias resolves first.
pub(super) fn refuse_open_output(output: &Path, open_files: &[PathBuf]) -> Result<(), String> {
    let resolved = output
        .canonicalize()
        .unwrap_or_else(|_| output.to_path_buf());
    if open_files.contains(&resolved) {
        return Err(format!(
            "image {} is already open; choose another image path",
            output.display()
        )
        .replace(['\n', '\r'], " "));
    }
    Ok(())
}
pub(super) fn inside_local_assets(path: &Path, checkout: &Path) -> bool {
    absolute_output(path, checkout)
        .is_ok_and(|path| path.starts_with(checkout.join("assets/splites")))
}
pub(super) fn ensure_output_dir(path: &Path, checkout: &Path) -> Result<PathBuf, StoreError> {
    let output = absolute_output(path, checkout)?;
    let parent = output
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "output needs a parent"))?;
    // Do not canonicalize through symlinks and accidentally create directories
    // outside the allowed tree. Examine each existing component without following it.
    let mut prefix = PathBuf::new();
    for component in parent.components() {
        prefix.push(component.as_os_str());
        match fs::symlink_metadata(&prefix) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "output directory must not traverse a symlink",
                )
                .into());
            }
            Ok(meta) if !meta.is_dir() => {
                return Err(io::Error::new(
                    io::ErrorKind::NotADirectory,
                    "output parent component is not a directory",
                )
                .into());
            }
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => break,
            Err(e) => return Err(e.into()),
        }
    }
    if !parent.is_dir() {
        if !inside_local_assets(&output, checkout) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "output directory does not exist; only assets/splites directories are created",
            )
            .into());
        }
        fs::create_dir_all(parent)?;
    }
    Ok(output)
}
pub(super) fn path_note(path: &Path, checkout: &Path) -> &'static str {
    if path.starts_with(checkout) && !inside_local_assets(path, checkout) {
        "; note: this path is inside the repo and not ignored"
    } else {
        ""
    }
}

struct ExtractPlan {
    output: PathBuf,
    checkout: PathBuf,
    png: Vec<u8>,
    sidecar_path: PathBuf,
    map_path: PathBuf,
    map_json: String,
    sheet: SpriteSheet,
    report: String,
}
fn plan_from_analysis(
    analysis: &Analysis,
    grid: &Grid,
    columns: u32,
    output: &Path,
    checkout: &Path,
    source_stem: &str,
) -> Result<ExtractPlan, String> {
    if analysis.dedupe.unique.is_empty() {
        return Err("no full grid cells to extract".into());
    }
    grid.validate()?;
    let output = absolute_output(output, checkout).map_err(|e| e.to_string())?;
    let name = output
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("output file name must be valid UTF-8")?;
    let sheet = SpriteSheet::new_for(
        name,
        Grid {
            tile: grid.tile,
            offset: [0, 0],
            spacing: 0,
        },
    );
    let image = raster::build_sheet(&analysis.dedupe.unique, grid.tile, columns.max(1));
    sheet
        .validate_bounds(image.width, image.height)
        .map_err(|e| e.join("; "))?;
    let map_path = tilemap::map_path_for(&output, source_stem);
    if map_path.parent() != output.parent()
        || !store::image_path_is_safe(&format!("{source_stem}.map.json"))
    {
        return Err("invalid source name for map file".into());
    }
    let map = TileMap {
        version: 1,
        tileset: name.to_owned(),
        tile: grid.tile,
        cols: analysis.dedupe.cols,
        rows: analysis.dedupe.rows,
        cells: analysis
            .dedupe
            .map
            .iter()
            .map(|&(index, transform)| Cell {
                index,
                flip_x: transform.flip_x(),
                rotation: transform.rotation(),
            })
            .collect(),
    };
    map.validate_against(image.width, image.height)?;
    Ok(ExtractPlan {
        map_path,
        map_json: tilemap::to_json(&map),
        sidecar_path: store::sidecar_for(&output),
        sheet,
        output,
        checkout: checkout.to_path_buf(),
        png: image.to_png()?,
        report: analysis.report(),
    })
}
#[cfg(test)]
fn plan_extract(
    image: &RgbaImage,
    grid: &Grid,
    columns: u32,
    output: &Path,
    checkout: &Path,
) -> Result<ExtractPlan, String> {
    plan_from_analysis(
        &Analysis::new(image, grid, false),
        grid,
        columns,
        output,
        checkout,
        "map",
    )
}

fn save_sidecar_new(path: &Path, sheet: &SpriteSheet) -> Result<(), StoreError> {
    sheet.validate().map_err(StoreError::Invalid)?;
    store::publish_new(path, store::to_json(sheet).as_bytes())
}
fn write_plan(plan: &ExtractPlan, open_files: &[PathBuf]) -> Result<String, String> {
    let output = absolute_output(&plan.output, &plan.checkout).map_err(|e| e.to_string())?;
    refuse_open_output(&output, open_files)?;
    let output = ensure_output_dir(&output, &plan.checkout).map_err(|e| e.to_string())?;
    store::write_atomic(&output, &plan.png).map_err(|e| format!("PNG not written: {e}"))?;
    // PNG and map are regenerated artifacts, replaced even when edited metadata exists.
    // The three files are not a transaction: after any JSON error the PNG remains.
    let sidecar = match save_sidecar_new(&plan.sidecar_path, &plan.sheet) {
        Ok(()) => format!("sidecar written: {}", plan.sidecar_path.display()),
        Err(error) if matches!(&error,StoreError::Io(e) if e.kind()==io::ErrorKind::AlreadyExists) => {
            "sidecar not written: sidecar exists; remove it first".into()
        }
        Err(error) => format!("sidecar not written: {error}"),
    };
    let map_write = match fs::symlink_metadata(&plan.map_path) {
        Ok(meta) if meta.file_type().is_symlink() => Err("map output is a symlink".to_string()),
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e.to_string()),
        _ => {
            store::write_atomic(&plan.map_path, plan.map_json.as_bytes()).map_err(|e| e.to_string())
        }
    };
    let map = match map_write {
        Ok(()) => format!("map written: {}", plan.map_path.display()),
        Err(error) => format!("map not written: {error}"),
    };
    let message = format!(
        "PNG written: {}; {sidecar}; {map}{}",
        output.display(),
        path_note(&output, &plan.checkout)
    );
    if sidecar.starts_with("sidecar not") || map.starts_with("map not") {
        Err(message)
    } else {
        Ok(message)
    }
}

pub(super) fn tab(editor: &mut SheetEditor, ui: &Ui, open_files: &[PathBuf]) {
    let Some(_tab) = ui.tab_item("Extract") else {
        return;
    };
    let SheetEditor {
        sheet,
        extract: state,
        log,
        pending,
        ..
    } = editor;
    let checkout = match std::env::current_dir() {
        Ok(path) => path,
        Err(e) => {
            log::append(log, e.to_string());
            return;
        }
    };
    state.sync(sheet, &checkout);
    let mut changed = view::input_u32(ui, "Columns", &mut state.columns, 1);
    changed |= ui.input_text("Output", &mut state.output).build();
    if changed {
        state.written = None;
    }
    let mut grouped = state.group_transforms;
    if ui.checkbox("Group rotations/flips", &mut grouped) {
        state.set_group_transforms(grouped);
    }
    if ui.button("Analyze") {
        // Synchronous, explicit action. Large sheets pause this dev tool.
        state.analysis = Some(Analysis::new(
            &sheet.image,
            &sheet.meta.grid,
            state.group_transforms,
        ));
        state.written = None;
    }
    if let Some(analysis) = &state.analysis {
        let report = analysis.report();
        if analysis.rebuild.is_err() {
            ui.text_colored([1.0, 0.3, 0.3, 1.0], &report);
        } else {
            ui.text_wrapped(&report);
        }
    }
    let ready = state
        .analysis
        .as_ref()
        .is_some_and(|a| !a.dedupe.unique.is_empty());
    {
        let _disabled = ui.begin_disabled_with_cond(!ready);
        if ui.button("Write tileset") && ready {
            state.written = None;
            let analysis = state.analysis.as_ref().unwrap();
            match plan_from_analysis(
                analysis,
                &sheet.meta.grid,
                state.columns,
                Path::new(&state.output),
                &checkout,
                &source_stem(&sheet.source),
            )
            .and_then(|plan| {
                write_plan(&plan, open_files).map(|message| format!("{message}; {}", plan.report))
            }) {
                Ok(message) => {
                    log::append(log, message);
                    state.written = absolute_output(Path::new(&state.output), &checkout).ok();
                }
                Err(error) => log::append(log, error),
            }
        }
    }
    if let Some(path) = &state.written
        && ui.button("Open result")
    {
        pending.push(Action::Load(path.clone()));
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use std::{
        io::Read,
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT_STAGE: AtomicU64 = AtomicU64::new(0);
    pub(crate) struct Temp(pub PathBuf);
    impl Temp {
        pub(crate) fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "rn-extract-{}-{}",
                std::process::id(),
                NEXT_STAGE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    pub(crate) fn memory_sheet() -> OpenSheet {
        let (image, _) = crate::sprite::example();
        let meta = SpriteSheet::new_for(
            "map.png",
            Grid {
                tile: 16,
                offset: [0, 0],
                spacing: 0,
            },
        );
        OpenSheet {
            source: SheetSource::Memory { name: "map".into() },
            image,
            meta,
            texture: None,
            imgui_id: TextureId::new(50),
            external: None,
            dirty: true,
            selection: Default::default(),
        }
    }
    fn map(spacing: u32, offset: [u32; 2]) -> (RgbaImage, Grid) {
        let grid = Grid {
            tile: 16,
            offset,
            spacing,
        };
        let (x, y) = grid.origin(3, 1);
        let mut image = RgbaImage::new(x + 16, y + 16);
        for (i, id) in [1, 2, 1, 3, 2, 1, 3, 3].into_iter().enumerate() {
            let (x, y) = grid.origin(i as u32 % 4, i as u32 / 4);
            image.fill_rect(x, y, 16, 16, [id, 20, 30, 255]);
        }
        (image, grid)
    }
    fn assert_no_temps(root: &Path) {
        for entry in fs::read_dir(root).unwrap() {
            let entry = entry.unwrap();
            assert!(
                !entry.file_name().to_string_lossy().contains(".tmp-"),
                "{}",
                entry.path().display()
            );
            if entry.file_type().unwrap().is_dir() {
                assert_no_temps(&entry.path());
            }
        }
    }
    #[test]
    fn extraction_plan_has_unique_tiles_empty_metadata_and_diagnostics() {
        let temp = Temp::new();
        for (spacing, offset) in [(0, [0, 0]), (1, [2, 0])] {
            let (image, grid) = map(spacing, offset);
            let plan = plan_extract(&image, &grid, 16, &temp.0.join("map.png"), &temp.0).unwrap();
            let png = RgbaImage::from_png(&plan.png).unwrap();
            assert_eq!((png.width, png.height), (48, 16));
            assert_eq!(
                plan.sheet.grid,
                Grid {
                    tile: 16,
                    offset: [0, 0],
                    spacing: 0
                }
            );
            assert_eq!(plan.sheet.image, "map.png");
            assert!(
                plan.sheet.tiles.is_empty()
                    && plan.sheet.sprites.is_empty()
                    && plan.sheet.clips.is_empty()
            );
            assert!(plan.report.contains("unique 3") && plan.report.contains("identical"));
            assert_eq!(
                png,
                raster::build_sheet(&raster::dedupe(&image, &grid).unique, 16, 16)
            );
            let mut analysis = Analysis::new(&image, &grid, false);
            analysis.dedupe.map[0] = (1, crate::sprite::Transform::IDENTITY);
            analysis.rebuild = raster::rebuild_matches(&image, &grid, &analysis.dedupe);
            assert!(analysis.report().contains("1 cells differ (first 0,0)"));
        }
        assert!(
            plan_extract(
                &RgbaImage::new(7, 7),
                &Grid {
                    tile: 16,
                    offset: [0, 0],
                    spacing: 0
                },
                16,
                &temp.0.join("map.png"),
                &temp.0
            )
            .is_err()
        );
        assert!(
            plan_extract(
                &map(0, [0, 0]).0,
                &map(0, [0, 0]).1,
                16,
                &temp.0.join("map.jpg"),
                &temp.0
            )
            .is_err()
        );
    }
    #[test]
    fn grouped_map_rebuilds_all_eight_variants() {
        let temp = Temp::new();
        let mut tile = RgbaImage::new(2, 2);
        for y in 0..2 {
            for x in 0..2 {
                tile.set_pixel(x, y, [(1 + x + 2 * y) as u8, 0, 0, 255]);
            }
        }
        let mut image = RgbaImage::new(16, 2);
        for (index, transform) in crate::sprite::Transform::ALL.into_iter().enumerate() {
            let variant = tile.transformed(transform);
            for y in 0..2 {
                for x in 0..2 {
                    image.set_pixel(index as u32 * 2 + x, y, variant.pixel(x, y));
                }
            }
        }
        let grid = Grid {
            tile: 2,
            offset: [0, 0],
            spacing: 0,
        };
        let analysis = Analysis::new(&image, &grid, true);
        let plan = plan_from_analysis(
            &analysis,
            &grid,
            16,
            &temp.0.join("tiles.png"),
            &temp.0,
            "map",
        )
        .unwrap();
        write_plan(&plan, &[]).unwrap();
        let map = tilemap::parse(&fs::read_to_string(&plan.map_path).unwrap()).unwrap();
        assert_eq!(
            map.rebuild(&RgbaImage::from_png(&plan.png).unwrap())
                .unwrap(),
            image
        );
    }
    #[cfg(unix)]
    #[test]
    fn map_destination_symlink_is_refused() {
        let temp = Temp::new();
        let (image, grid) = map(0, [0, 0]);
        let plan = plan_extract(&image, &grid, 16, &temp.0.join("tiles.png"), &temp.0).unwrap();
        let target = temp.0.join("untouched");
        fs::write(&target, b"untouched").unwrap();
        std::os::unix::fs::symlink(&target, &plan.map_path).unwrap();
        let error = write_plan(&plan, &[]).unwrap_err();
        assert!(error.contains("map not written: map output is a symlink"));
        assert_eq!(fs::read(target).unwrap(), b"untouched");
        assert_no_temps(&temp.0);
    }
    #[test]
    fn defaults_and_output_directory_boundaries() {
        let temp = Temp::new();
        let file = SheetSource::File("/x/map-01.png".into());
        let memory = SheetSource::Memory {
            name: "224070-region-1".into(),
        };
        assert_eq!(
            default_output(&file, &temp.0),
            temp.0.join("assets/splites/map-01/map-01-tileset.png")
        );
        assert_eq!(
            default_output(&memory, &temp.0),
            temp.0
                .join("assets/splites/224070-region-1/224070-region-1-tileset.png")
        );
        let output = default_output(&file, &temp.0);
        assert!(inside_local_assets(&output, &temp.0));
        assert!(inside_local_assets(
            Path::new("assets/splites/map/out.png"),
            &temp.0
        ));
        assert!(!inside_local_assets(
            &temp.0.join("assets/splites/../x/out.png"),
            &temp.0
        ));
        assert!(!inside_local_assets(
            &temp.0.join("assets/splites-other/out.png"),
            &temp.0
        ));
        ensure_output_dir(&output, &temp.0).unwrap();
        assert!(output.parent().unwrap().is_dir());
        for directory in [
            temp.0.join("other"),
            temp.0
                .parent()
                .unwrap()
                .join(format!("rn-extract-forbidden-{}", std::process::id())),
        ] {
            assert!(!directory.exists());
            assert!(ensure_output_dir(&directory.join("out.png"), &temp.0).is_err());
            assert!(!directory.exists());
        }
        assert!(ensure_output_dir(&temp.0.join("out.png"), &temp.0).is_ok());
        assert!(
            path_note(&temp.0.join("out.png"), &temp.0).contains("inside the repo and not ignored")
        );
        assert_eq!(path_note(&output, &temp.0), "");
        #[cfg(unix)]
        {
            let external = Temp::new();
            std::os::unix::fs::symlink(&external.0, temp.0.join("assets/splites/link")).unwrap();
            assert!(
                ensure_output_dir(&temp.0.join("assets/splites/link/new/out.png"), &temp.0)
                    .is_err()
            );
            assert!(!external.0.join("new").exists());
        }
    }
    #[test]
    fn opened_output_is_refused_before_any_disk_changes() {
        let temp = Temp::new();
        let (image, grid) = map(0, [0, 0]);
        let source = temp.0.join("original.png");
        fs::write(&source, image.to_png().unwrap()).unwrap();
        let original = fs::read(&source).unwrap();
        let open_files = [source.canonicalize().unwrap()];
        let mut outputs = vec![source.clone()];
        #[cfg(unix)]
        {
            let alias = temp.0.join("alias.png");
            std::os::unix::fs::symlink(&source, &alias).unwrap();
            outputs.push(alias);
        }
        let entries = fs::read_dir(&temp.0).unwrap().count();
        for output in outputs {
            let plan = plan_extract(&image, &grid, 16, &output, &temp.0).unwrap();
            let error = write_plan(&plan, &open_files).unwrap_err();
            assert!(error.contains("already open"), "{error}");
            assert!(error.is_ascii() && !error.contains(['\r', '\n']));
            assert_eq!(fs::read(&source).unwrap(), original);
            assert!(!plan.sidecar_path.exists());
            assert_eq!(fs::read_dir(&temp.0).unwrap().count(), entries);
            assert_no_temps(&temp.0);
        }
        // A still-open editor may hold a path whose PNG has since disappeared.
        // The refusal must happen before ensure_output_dir creates this parent.
        let missing = temp.0.join("assets/splites/missing/result.png");
        let plan = plan_extract(&image, &grid, 16, &missing, &temp.0).unwrap();
        assert!(
            write_plan(&plan, std::slice::from_ref(&missing))
                .unwrap_err()
                .contains("already open")
        );
        assert!(!temp.0.join("assets").exists());
        assert_eq!(fs::read_dir(&temp.0).unwrap().count(), entries);
        // Once no editor holds that path, the same new output is writable.
        write_plan(&plan, &[]).unwrap();
        assert_eq!(fs::read(&missing).unwrap(), plan.png);
        assert_no_temps(&temp.0);
    }
    #[test]
    fn writes_are_atomic_png_replaced_sidecar_preserved_and_temps_cleaned() {
        let temp = Temp::new();
        let (mut image, grid) = map(1, [2, 0]);
        let output = temp.0.join("assets/splites/map/tiles.png");
        let first = plan_extract(&image, &grid, 16, &output, &temp.0).unwrap();
        write_plan(&first, &[]).unwrap();
        assert_eq!(first.map_path, output.with_file_name("map.map.json"));
        let stored_map = tilemap::parse(&fs::read_to_string(&first.map_path).unwrap()).unwrap();
        let tileset = RgbaImage::from_png(&fs::read(&output).unwrap()).unwrap();
        assert_eq!(stored_map.rebuild(&tileset).unwrap(), map(0, [0, 0]).0);
        let saved = store::load(&first.sidecar_path).unwrap();
        assert_eq!(saved.sheet, first.sheet);
        assert_eq!(
            RgbaImage::from_png(&fs::read(&output).unwrap()).unwrap(),
            RgbaImage::from_png(&first.png).unwrap()
        );
        let original_json = fs::read(&first.sidecar_path).unwrap();
        let mut old_inode = fs::File::open(&output).unwrap();
        image.set_pixel(2, 0, [200, 100, 50, 255]);
        let second = plan_extract(&image, &grid, 16, &output, &temp.0).unwrap();
        // A PNG open in an editor is left alone, bytes and sidecar included.
        let open = [output.canonicalize().unwrap()];
        assert!(
            write_plan(&second, &open)
                .unwrap_err()
                .contains("already open")
        );
        assert_eq!(fs::read(&output).unwrap(), first.png);
        let error = write_plan(&second, &[]).unwrap_err();
        assert!(
            error.contains("PNG written:") && error.contains("sidecar exists; remove it first"),
            "{error}"
        );
        assert_eq!(fs::read(&first.sidecar_path).unwrap(), original_json);
        assert_eq!(
            fs::read_to_string(&second.map_path).unwrap(),
            second.map_json
        );
        assert!(error.contains("map written:"));
        assert_eq!(fs::read(&output).unwrap(), second.png);
        let mut original_png = Vec::new();
        old_inode.read_to_end(&mut original_png).unwrap();
        assert_eq!(
            original_png, first.png,
            "atomic replacement must leave old open inode intact"
        );
        assert_no_temps(&temp.0);
        // A publication error after PNG success is reported explicitly and cleans staging files.
        fs::remove_file(&first.sidecar_path).unwrap();
        fs::create_dir(&first.sidecar_path).unwrap();
        assert!(
            write_plan(&first, &[])
                .unwrap_err()
                .contains("PNG written:")
        );
        assert_no_temps(&temp.0);
    }
    #[test]
    fn sidecar_publication_validates_before_creating_files() {
        let temp = Temp::new();
        let path = temp.0.join("invalid.sprites.json");
        let mut sheet = memory_sheet().meta;
        sheet.grid.tile = 0;
        assert!(matches!(
            save_sidecar_new(&path, &sheet),
            Err(StoreError::Invalid(_))
        ));
        assert_eq!(fs::read_dir(&temp.0).unwrap().count(), 0);
    }
    #[test]
    fn sidecar_publication_cannot_clobber_a_concurrent_writer() {
        let temp = Temp::new();
        let path = temp.0.join("same.sprites.json");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let workers: Vec<_> = (0..2)
            .map(|i| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let sheet = SpriteSheet::new_for(
                        &format!("image-{i}.png"),
                        Grid {
                            tile: 16,
                            offset: [0, 0],
                            spacing: 0,
                        },
                    );
                    barrier.wait();
                    save_sidecar_new(&path, &sheet).is_ok()
                })
            })
            .collect();
        assert_eq!(
            workers
                .into_iter()
                .map(|w| usize::from(w.join().unwrap()))
                .sum::<usize>(),
            1
        );
        assert!(store::load(&path).is_ok());
        assert_no_temps(&temp.0);
    }
    #[test]
    fn grouping_toggle_invalidates_results_and_is_reported() {
        let sheet = memory_sheet();
        let mut state = ExtractState::default();
        assert!(!state.group_transforms);
        state.analysis = Some(Analysis::new(&sheet.image, &sheet.meta.grid, false));
        state.written = Some("old.png".into());
        state.set_group_transforms(true);
        assert!(state.analysis.is_none() && state.written.is_none());
        assert!(
            Analysis::new(&sheet.image, &sheet.meta.grid, true)
                .report()
                .contains("rotations/flips grouped")
        );
    }
    #[test]
    fn grid_and_image_changes_invalidate_analysis_and_written_result() {
        let temp = Temp::new();
        let mut sheet = memory_sheet();
        let mut state = ExtractState::default();
        state.sync(&sheet, &temp.0);
        state.analysis = Some(Analysis::new(
            &sheet.image,
            &sheet.meta.grid,
            state.group_transforms,
        ));
        state.written = Some("old.png".into());
        sheet.meta.grid.offset = [2, 0];
        state.sync(&sheet, &temp.0);
        assert!(state.analysis.is_none() && state.written.is_none());
        state.analysis = Some(Analysis::new(
            &sheet.image,
            &sheet.meta.grid,
            state.group_transforms,
        ));
        sheet.meta.grid.spacing = 1;
        state.sync(&sheet, &temp.0);
        assert!(state.analysis.is_none());
        state.analysis = Some(Analysis::new(
            &sheet.image,
            &sheet.meta.grid,
            state.group_transforms,
        ));
        sheet.meta.grid.tile = 8;
        state.sync(&sheet, &temp.0);
        assert!(state.analysis.is_none());
        state.analysis = Some(Analysis::new(
            &sheet.image,
            &sheet.meta.grid,
            state.group_transforms,
        ));
        sheet.imgui_id = TextureId::new(51);
        state.sync(&sheet, &temp.0);
        assert!(state.analysis.is_none());
    }
}
