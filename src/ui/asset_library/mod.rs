mod animations;
mod edit;
mod extract;
mod guess;
mod job;
mod log;
mod model;
mod preview;
mod props;
mod regions;
mod sprites;
mod tileset;
mod view;
pub use model::*;

use crate::{
    raster::{self, GridGuess, RgbaImage},
    sprite::{Grid, SpriteSheet, store, texture::SheetTexture},
};
use dear_imgui_glow::{ExternalTextureId, GlowRenderer};
use dear_imgui_rs::{Condition, TabItemOptions, TabItemPlacement, TextureId, TreeNodeFlags, Ui};
use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

fn resolve_load_path(given: &Path) -> Result<PathBuf, String> {
    // Unlike store::load, editor Load requires an existing image. Use the exact
    // path the common rule checked, before canonical identity can focus an editor.
    store::image_within_directory(given, given.parent().unwrap_or(Path::new("."))).map_err(|e| {
        match e {
            store::StoreError::Io(error) => error.to_string(),
            other => other.to_string(),
        }
    })
}

#[derive(Debug, PartialEq, Eq)]
enum LoadStep {
    Focused,
    Open(PathBuf),
}

pub enum SheetSource {
    File(PathBuf),
    Memory { name: String },
}
pub struct OpenSheet {
    pub source: SheetSource,
    pub image: RgbaImage,
    pub texture: Option<SheetTexture>,
    pub imgui_id: TextureId,
    pub external: Option<ExternalTextureId>,
    pub meta: SpriteSheet,
    pub dirty: bool,
    pub selection: Selection,
}
/// Identifies one open sheet so queued actions reach the editor that asked.
pub type SheetId = u64;
pub enum Action {
    Load(PathBuf),
    Reload(SheetId),
    Close(SheetId),
    Save(SheetId),
    Replace {
        image: RgbaImage,
        name: String,
        grid: Grid,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum EditorTab {
    #[default]
    Tileset,
    Sprites,
    Animations,
    Import,
}

/// Everything needed to edit one sheet. The library owns a list of these.
pub struct SheetEditor {
    pub id: SheetId,
    pub sheet: OpenSheet,
    now: fn() -> u64,
    active_tab: EditorTab,
    focus_requested: bool,
    confirm_close: bool,
    #[cfg(test)]
    close_controls: [[f32; 2]; 3],
    #[cfg(test)]
    tile_name_input: [f32; 2],
    #[cfg(test)]
    sprite_name_input: [f32; 2],
    #[cfg(test)]
    sprite_new_input: [f32; 2],
    pub zoom: u32,
    pub show_grid: bool,
    pub guess: Option<GridGuess>,
    guess_job: Option<job::Job<GridGuess>>,
    pub log: log::SheetLog,
    log_view: log::LogView,
    /// Actions this editor queued; the library takes them after drawing.
    pub pending: Vec<Action>,
    /// Where Save image as wrote the memory image, for the library's PNG path field.
    saved_path: Option<PathBuf>,
    tile_draft: tileset::TileDraft,
    selected_sprite: Option<u64>,
    sprite_props_target: Option<u64>,
    sprite_props: props::PropsDraft,
    selected_animation: Option<u64>,
    animation_draft: animations::AnimationDraft,
    preview: preview::PreviewState,
    regions: regions::RegionsState,
    extract: extract::ExtractState,
}
pub struct AssetLibrary {
    pub open: bool,
    pub path_input: String,
    pub status: String,
    pub assets: Vec<PathBuf>,
    pub key_color: String,
    pub use_key: bool,
    pub sheets: Vec<SheetEditor>,
    pub pending: Vec<Action>,
    assets_scanned: bool,
    next_id: SheetId,
    #[cfg(test)]
    import_controls: tests::ImportControls,
}
impl Default for AssetLibrary {
    fn default() -> Self {
        Self {
            open: false,
            path_input: String::new(),
            status: String::new(),
            assets: vec![],
            key_color: "ff00ff".into(),
            use_key: false,
            sheets: vec![],
            pending: vec![],
            assets_scanned: false,
            next_id: 0,
            #[cfg(test)]
            import_controls: Default::default(),
        }
    }
}
impl AssetLibrary {
    pub fn queue_drop(&mut self, path: PathBuf) {
        self.open = true;
        self.pending.push(Action::Load(path));
    }
    pub fn apply_pending(&mut self, gl: &Rc<glow::Context>, renderer: &mut GlowRenderer) {
        for editor in &mut self.sheets {
            editor.poll_guess();
            editor.poll_find();
            editor.poll_analysis();
            editor.poll_batch();
        }
        for action in std::mem::take(&mut self.pending) {
            let target = match &action {
                Action::Save(id) | Action::Reload(id) | Action::Close(id) => Some(*id),
                _ => None,
            };
            if let Err(error) = self.apply_action(action, gl, renderer) {
                if let Some(editor) = target.and_then(|id| self.editor_mut(id)) {
                    editor.log(error);
                } else {
                    self.status = error.replace(['\n', '\r'], " ");
                }
            }
        }
    }
    fn key(&self) -> Result<Option<[u8; 3]>, String> {
        if self.use_key {
            raster::parse_hex_color(&self.key_color).map(Some)
        } else {
            Ok(None)
        }
    }
    fn editor_mut(&mut self, id: SheetId) -> Option<&mut SheetEditor> {
        self.sheets.iter_mut().find(|e| e.id == id)
    }
    /// Refuse escaping paths before focus, reads or uploads, even if their target is open.
    /// Takes the input by value, so a caller can only open the path `Open` returns.
    fn begin_load(&mut self, given: PathBuf) -> Result<LoadStep, String> {
        let path = resolve_load_path(&given)?;
        if let Some(editor) = self
            .sheets
            .iter_mut()
            .find(|editor| matches!(&editor.sheet.source, SheetSource::File(open) if open == &path))
        {
            editor.focus_requested = true;
            return Ok(LoadStep::Focused);
        }
        Ok(LoadStep::Open(path))
    }
    fn apply_action(
        &mut self,
        action: Action,
        gl: &Rc<glow::Context>,
        renderer: &mut GlowRenderer,
    ) -> Result<(), String> {
        match action {
            Action::Load(path) => {
                let path = match self.begin_load(path)? {
                    LoadStep::Focused => return Ok(()),
                    LoadStep::Open(path) => path,
                };
                let (image, meta, dirty, dropped) = read_sheet(&path, self.key()?)?;
                let opened = OpenSheet::prepare(
                    SheetSource::File(path.clone()),
                    image,
                    meta,
                    dirty,
                    gl,
                    renderer,
                )?;
                self.install_with_warning(opened, &dropped);
                self.path_input = path.to_string_lossy().into_owned();
                self.status = format!("Opened {}", path.display());
            }
            Action::Replace { image, name, grid } => {
                let meta = SpriteSheet::new_for(&format!("{name}.png"), grid);
                let opened = OpenSheet::prepare(
                    SheetSource::Memory { name },
                    image,
                    meta,
                    true,
                    gl,
                    renderer,
                )?;
                self.install(opened);
                self.status = "Memory image: save the image first".into();
            }
            Action::Reload(id) => {
                let key = self.key()?;
                let Some(editor) = self.editor_mut(id) else {
                    return Ok(());
                };
                let sheet = &editor.sheet;
                let SheetSource::File(path) = &sheet.source else {
                    return Err("save the image first".into());
                };
                let (image, meta, _) = open_sheet(
                    path,
                    &std::fs::read(path).map_err(|e| e.to_string())?,
                    key,
                    Some(sheet.meta.clone()),
                )?;
                let opened = OpenSheet::prepare(
                    SheetSource::File(path.clone()),
                    image,
                    meta,
                    sheet.dirty,
                    gl,
                    renderer,
                )?;
                editor.replace_sheet(opened, renderer)?;
                editor.log("Image reloaded; metadata retained");
            }
            Action::Close(id) => {
                self.close(id, renderer)?;
                self.status = "Sheet closed".into();
            }
            Action::Save(id) => {
                let Some(editor) = self.editor_mut(id) else {
                    return Ok(());
                };
                let message = editor.save()?;
                editor.log(message);
            }
        }
        Ok(())
    }
    /// Adds an independent editor. File identity is checked before loading/uploading.
    fn install(&mut self, opened: OpenSheet) {
        let id = self.next_id;
        self.next_id += 1;
        let mut editor = SheetEditor::new(id, opened);
        match &editor.sheet.source {
            SheetSource::File(path) => editor.log(format!("Opened {}", path.display())),
            SheetSource::Memory { .. } => editor.log("Memory image: save the image first"),
        }
        self.sheets.push(editor);
    }
    fn install_with_warning(&mut self, opened: OpenSheet, dropped: &[(u32, u32)]) {
        self.install(opened);
        if let Some(warning) = store::dropped_tile_animation_warning(dropped) {
            self.sheets
                .last_mut()
                .expect("installed editor")
                .log(warning);
        }
    }
    /// Releases one sheet's texture and drops its editor. On error the editor stays.
    pub fn close(&mut self, id: SheetId, renderer: &mut GlowRenderer) -> Result<(), String> {
        let Some(index) = self.sheets.iter().position(|e| e.id == id) else {
            return Ok(());
        };
        self.sheets[index].release_texture(renderer)?;
        self.sheets.remove(index);
        Ok(())
    }
    pub fn close_all(&mut self, renderer: &mut GlowRenderer) -> Result<(), String> {
        for id in self.sheets.iter().map(|e| e.id).collect::<Vec<_>>() {
            self.close(id, renderer)?;
        }
        Ok(())
    }
    pub fn draw(&mut self, ui: &Ui, now_ms: u64) {
        if self.open {
            if !self.assets_scanned {
                self.refresh_assets();
                self.assets_scanned = true;
            }
            let mut open = self.open;
            ui.window("Asset Library")
                .opened(&mut open)
                .size([620.0, 260.0], Condition::FirstUseEver)
                .position([140.0, 40.0], Condition::FirstUseEver)
                .build(|| {
                    self.import_panel(ui);
                    ui.text_wrapped(&self.status);
                });
            self.open = open;
        }
        let open_files: Vec<_> = self
            .sheets
            .iter()
            .filter_map(|editor| match &editor.sheet.source {
                SheetSource::File(path) => Some(path.clone()),
                SheetSource::Memory { .. } => None,
            })
            .collect();
        for editor in &mut self.sheets {
            editor.draw(ui, now_ms, &open_files);
        }
        self.collect_editor_output();
    }
    /// Moves what the editors produced while drawing into the library.
    fn collect_editor_output(&mut self) {
        for editor in &mut self.sheets {
            self.pending.append(&mut editor.pending);
            if let Some(path) = editor.saved_path.take() {
                self.path_input = path.to_string_lossy().into_owned();
            }
        }
    }
    fn refresh_assets(&mut self) {
        match std::env::current_dir() {
            Ok(root) => match list_assets(&root.join("assets")) {
                Ok(files) => {
                    self.assets = files;
                    self.status = old_output_folder_hint(&root).unwrap_or_default();
                }
                Err(error) => {
                    self.assets.clear();
                    self.status = error;
                }
            },
            Err(error) => self.status = error.to_string(),
        }
    }
    fn import_panel(&mut self, ui: &Ui) {
        let style = ui.clone_style();
        let reserved_width = ui.calc_text_size("PNG path")[0]
            + style.item_inner_spacing()[0]
            + ui.calc_text_size("Load")[0]
            + 2.0 * style.frame_padding()[0]
            + style.item_spacing()[0];
        ui.set_next_item_width((ui.content_region_avail()[0] - reserved_width).max(1.0));
        ui.input_text("PNG path", &mut self.path_input).build();
        #[cfg(test)]
        {
            self.import_controls.input = tests::ImportItem::last(ui);
        }
        ui.same_line();
        {
            let _disabled =
                ui.begin_disabled_with_cond(!import_path_has_file_name(&self.path_input));
            if ui.button("Load") {
                self.pending
                    .push(Action::Load(PathBuf::from(self.path_input.trim())));
            }
            #[cfg(test)]
            {
                self.import_controls.load = tests::ImportItem::last(ui);
            }
        }
        if ui.button("Browse...")
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("PNG", &["png"])
                .pick_file()
        {
            self.pending.push(Action::Load(path));
        }
        #[cfg(test)]
        {
            self.import_controls.browse = tests::ImportItem::last(ui);
        }
        ui.same_line();
        if ui.button("Refresh assets/") {
            self.refresh_assets();
        }
        #[cfg(test)]
        {
            self.import_controls.refresh = tests::ImportItem::last(ui);
            self.import_controls.asset = None;
        }
        ui.checkbox("Key color", &mut self.use_key);
        ui.same_line();
        ui.input_text("RGB hex", &mut self.key_color).build();
        ui.child_window("assets list")
            .size([0.0, 62.0])
            .build(ui, || {
                for path in &self.assets {
                    if ui.selectable(path.to_string_lossy()) {
                        self.pending.push(Action::Load(path.clone()));
                    }
                    #[cfg(test)]
                    {
                        self.import_controls.asset = Some(tests::ImportItem::last(ui));
                    }
                }
            });
    }
}
impl SheetEditor {
    pub fn new(id: SheetId, sheet: OpenSheet) -> Self {
        Self {
            id,
            sheet,
            now: || {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            },
            active_tab: EditorTab::default(),
            focus_requested: true,
            confirm_close: false,
            #[cfg(test)]
            close_controls: [[0.0; 2]; 3],
            #[cfg(test)]
            tile_name_input: [0.0; 2],
            #[cfg(test)]
            sprite_name_input: [0.0; 2],
            #[cfg(test)]
            sprite_new_input: [0.0; 2],
            zoom: 2,
            show_grid: true,
            guess: None,
            guess_job: None,
            log: Default::default(),
            log_view: Default::default(),
            pending: vec![],
            saved_path: None,
            tile_draft: Default::default(),
            selected_sprite: None,
            sprite_props_target: None,
            sprite_props: Default::default(),
            selected_animation: None,
            animation_draft: Default::default(),
            preview: Default::default(),
            regions: Default::default(),
            extract: Default::default(),
        }
    }
    /// Swaps in a reloaded or replacing image. View settings and per-tab inputs stay;
    /// selections, drafts and results tied to the old image are cleared.
    fn replace_sheet(
        &mut self,
        mut opened: OpenSheet,
        renderer: &mut GlowRenderer,
    ) -> Result<(), String> {
        if let Err(error) = self.release_texture(renderer) {
            if let Some(id) = opened.external.take() {
                let _ = renderer.unregister_external_texture(id);
            }
            return Err(error);
        }
        self.cancel_guess();
        self.cancel_find();
        self.cancel_analysis();
        self.cancel_batch();
        self.sheet = opened;
        self.selected_sprite = None;
        self.sprite_props_target = None;
        self.sprite_props = Default::default();
        self.selected_animation = None;
        self.animation_draft = Default::default();
        self.preview = Default::default();
        self.guess = None;
        self.tile_draft = Default::default();
        Ok(())
    }
    fn release_texture(&mut self, renderer: &mut GlowRenderer) -> Result<(), String> {
        if let Some(id) = self.sheet.external {
            renderer
                .unregister_external_texture(id)
                .map_err(|e| e.to_string())?;
            self.sheet.external = None;
        }
        Ok(())
    }
    fn can_save(&self) -> bool {
        self.sheet.dirty && matches!(self.sheet.source, SheetSource::File(_))
    }
    fn save(&mut self) -> Result<String, String> {
        self.tile_draft.validate(&self.sheet.meta)?;
        if self.sprite_props_target.is_some() {
            self.sprite_props.validate()?;
        }
        self.animation_draft.validate()?;
        let sheet = &mut self.sheet;
        let SheetSource::File(path) = &sheet.source else {
            return Err("save the image first".into());
        };
        sheet
            .meta
            .validate_bounds(sheet.image.width, sheet.image.height)
            .map_err(|e| e.join("; "))?;
        let sidecar = store::sidecar_for(path);
        store::save(&sidecar, &sheet.meta).map_err(|e| e.to_string())?;
        sheet.dirty = false;
        Ok(format!("Saved {}", sidecar.display()))
    }
    fn window_title(&self) -> String {
        let name = match &self.sheet.source {
            SheetSource::File(path) => path
                .file_name()
                .map_or_else(|| "?".into(), |s| s.to_string_lossy()),
            SheetSource::Memory { name } => name.as_str().into(),
        };
        format!(
            "{name}{} - Sheet Editor###sheet-{}",
            if self.sheet.dirty { " *" } else { "" },
            self.id
        )
    }
    fn request_close(&mut self) {
        if self.sheet.dirty {
            self.confirm_close = true;
        } else {
            self.pending.push(Action::Close(self.id));
        }
    }
    fn draw(&mut self, ui: &Ui, now_ms: u64, open_files: &[PathBuf]) {
        let mut open = true;
        let focus = std::mem::take(&mut self.focus_requested);
        let mut window = ui
            .window(self.window_title())
            .opened(&mut open)
            .size([1280.0, 760.0], Condition::FirstUseEver)
            .position(
                [
                    180.0 + (self.id % 8) as f32 * 28.0,
                    70.0 + (self.id % 8) as f32 * 28.0,
                ],
                Condition::FirstUseEver,
            )
            .focused(focus);
        if focus || self.confirm_close {
            window = window.collapsed(false, Condition::Always);
        }
        window.build(|| {
            #[cfg(test)]
            {
                let pos = ui.window_pos();
                self.close_controls[0] = [
                    pos[0] + ui.window_size()[0] - ui.frame_height() * 0.5,
                    pos[1] + ui.frame_height() * 0.5,
                ];
            }
            let footer = log::HEIGHT + ui.frame_height_with_spacing() + 12.0;
            ui.child_window("Editor controls")
                .size([0.0, -footer])
                .build(ui, || {
                    if self.confirm_close {
                        ui.open_popup(format!("Discard changes?###discard-sheet-{}", self.id));
                        self.confirm_close = false;
                    }
                    self.draw_contents(ui, now_ms, open_files);
                });
            self.log_view.draw(&mut self.log, ui);
        });
        if !open {
            self.request_close();
        }
    }
    fn draw_contents(&mut self, ui: &Ui, now_ms: u64, open_files: &[PathBuf]) {
        let sheet = &self.sheet;
        let memory = matches!(sheet.source, SheetSource::Memory { .. });
        {
            let _disabled = ui.begin_disabled_with_cond(!self.can_save());
            if ui.button("Save") {
                self.pending.push(Action::Save(self.id));
            }
        }
        ui.same_line();
        {
            let _disabled = ui.begin_disabled_with_cond(memory);
            if ui.button("Reload image") {
                self.pending.push(Action::Reload(self.id));
            }
        }
        if memory {
            ui.text("Memory image: save the image first");
            regions::memory_header(self, ui, open_files);
        }
        ui.modal_popup(
            format!("Discard changes?###discard-sheet-{}", self.id),
            || {
                ui.text("Discard unsaved changes?");
                if ui.button("Discard") {
                    self.pending.push(Action::Close(self.id));
                    ui.close_current_popup();
                }
                #[cfg(test)]
                {
                    self.close_controls[1] = tests::item_center(ui);
                }
                ui.same_line();
                if ui.button("Cancel") {
                    ui.close_current_popup();
                }
                #[cfg(test)]
                {
                    self.close_controls[2] = tests::item_center(ui);
                }
            },
        );
        self.grid_panel(ui);
        let available = ui.content_region_avail()[0];
        let side = 300.0_f32.min((available * 0.3).max(120.0));
        let sheet_width = ((available - side - 20.0) * 0.5).max(100.0);
        let middle_width = (available - side - sheet_width - 20.0).max(100.0);
        ui.child_window("Sheet column")
            .size([sheet_width, 0.0])
            .build(ui, || {
                ui.text("Sheet");
                ui.text_disabled("Click: select  Shift: range  Ctrl: toggle");
                let overlays = self.sprite_overlays();
                regions::sheet_view(self, ui, overlays);
            });
        ui.same_line();
        ui.child_window("Editor tabs column")
            .size([middle_width, 0.0])
            .build(ui, || {
                if let Some(_tabs) = ui.tab_bar("Sheet editor tabs") {
                    if let Some(_tab) = ui.tab_item("Tileset") {
                        self.active_tab = EditorTab::Tileset;
                        self.tileset_tab(ui);
                    }
                    if let Some(_tab) = ui.tab_item(format!(
                        "Sprites ({})###sprites",
                        self.sheet.meta.sprites.len()
                    )) {
                        self.active_tab = EditorTab::Sprites;
                        self.sprites_tab(ui);
                    }
                    if let Some(_tab) = ui.tab_item(format!(
                        "Animations ({})###animations",
                        self.sheet.meta.animations.len()
                    )) {
                        self.active_tab = EditorTab::Animations;
                        self.animations_tab(ui);
                    }
                    if let Some(_tab) = ui.tab_item_with_flags(
                        "Import",
                        None,
                        TabItemOptions::new().placement(TabItemPlacement::Trailing),
                    ) {
                        self.active_tab = EditorTab::Import;
                        if ui.collapsing_header("Regions", TreeNodeFlags::empty()) {
                            ui.text_wrapped("Cut regions out of a large picture into new PNGs.");
                            regions::section(self, ui, open_files);
                        }
                        if ui.collapsing_header("Extract tileset", TreeNodeFlags::empty()) {
                            ui.text_wrapped(
                                "Turn a map picture into a tileset PNG and a .map.json.",
                            );
                            extract::section(self, ui, open_files);
                        }
                    }
                }
            });
        ui.same_line();
        ui.child_window("Preview column")
            .size([0.0, 0.0])
            .build(ui, || {
                self.preview_panel(ui, now_ms);
            });
    }
    fn log(&mut self, message: impl Into<String>) {
        log::append(&mut self.log, message.into());
    }
    fn start_guess(&mut self) {
        if self.guess_job.is_some() {
            return;
        }
        self.guess = None;
        match guess::start(self.sheet.image.clone()) {
            Ok(job) => {
                self.guess_job = Some(job);
                self.log("Guess started (0%)");
            }
            Err(error) => log::append(&mut self.log, error),
        }
    }
    fn cancel_guess(&mut self) {
        if self.guess_job.take().is_some() {
            log::append(&mut self.log, "Guess cancelled".into());
        }
    }
    fn poll_guess(&mut self) {
        let Some(job) = &mut self.guess_job else {
            return;
        };
        match job.poll() {
            Ok(Some(result)) => {
                self.log("Guess completed (100%)");
                for score in &result.scores {
                    self.log(format!(
                        "{} px offset ({}, {}): {}/{} unique ({:.3})",
                        score.tile,
                        score.offset.0,
                        score.offset.1,
                        score.unique,
                        score.total,
                        score.ratio
                    ));
                }
                if result.pick.is_none() {
                    self.log("No strong grid suggestion; keep manual values");
                }
                self.guess = Some(result);
                self.guess_job = None;
            }
            Ok(None) => {
                if let Some(percent) = job.next_step() {
                    self.log(format!("Guessing... {percent}%"));
                }
            }
            Err(error) => {
                self.guess_job = None;
                log::append(&mut self.log, error);
            }
        }
    }
    fn grid_panel(&mut self, ui: &Ui) {
        let mut grid = self.sheet.meta.grid;
        let mut changed = false;
        ui.set_next_item_width(130.0);
        changed |= view::input_u32(ui, "Tile", &mut grid.tile, 1);
        ui.same_line();
        ui.set_next_item_width(130.0);
        changed |= view::input_u32(ui, "Offset x", &mut grid.offset[0], 0);
        ui.same_line();
        ui.set_next_item_width(130.0);
        changed |= view::input_u32(ui, "Offset y", &mut grid.offset[1], 0);
        ui.same_line();
        ui.set_next_item_width(130.0);
        changed |= view::input_u32(ui, "Spacing", &mut grid.spacing, 0);
        if let Some(job) = &self.guess_job {
            if view::job_progress(ui, job.fraction(), "Guessing", "Cancel Guess") {
                self.cancel_guess();
            }
        } else if ui.button("Guess") {
            self.start_guess();
        }
        ui.same_line();
        ui.checkbox("Show grid", &mut self.show_grid);
        ui.same_line();
        ui.set_next_item_width(160.0);
        ui.slider("Zoom", 1, 16, &mut self.zoom);
        if let Some(guess) = &self.guess {
            if let Some(pick) = guess.pick {
                if ui.button(format!(
                    "Apply {} px, offset ({}, {})",
                    pick.0, pick.1.0, pick.1.1
                )) {
                    grid = apply_guess(grid, pick);
                    changed = true;
                }
            } else {
                ui.text("No strong grid suggestion; keep manual values");
            }
        }
        if changed {
            match self.sheet.set_grid(grid) {
                Ok(()) => {
                    self.tile_draft = Default::default();
                    self.cancel_analysis();
                    self.extract.invalidate();
                    log::append_repeat(&mut self.log, "Grid updated");
                }
                Err(error) => log::append(&mut self.log, error),
            }
        }
        let cells = self
            .sheet
            .meta
            .grid
            .cells(self.sheet.image.width, self.sheet.image.height);
        ui.text(format!(
            "{} x {} cells, leftover {} x {} px",
            cells.cols, cells.rows, cells.leftover_right, cells.leftover_bottom
        ));
    }
}
fn import_path_has_file_name(input: &str) -> bool {
    let path = input.trim();
    // Path::file_name normalizes a trailing `/` and `/.` away ("a/." names "a"),
    // so the last raw segment is checked before it.
    let last = path.rsplit('/').next().unwrap_or(path);
    !matches!(last, "" | "." | "..") && std::path::Path::new(path).file_name().is_some()
}

impl OpenSheet {
    fn set_grid(&mut self, grid: Grid) -> Result<(), String> {
        let mut candidate = self.meta.clone();
        candidate.grid = grid;
        candidate
            .validate_edit_bounds(self.image.width, self.image.height)
            .map_err(|e| e.join("; "))?;
        self.meta = candidate;
        self.selection = Selection::default();
        self.dirty = true;
        Ok(())
    }
    fn prepare(
        source: SheetSource,
        image: RgbaImage,
        meta: SpriteSheet,
        dirty: bool,
        gl: &Rc<glow::Context>,
        renderer: &mut GlowRenderer,
    ) -> Result<Self, String> {
        meta.validate_bounds(image.width, image.height)
            .map_err(|e| e.join("; "))?;
        let texture = SheetTexture::upload(gl, &image)?;
        let external = renderer
            .register_external_texture(texture.raw())
            .map_err(|e| e.to_string())?;
        Ok(Self {
            source,
            image,
            texture: Some(texture),
            imgui_id: external.texture_id(),
            external: Some(external),
            meta,
            dirty,
            selection: Selection::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dear_imgui_rs::{
        BackendFlags, Context,
        render::{SynchronousRendererConsumer, TextureOp},
    };

    #[cfg(unix)]
    #[test]
    fn load_path_rules_in_their_own_process() {
        use std::{fs, os::unix::fs::symlink};
        // A private cwd exercises the empty parent of "example.png" without
        // changing the process-wide directory used by concurrently running tests.
        const CHILD: &str = "RN_LOAD_PATH_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let fixture = crate::test_support::TempDir::new("extract");
            crate::test_support::in_child_process(
                "ui::asset_library::tests::load_path_rules_in_their_own_process",
                Some(CHILD),
                false,
                Some(&fixture.0),
            );
            return;
        }
        let dir = std::env::current_dir().unwrap();
        let outside = crate::test_support::TempDir::new("extract");
        let (image, mut meta) = crate::sprite::example();
        let png = image.to_png().unwrap();
        let image_path = Path::new("example.png");
        let sidecar = store::sidecar_for(image_path);
        let target = outside.0.join("outside.png");
        fs::write(&target, &png).unwrap();
        symlink(&target, image_path).unwrap();
        store::save(&sidecar, &meta).unwrap();
        let json = fs::read(&sidecar).unwrap();
        let error = resolve_load_path(image_path).unwrap_err();
        assert_eq!(error, "unsafe image path: \"example.png\"");
        assert!(!error.contains(['\n', '\r']));
        assert!(matches!(
            store::load(&sidecar),
            Err(store::StoreError::UnsafeImagePath(_))
        ));
        assert_eq!(fs::read(&target).unwrap(), png);
        assert_eq!(fs::read(&sidecar).unwrap(), json);
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 2);
        assert_eq!(fs::read_dir(&outside.0).unwrap().count(), 1);

        fs::remove_file(image_path).unwrap();
        fs::write("local.png", &png).unwrap();
        symlink("local.png", image_path).unwrap();
        let canonical = dir.join("local.png").canonicalize().unwrap();
        assert_eq!(resolve_load_path(image_path).unwrap(), canonical);
        store::load(&sidecar).unwrap();
        // The resolved path is what Load reads. Replacing the input alias later
        // must not redirect that read to an unchecked outside image.
        let checked = resolve_load_path(image_path).unwrap();
        fs::remove_file(image_path).unwrap();
        symlink(&target, image_path).unwrap();
        assert_eq!(fs::read(&checked).unwrap(), png);
        assert_eq!(checked, canonical);
        assert!(
            resolve_load_path(image_path)
                .unwrap_err()
                .starts_with("unsafe image path:")
        );
        fs::remove_file(image_path).unwrap();
        symlink("local.png", image_path).unwrap();
        // A directory symlink resolves image and parent into the same directory.
        let alias = outside.0.join("directory");
        symlink(&dir, &alias).unwrap();
        assert_eq!(
            resolve_load_path(&alias.join(image_path)).unwrap(),
            canonical
        );
        store::load(&alias.join(&sidecar)).unwrap();

        fs::remove_file(image_path).unwrap();
        fs::write(image_path, &png).unwrap();
        assert_eq!(resolve_load_path(image_path).unwrap(), dir.join(image_path));
        // A missing image is allowed by the metadata loader, but editor Load
        // still fails with the ordinary canonicalize diagnostic.
        meta.image = "missing.png".into();
        store::save(&sidecar, &meta).unwrap();
        store::load(&sidecar).unwrap();
        let missing = Path::new("missing.png");
        assert_eq!(
            resolve_load_path(missing).unwrap_err(),
            missing.canonicalize().unwrap_err().to_string()
        );
    }

    // Observe the actual submitted items, rather than repeating import_panel's
    // layout or enabled condition in a test-only copy of the UI.
    #[derive(Clone, Copy, Debug, Default)]
    pub(super) struct ImportItem {
        min: [f32; 2],
        max: [f32; 2],
        disabled: bool,
    }
    impl ImportItem {
        pub(super) fn last(ui: &Ui) -> Self {
            use dear_imgui_rs::sys;
            Self {
                min: ui.item_rect_min(),
                max: ui.item_rect_max(),
                // SAFETY: called immediately after a widget in an active frame
                // on the subprocess's sole ImGui owning thread.
                disabled: unsafe { sys::igGetItemFlags() & sys::ImGuiItemFlags_Disabled != 0 },
            }
        }
        fn center(self) -> [f32; 2] {
            [
                (self.min[0] + self.max[0]) * 0.5,
                (self.min[1] + self.max[1]) * 0.5,
            ]
        }
    }
    #[derive(Clone, Copy, Default)]
    pub(super) struct ImportControls {
        pub input: ImportItem,
        pub load: ImportItem,
        pub browse: ImportItem,
        pub refresh: ImportItem,
        pub asset: Option<ImportItem>,
    }

    const IMPORT_PATH_CASES: &[(&str, Option<&str>)] = &[
        ("", None),
        ("  ", None),
        ("\t\n", None),
        ("/", None),
        (".", None),
        ("..", None),
        ("assets/", None),
        (" assets/ \t", None),
        ("assets/.", None),
        ("/tmp/a.png/.", None),
        ("assets/..", None),
        ("a.png", Some("a.png")),
        ("assets/a.png", Some("assets/a.png")),
        ("/tmp/a.png", Some("/tmp/a.png")),
        ("  assets/a.png \t", Some("assets/a.png")),
    ];

    fn click(context: &mut Context, render: impl Fn(&mut Context), point: [f32; 2]) {
        context.io_mut().add_mouse_pos_event(point);
        render(context);
        context
            .io_mut()
            .add_mouse_button_event(dear_imgui_rs::MouseButton::Left, true);
        render(context);
        context
            .io_mut()
            .add_mouse_button_event(dear_imgui_rs::MouseButton::Left, false);
        render(context);
    }

    #[test]
    fn import_panel_in_its_own_process() {
        import_panel_clicks_and_layout();
    }

    #[test]
    #[ignore = "run by import_panel_in_its_own_process with isolated assets fixture"]
    fn import_panel_clicks_and_layout() {
        use std::cell::{Cell, RefCell};
        // Both the normal wrapper and bulk --ignored runs enter here. Only
        // the exact-test child owns an ImGui context and scans the fixture.
        const CHILD: &str = "RN_IMPORT_PANEL_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let fixture = crate::test_support::TempDir::new("extract");
            std::fs::create_dir(fixture.0.join("assets")).unwrap();
            // Refresh only lists the name; no PNG decoding, GL or local assets.
            std::fs::write(fixture.0.join("assets/from-refresh.png"), []).unwrap();
            crate::test_support::in_child_process(
                "ui::asset_library::tests::import_panel_clicks_and_layout",
                Some(CHILD),
                true,
                Some(&fixture.0),
            );
            return;
        }
        let (mut context, consumer) = context();
        let library = RefCell::new(AssetLibrary::default());
        let content_right = Cell::new(0.0);
        for width in [320.0, 620.0, 900.0] {
            let render = |context: &mut Context| {
                vertices(context, &consumer, |ui| {
                    ui.window("Import panel regression")
                        .position([20.0, 20.0], Condition::Always)
                        .size([width, 260.0], Condition::Always)
                        .build(|| {
                            content_right
                                .set(ui.cursor_screen_pos()[0] + ui.content_region_avail()[0]);
                            library.borrow_mut().import_panel(ui);
                        });
                });
            };
            for &(input, expected) in IMPORT_PATH_CASES {
                library.borrow_mut().path_input = input.into();
                library.borrow_mut().assets = vec!["from-list.png".into()];
                render(&mut context);
                let items = library.borrow().import_controls;
                assert_eq!(
                    items.load.disabled,
                    expected.is_none(),
                    "Load disabled: {input:?}, width={width}"
                );
                for (name, item) in [
                    ("PNG path", items.input),
                    ("Browse", items.browse),
                    ("Refresh", items.refresh),
                    ("asset", items.asset.expect("fixture asset was submitted")),
                ] {
                    assert!(!item.disabled, "{name} disabled: {input:?}, width={width}");
                }
                assert_eq!(
                    items.input.min[1], items.load.min[1],
                    "Load shares input row"
                );
                assert!(
                    items.load.min[0] > items.input.max[0],
                    "Load follows path label"
                );
                assert!(
                    items.load.max[0] <= content_right.get() + 1.0,
                    "Load fits content: {:?}, right={}, width={width}",
                    items.load,
                    content_right.get()
                );
                assert!(
                    items.browse.min[1] > items.load.max[1],
                    "Browse is on next row"
                );
                assert_eq!(
                    items.browse.min[1], items.refresh.min[1],
                    "Refresh shares Browse row"
                );
                assert!(
                    items.refresh.min[0] > items.browse.max[0],
                    "Refresh follows Browse"
                );

                click(&mut context, render, items.load.center());
                let pending = std::mem::take(&mut library.borrow_mut().pending);
                match expected {
                    None => assert!(pending.is_empty(), "disabled Load queued: {input:?}"),
                    Some(path) => assert!(
                        matches!(&pending[..], [Action::Load(actual)] if actual == &PathBuf::from(path)),
                        "Load queues exactly one trimmed path: {input:?}, width={width}"
                    ),
                }
            }
            // A disabled Load must leave the following row and child list usable.
            // Do not click Browse: that would open a native OS file dialog.
            library.borrow_mut().path_input.clear();
            render(&mut context);
            let refresh = library.borrow().import_controls.refresh.center();
            click(&mut context, render, refresh);
            assert_eq!(
                library.borrow().assets,
                [std::env::current_dir()
                    .unwrap()
                    .join("assets/from-refresh.png")],
                "Refresh really scans the isolated fixture while Load is disabled"
            );
            assert!(library.borrow().pending.is_empty());
            let asset = library.borrow().import_controls.asset.unwrap().center();
            click(&mut context, render, asset);
            let pending = std::mem::take(&mut library.borrow_mut().pending);
            assert!(
                matches!(&pending[..], [Action::Load(path)] if path == &library.borrow().assets[0]),
                "asset selection queues its own path while Load is disabled"
            );
        }
        context
            .prepare_renderer_texture_reset(&consumer)
            .unwrap()
            .commit();
        drop(consumer);
        context.poll_snapshot_completions().unwrap();
    }

    #[test]
    fn import_path_requires_a_file_name() {
        for &(path, expected) in IMPORT_PATH_CASES {
            assert_eq!(
                import_path_has_file_name(path),
                expected.is_some(),
                "{path:?}"
            );
        }
    }

    fn fake_sheet() -> OpenSheet {
        let (image, meta) = crate::sprite::example();
        OpenSheet {
            source: SheetSource::File("example.png".into()),
            image,
            meta,
            texture: None,
            imgui_id: TextureId::new(1),
            external: None,
            dirty: false,
            selection: Default::default(),
        }
    }
    #[test]
    fn load_preparation_checks_paths_before_focus_without_gl() {
        use std::fs;
        let temp = crate::test_support::TempDir::new("load-preparation");
        let outside = crate::test_support::TempDir::new("load-aliases");
        let png = crate::sprite::example().0.to_png().unwrap();
        let first = temp.0.join("first.png");
        let second = temp.0.join("second.png");
        fs::write(&first, &png).unwrap();
        fs::write(&second, &png).unwrap();
        fs::create_dir(temp.0.join("alias")).unwrap();
        #[cfg(unix)]
        for (target, name) in [(&first, "opened\n\r.png"), (&second, "unopened.png")] {
            std::os::unix::fs::symlink(target, outside.0.join(name)).unwrap();
        }
        let entries = (
            fs::read_dir(&temp.0).unwrap().count(),
            fs::read_dir(&outside.0).unwrap().count(),
        );
        let mut library = AssetLibrary {
            path_input: "keep this user input".into(),
            status: "before preparation".into(),
            ..Default::default()
        };
        let mut sheet = fake_sheet();
        sheet.source = SheetSource::File(first.canonicalize().unwrap());
        library.install(sheet);
        library.sheets[0].focus_requested = false;
        let snapshot = |library: &AssetLibrary| {
            (
                library.next_id,
                library.path_input.clone(),
                library.status.clone(),
                library.pending.len(),
                library
                    .sheets
                    .iter()
                    .map(|editor| {
                        (
                            editor.id,
                            editor.focus_requested,
                            match &editor.sheet.source {
                                SheetSource::File(path) => Some(path.clone()),
                                SheetSource::Memory { .. } => None,
                            },
                            editor.sheet.meta.clone(),
                            editor.sheet.dirty,
                            editor.sheet.imgui_id,
                            editor.sheet.external,
                            editor.sheet.texture.is_some(),
                            editor.sheet.image.clone(),
                            editor.sheet.selection.clone(),
                            editor.log.clone(),
                        )
                    })
                    .collect::<Vec<_>>(),
            )
        };
        let before = snapshot(&library);
        // This is the preparation used by Action::Load. Bypassing it in that arm
        // still needs the GL queue test; this test observes the preparation itself.
        // The arm moves its path in, so it cannot open the input instead of `Open`.
        #[cfg(unix)]
        for name in ["opened\n\r.png", "unopened.png"] {
            let error = library.begin_load(outside.0.join(name)).unwrap_err();
            assert!(error.starts_with("unsafe image path:"), "{error}");
            assert!(error.is_ascii() && !error.contains(['\n', '\r']));
            assert_eq!(snapshot(&library), before, "refusal must precede focus");
        }
        let missing = temp.0.join("missing.png");
        assert_eq!(
            library.begin_load(missing.clone()).unwrap_err(),
            missing.canonicalize().unwrap_err().to_string()
        );
        assert_eq!(snapshot(&library), before);
        // An alias of an unopened file opens the checked path, not the input.
        for given in [second.clone(), temp.0.join("alias/../second.png")] {
            assert_eq!(
                library.begin_load(given).unwrap(),
                LoadStep::Open(second.canonicalize().unwrap())
            );
            assert_eq!(snapshot(&library), before);
        }
        assert_eq!(
            library
                .begin_load(temp.0.join("alias/../first.png"))
                .unwrap(),
            LoadStep::Focused
        );
        assert!(library.sheets[0].focus_requested);
        library.sheets[0].focus_requested = false;
        assert_eq!(snapshot(&library), before, "only focus may change");
        assert_eq!(
            (
                fs::read_dir(&temp.0).unwrap().count(),
                fs::read_dir(&outside.0).unwrap().count(),
            ),
            entries
        );
        for path in [&first, &second] {
            assert_eq!(fs::read(path).unwrap(), png);
            assert!(!store::sidecar_for(path).exists());
        }
        #[cfg(unix)]
        for (target, name) in [(&first, "opened\n\r.png"), (&second, "unopened.png")] {
            assert_eq!(fs::read_link(outside.0.join(name)).unwrap(), *target);
            assert!(!store::sidecar_for(&outside.0.join(name)).exists());
        }
        extract::tests::assert_no_temps(&temp.0);
        extract::tests::assert_no_temps(&outside.0);
    }
    pub(super) fn context() -> (Context, SynchronousRendererConsumer) {
        let mut context = crate::dev::context().unwrap();
        context.io_mut().set_display_size([2000.0, 1100.0]);
        context.io_mut().set_delta_time(1.0 / 60.0);
        context
            .io_mut()
            .set_backend_flags(BackendFlags::RENDERER_HAS_TEXTURES);
        let consumer = context.create_synchronous_renderer_consumer().unwrap();
        (context, consumer)
    }
    pub(super) fn vertices(
        context: &mut Context,
        consumer: &SynchronousRendererConsumer,
        draw: impl Fn(&Ui),
    ) -> usize {
        let mut count = 0;
        for _ in 0..4 {
            draw(context.frame());
            let pending = context.render(consumer);
            let feedback = pending
                .texture_requests()
                .iter()
                .map(|request| match request.operation() {
                    TextureOp::Create { .. } | TextureOp::Update { .. } => {
                        request.uploaded(TextureId::new(2)).unwrap()
                    }
                    TextureOp::Destroy => request.destroyed().unwrap(),
                })
                .collect::<Vec<_>>();
            count = pending
                .reconcile_texture_feedback(feedback)
                .unwrap()
                .draw_data()
                .total_vtx_count();
        }
        count
    }
    pub(super) fn item_center(ui: &Ui) -> [f32; 2] {
        let min = ui.item_rect_min();
        let size = ui.item_rect_size();
        [min[0] + size[0] * 0.5, min[1] + size[1] * 0.5]
    }
    fn type_into(context: &mut Context, render: impl Fn(&mut Context), at: [f32; 2], text: &str) {
        click(context, &render, at);
        for ch in text.chars() {
            context.io_mut().add_input_character(ch);
            render(context);
        }
    }
    fn check_tile_log_input(context: &mut Context, consumer: &SynchronousRendererConsumer) {
        use dear_imgui_rs::Condition;
        use std::cell::RefCell;
        let sheet = || {
            let mut sheet = fake_sheet();
            sheet.meta.tiles.clear();
            sheet.meta.sprites.clear();
            sheet.meta.animations[0].frames.clear();
            sheet.selection.apply(Some((0, 0)), false, false);
            sheet
        };
        let editor = RefCell::new(SheetEditor::new(20, sheet()));
        // Drawn in the same frames with the same fields, so a run or a keystroke
        // that leaked across sheets would land here.
        let other = RefCell::new(SheetEditor::new(21, sheet()));
        let render = |context: &mut Context| {
            vertices(context, consumer, |ui| {
                for (target, y) in [(&editor, 40.0), (&other, 300.0)] {
                    let id = target.borrow().id;
                    ui.window(format!("Tile input test {id}"))
                        .position([40.0, y], Condition::Always)
                        .size([900.0, 220.0], Condition::Always)
                        .build(|| target.borrow_mut().tileset_tab(ui));
                }
            });
        };
        render(context);
        let at = editor.borrow().tile_name_input;
        type_into(context, render, at, "abcdefghijkl");
        let editor = editor.borrow();
        assert_eq!(
            editor.sheet.meta.tiles[0].name.as_deref(),
            Some("abcdefghijkl")
        );
        assert!(editor.sheet.dirty);
        assert_eq!(editor.log.len(), 1, "each keystroke belongs to one run");
        assert!(
            editor.log[0].ends_with("Tile metadata updated (x12)"),
            "{:?}",
            editor.log
        );
        let other = other.borrow();
        assert!(other.log.is_empty(), "other sheets have independent logs");
        assert!(other.sheet.meta.tiles.is_empty() && !other.sheet.dirty);
    }
    fn check_tile_properties_and_marker(
        context: &mut Context,
        consumer: &SynchronousRendererConsumer,
    ) {
        use std::cell::RefCell;
        let editor = RefCell::new(SheetEditor::new(23, fake_sheet()));
        editor
            .borrow_mut()
            .sheet
            .selection
            .apply(Some((0, 0)), false, false);
        let render = |context: &mut Context| {
            vertices(context, consumer, |ui| {
                ui.window("Tile props test")
                    .position([40.0, 40.0], Condition::Always)
                    .size([700.0, 400.0], Condition::Always)
                    .build(|| editor.borrow_mut().tileset_tab(ui));
            });
        };
        render(context);
        let add = editor.borrow().tile_draft.props.add_button.unwrap();
        click(context, render, add);
        let key = editor.borrow().tile_draft.props.key_input.unwrap();
        type_into(context, render, key, "abc");
        assert_eq!(editor.borrow().tile_draft.props.rows[0].0, "abc");
        assert_eq!(
            editor.borrow().sheet.meta.tiles[0]
                .props
                .get("abc")
                .map(String::as_str),
            Some("")
        );
        editor.borrow_mut().tile_name_input = [0.0; 2];
        editor
            .borrow_mut()
            .sheet
            .selection
            .apply(Some((1, 0)), false, true);
        render(context);
        assert_eq!(editor.borrow().tile_name_input, [0.0; 2]);
        let marked = RefCell::new(SheetEditor::new(24, fake_sheet()));
        marked.borrow_mut().show_grid = false;
        let draw = |context: &mut Context| {
            vertices(context, consumer, |ui| {
                ui.window("Marker test")
                    .position([40.0, 40.0], Condition::Always)
                    .size([400.0, 300.0], Condition::Always)
                    .build(|| marked.borrow_mut().sheet_view(ui, &[]));
            })
        };
        draw(context);
        let without = draw(context);
        marked
            .borrow_mut()
            .sheet
            .meta
            .tiles
            .push(crate::sprite::format::TileMeta {
                col: 0,
                row: 0,
                name: Some("marked".into()),
                tags: vec![],
                props: Default::default(),
            });
        let with = draw(context);
        assert!(with >= without + 4, "{without} {with}");
    }
    fn check_sprite_list_and_creation(
        context: &mut Context,
        consumer: &SynchronousRendererConsumer,
    ) {
        use std::cell::RefCell;
        let mut sheet = fake_sheet();
        sheet.image = RgbaImage::new(160, 64);
        let editor = RefCell::new(SheetEditor::new(25, sheet));
        editor.borrow_mut().now = || 777;
        let render = |context: &mut Context| {
            vertices(context, consumer, |ui| {
                ui.window("Sprite create test")
                    .position([40.0, 40.0], Condition::Always)
                    .size([900.0, 600.0], Condition::Always)
                    .build(|| editor.borrow_mut().sprites_tab(ui));
            });
        };
        render(context);
        let button = editor.borrow().sprite_new_input;
        let count = editor.borrow().sheet.meta.sprites.len();
        click(context, render, button);
        assert_eq!(editor.borrow().sheet.meta.sprites.len(), count);
        assert!(editor.borrow().log.is_empty());
        {
            let mut e = editor.borrow_mut();
            e.sheet.selection.cells = [(1, 2), (3, 3)].into_iter().collect();
        }
        click(context, render, button);
        {
            let e = editor.borrow();
            let sprite = e.sheet.meta.sprites.last().unwrap();
            assert_eq!(sprite.pivot, [24.0, 32.0]);
            assert!(matches!(
                sprite.shape,
                crate::sprite::format::Shape::Set { canvas: [3, 2], .. }
            ));
            assert_eq!(sprite.modified, Some(777));
        }
        {
            let mut e = editor.borrow_mut();
            e.sheet.selection.cells = [(0, 0), (8, 0)].into_iter().collect();
        }
        click(context, render, button);
        assert_eq!(editor.borrow().sheet.meta.sprites.len(), count + 1);
        assert!(
            editor
                .borrow()
                .log
                .iter()
                .any(|s| s.contains("at most 8 x 8"))
        );
        render(context);
        let add = editor.borrow().sprite_props.add_button.unwrap();
        click(context, render, add);
        let key = editor.borrow().sprite_props.key_input.unwrap();
        type_into(context, render, key, "key");
        let value = editor.borrow().sprite_props.value_input.unwrap();
        type_into(context, render, value, "value");
        {
            let e = editor.borrow();
            let sprite = e.sheet.meta.sprites.last().unwrap();
            assert_eq!(sprite.props.get("key").map(String::as_str), Some("value"));
            assert_eq!(sprite.modified, Some(777));
        }
        let list = RefCell::new(SheetEditor::new(26, fake_sheet()));
        let count_vertices = |context: &mut Context, n: usize| {
            {
                let mut e = list.borrow_mut();
                let template = e.sheet.meta.sprites[0].clone();
                e.sheet.meta.sprites = (0..n)
                    .map(|i| {
                        let mut s = template.clone();
                        s.id = i as u64 + 1;
                        s.name = format!("sprite-{i}");
                        s
                    })
                    .collect();
                e.selected_sprite = None;
            }
            vertices(context, consumer, |ui| {
                ui.window("Sprite list test")
                    .position([40.0, 40.0], Condition::Always)
                    .size([700.0, 400.0], Condition::Always)
                    .build(|| list.borrow_mut().sprites_tab(ui));
            })
        };
        count_vertices(context, 20);
        let small = count_vertices(context, 20);
        let large = count_vertices(context, 1000);
        assert!(large <= small + 100, "20 rows: {small}, 1000 rows: {large}");
    }
    fn check_sprite_rename_log_input(
        context: &mut Context,
        consumer: &SynchronousRendererConsumer,
    ) {
        use dear_imgui_rs::Condition;
        let mut editor = SheetEditor::new(22, fake_sheet());
        editor.selected_sprite = Some(1);
        editor.preview.target = preview::PreviewTarget::Sprite(1);
        editor.now = || 12345;
        let animation_before = editor.sheet.meta.animations[0].clone();
        let editor = std::cell::RefCell::new(editor);
        let render = |context: &mut Context| {
            vertices(context, consumer, |ui| {
                ui.window("Sprite input test")
                    .position([40.0, 40.0], Condition::Always)
                    .size([900.0, 400.0], Condition::Always)
                    .build(|| editor.borrow_mut().sprites_tab(ui));
            });
        };
        render(context);
        let at = editor.borrow().sprite_name_input;
        type_into(context, render, at, "mnopqrstuv");
        let editor = editor.borrow();
        assert_eq!(editor.sheet.meta.sprites[0].name, "amnopqrstuv");
        assert_eq!(editor.sheet.meta.animations[0], animation_before);
        assert_eq!(editor.selected_sprite, Some(1));
        assert_eq!(editor.preview.target, preview::PreviewTarget::Sprite(1));
        assert_eq!(editor.sheet.meta.sprites[0].modified, Some(12345));
        assert_eq!(editor.log.len(), 1, "{:?}", editor.log);
        assert!(
            editor.log[0].ends_with("Sprite renamed (x10)"),
            "{:?}",
            editor.log
        );
    }
    fn check_window_close_interactions(
        context: &mut Context,
        consumer: &SynchronousRendererConsumer,
    ) {
        use std::cell::RefCell;
        let library = RefCell::new(AssetLibrary::default());
        library.borrow_mut().sheets.extend([
            SheetEditor::new(10, fake_sheet()),
            SheetEditor::new(11, fake_sheet()),
        ]);
        library.borrow_mut().sheets[0].sheet.dirty = true;
        let render = |context: &mut Context| {
            vertices(context, consumer, |ui| library.borrow_mut().draw(ui, 0));
        };
        render(context);
        library.borrow_mut().sheets[0].focus_requested = true;
        render(context);
        let first_x = library.borrow().sheets[0].close_controls[0];
        click(context, render, first_x);
        assert!(
            library.borrow().pending.is_empty(),
            "dirty x waits for confirmation"
        );
        let cancel = library.borrow().sheets[0].close_controls[2];
        assert!(cancel[0] > 0.0, "the discard modal was rendered");
        click(context, render, cancel);
        assert!(
            library.borrow().pending.is_empty(),
            "Cancel keeps the editor"
        );
        click(context, render, first_x);
        let discard = library.borrow().sheets[0].close_controls[1];
        click(context, render, discard);
        assert!(
            matches!(library.borrow().pending[..], [Action::Close(10)]),
            "Discard targets its sheet"
        );
        library.borrow_mut().pending.clear();
        library.borrow_mut().sheets.remove(0);
        render(context);
        let second_x = library.borrow().sheets[0].close_controls[0];
        click(context, render, second_x);
        assert!(
            matches!(library.borrow().pending[..], [Action::Close(11)]),
            "clean x closes immediately"
        );
    }
    #[test]
    fn invalid_tile_properties_prevent_save() {
        let temp = crate::test_support::TempDir::new("invalid-tile-props");
        let image = temp.0.join("example.png");
        let sidecar = store::sidecar_for(&image);
        std::fs::write(&sidecar, "unchanged").unwrap();
        let mut sheet = fake_sheet();
        sheet.source = SheetSource::File(image);
        sheet.dirty = true;
        let mut editor = SheetEditor::new(100, sheet);
        editor.tile_draft.cell = Some((0, 0));
        editor
            .tile_draft
            .props
            .rows
            .push((" ".into(), "value".into()));
        assert!(editor.save().unwrap_err().contains("property row 1"));
        assert_eq!(std::fs::read_to_string(sidecar).unwrap(), "unchanged");
    }
    #[test]
    fn install_warns_after_open_and_preserves_dirty() {
        let mut library = AssetLibrary::default();
        let mut sheet = fake_sheet();
        sheet.dirty = true;
        library.install_with_warning(sheet, &[(1, 0), (2, 0)]);
        let editor = &library.sheets[0];
        assert!(editor.sheet.dirty);
        assert!(editor.log[0].contains("Opened"));
        assert!(editor.log[1].contains("(1, 0)") && editor.log[1].contains("(2, 0)"));
    }
    #[test]
    fn incomplete_animation_does_not_block_grid_edit() {
        let mut sheet = fake_sheet();
        sheet.meta.animations[0].frames.clear();
        assert!(sheet.meta.validate().is_err());
        assert!(sheet.set_grid(sheet.meta.grid).is_ok());
        assert!(sheet.meta.validate().is_err());
    }
    #[test]
    fn invalid_grid_edit_preserves_metadata_and_selection() {
        let mut sheet = fake_sheet();
        sheet.selection.apply(Some((2, 0)), false, false);
        let original = sheet.meta.clone();
        assert!(
            sheet
                .set_grid(Grid {
                    tile: 32,
                    offset: [0, 0],
                    spacing: 0
                })
                .is_err()
        );
        assert_eq!(sheet.meta, original);
        assert_eq!(sheet.selection.single(), Some((2, 0)));
        assert!(!sheet.dirty);
        assert!(
            sheet
                .set_grid(Grid {
                    tile: 0,
                    offset: [0, 0],
                    spacing: 0
                })
                .is_err()
        );
    }
    #[test]
    fn cancelled_guess_cannot_install_a_late_result_and_editors_are_independent() {
        let mut first = SheetEditor::new(1, fake_sheet());
        let second = SheetEditor::new(2, fake_sheet());
        first.start_guess();
        assert!(first.guess_job.is_some());
        assert!(second.guess_job.is_none());
        first.cancel_guess();
        assert!(first.log.iter().any(|line| line.contains("Guess started")));
        assert!(
            first
                .log
                .iter()
                .any(|line| line.contains("Guess cancelled"))
        );
        assert!(first.guess_job.is_none());
        first.poll_guess();
        assert!(first.guess.is_none());
        first.start_guess();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while first.guess_job.is_some() {
            first.poll_guess();
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert_eq!(
            first.guess,
            Some(raster::guess_grid(
                &first.sheet.image,
                &raster::DEFAULT_GUESS_CANDIDATES,
                true
            ))
        );
        assert!(
            first
                .log
                .iter()
                .any(|line| line.contains("Guess completed (100%)"))
        );
        assert!(first.log.iter().any(|line| line.contains("unique (")));
        assert!(second.log.is_empty());
        assert!(second.guess.is_none());
    }
    #[test]
    fn save_state_tracks_edits_success_failure_and_memory_images() {
        let temp = crate::test_support::TempDir::new("extract");
        let path = temp.0.join("example.png");
        let (image, meta) = crate::sprite::example();
        let bytes = image.to_png().unwrap();
        let (_, _, dirty) = open_sheet(&path, &bytes, None, Some(meta.clone())).unwrap();
        let mut sheet = fake_sheet();
        sheet.source = SheetSource::File(path.clone());
        sheet.dirty = dirty;
        let mut editor = SheetEditor::new(0, sheet);
        assert!(!editor.can_save());
        let mut grid = editor.sheet.meta.grid;
        grid.tile = 8;
        editor.sheet.set_grid(grid).unwrap();
        assert!(editor.can_save());
        assert!(editor.window_title().contains(" * - Sheet Editor###"));
        editor.save().unwrap();
        assert!(!editor.can_save());
        assert!(!editor.window_title().contains(" * - Sheet Editor###"));
        assert_eq!(
            store::load(&store::sidecar_for(&path)).unwrap().sheet.grid,
            grid
        );
        editor.sheet.dirty = true;
        editor.sheet.meta.grid.tile = 0;
        assert!(editor.save().is_err());
        assert!(editor.can_save(), "failed validation leaves Save enabled");
        let (_, _, dirty) = open_sheet(&path, &bytes, None, None).unwrap();
        assert!(dirty, "a PNG without a sidecar starts dirty");
        editor.sheet.source = SheetSource::Memory {
            name: "crop".into(),
        };
        assert!(
            !editor.can_save(),
            "memory images must be saved as PNG first"
        );
    }
    #[test]
    fn title_and_close_request_keep_sheet_identity_and_dirty_changes() {
        let mut editor = SheetEditor::new(42, fake_sheet());
        let clean_title = editor.window_title();
        assert!(clean_title.ends_with(" - Sheet Editor###sheet-42"));
        editor.request_close();
        assert!(matches!(editor.pending[..], [Action::Close(42)]));
        assert!(!editor.confirm_close);
        editor.pending.clear();
        editor.sheet.dirty = true;
        assert_eq!(
            clean_title.split("###").last(),
            editor.window_title().split("###").last()
        );
        assert!(editor.window_title().contains(" * - Sheet Editor###"));
        editor.request_close();
        assert!(editor.confirm_close);
        assert!(editor.pending.is_empty(), "dirty x waits for Discard");
    }
    #[test]
    fn editor_output_moves_into_the_library_after_drawing() {
        let mut library = AssetLibrary::default();
        library.sheets.push(SheetEditor::new(7, fake_sheet()));
        library.status = "old".into();
        library.collect_editor_output();
        assert_eq!(
            library.status, "old",
            "an editor with nothing to say keeps the line"
        );
        let editor = &mut library.sheets[0];
        editor.log("Grid updated");
        editor.pending.push(Action::Save(7));
        editor.saved_path = Some("assets/sprites/map/map.png".into());
        library.collect_editor_output();
        assert_eq!(
            library.status, "old",
            "sheet logs do not replace Library status"
        );
        assert!(matches!(library.pending[..], [Action::Save(7)]));
        assert_eq!(library.path_input, "assets/sprites/map/map.png");
        let editor = &library.sheets[0];
        assert!(editor.log.last().unwrap().ends_with("Grid updated"));
        assert!(editor.pending.is_empty());
        assert!(editor.saved_path.is_none());
    }
    #[test]
    fn library_geometry_in_its_own_process() {
        // Dear ImGui permits one owning thread per process. Keep the original
        // UI test unchanged and run this second context in a separate process.
        hidden_library_grid_and_zoom_geometry();
    }
    #[test]
    #[ignore = "run by library_geometry_in_its_own_process"]
    fn hidden_library_grid_and_zoom_geometry() {
        // Both the default wrapper and bulk --ignored runs enter here. Only
        // the exact-test child owns an ImGui context.
        if !crate::test_support::in_child_process(
            "ui::asset_library::tests::hidden_library_grid_and_zoom_geometry",
            Some("RN_LIBRARY_GEOMETRY_CHILD"),
            true,
            None,
        ) {
            return;
        }
        // Same serialized CPU ImGui context style as the existing UI tests.
        let (mut context, consumer) = context();
        check_tile_log_input(&mut context, &consumer);
        check_tile_properties_and_marker(&mut context, &consumer);
        check_sprite_list_and_creation(&mut context, &consumer);
        check_sprite_rename_log_input(&mut context, &consumer);
        let library = std::cell::RefCell::new(AssetLibrary::default());
        assert!(!library.borrow().open);
        assert_eq!(
            vertices(&mut context, &consumer, |ui| library
                .borrow_mut()
                .draw(ui, 0)),
            0
        );
        library.borrow_mut().open = true;
        library.borrow_mut().assets_scanned = true;
        let empty = vertices(&mut context, &consumer, |ui| {
            library.borrow_mut().draw(ui, 0)
        });
        assert!(empty > 0);
        library
            .borrow_mut()
            .sheets
            .push(SheetEditor::new(0, fake_sheet()));
        library.borrow_mut().sheets[0].show_grid = false;
        let no_grid = vertices(&mut context, &consumer, |ui| {
            library.borrow_mut().draw(ui, 0)
        });
        library.borrow_mut().sheets[0].show_grid = true;
        let grid = vertices(&mut context, &consumer, |ui| {
            library.borrow_mut().draw(ui, 0)
        });
        assert!(grid > no_grid, "grid={grid}, no_grid={no_grid}");
        library.borrow_mut().sheets[0].zoom = 2;
        let size = std::cell::Cell::new([0.0; 2]);
        vertices(&mut context, &consumer, |ui| {
            ui.window("image dimensions").build(|| {
                let lib = library.borrow();
                let editor = &lib.sheets[0];
                let sheet = &editor.sheet;
                ui.image(
                    sheet.imgui_id,
                    [
                        sheet.image.width as f32 * editor.zoom as f32,
                        sheet.image.height as f32 * editor.zoom as f32,
                    ],
                );
                size.set(ui.item_rect_size());
            });
        });
        assert_eq!(size.get(), [128.0, 32.0]);
        library.borrow_mut().open = false;
        assert!(
            vertices(&mut context, &consumer, |ui| library
                .borrow_mut()
                .draw(ui, 0))
                > 0,
            "sheet windows survive closing the library"
        );
        let view = preview::PartView {
            id: TextureId::new(1),
            grid: crate::sprite::example().1.grid,
            width: 64,
            height: 16,
        };
        let one = [crate::sprite::format::Part {
            col: 0,
            row: 0,
            dx: 0,
            dy: 0,
            ..Default::default()
        }];
        let two = [
            one[0],
            crate::sprite::format::Part {
                col: 1,
                row: 0,
                dx: 16,
                dy: 0,
                ..Default::default()
            },
        ];
        let measure_parts = |context: &mut Context, parts: &[crate::sprite::format::Part]| {
            vertices(context, &consumer, |ui| {
                ui.window("Preview geometry").build(|| {
                    let origin = ui.cursor_screen_pos();
                    preview::draw_parts(ui, &view, parts, origin, 2);
                    ui.dummy([100.0, 100.0]);
                });
            })
        };
        let baseline = measure_parts(&mut context, &[]);
        let single = measure_parts(&mut context, &one);
        let double = measure_parts(&mut context, &two);
        assert!(
            single >= baseline + 4 && double >= single + 4,
            "baseline={baseline}, one={single}, two={double}"
        );
        let transition = std::cell::RefCell::new(SheetEditor::new(71, fake_sheet()));
        transition.borrow_mut().sheet.selection.cells.insert((0, 0));
        let draw_transition = |context: &mut Context| {
            vertices(context, &consumer, |ui| {
                ui.window("Transition test")
                    .position([40.0, 40.0], Condition::Always)
                    .size([1200.0, 700.0], Condition::Always)
                    .build(|| transition.borrow_mut().draw_contents(ui, 0, &[]));
            })
        };
        draw_transition(&mut context);
        {
            let mut editor = transition.borrow_mut();
            editor.active_tab = EditorTab::Animations;
            editor.selected_animation = Some(5);
        }
        draw_transition(&mut context);
        assert_eq!(transition.borrow().active_tab, EditorTab::Tileset);
        assert_eq!(
            transition.borrow().preview.target,
            preview::PreviewTarget::Selection
        );
        // Each tab paints an image when its target exists; empty targets paint only text.
        let editor = std::cell::RefCell::new(SheetEditor::new(70, fake_sheet()));
        let measure_preview = |context: &mut Context, tab: EditorTab, selected: bool| {
            {
                let mut editor = editor.borrow_mut();
                editor.active_tab = tab;
                editor.preview.drawn_parts = 0;
                editor.sheet.selection.cells.clear();
                editor.selected_sprite = None;
                editor.selected_animation = None;
                if selected {
                    match tab {
                        EditorTab::Tileset | EditorTab::Import => {
                            editor.sheet.selection.cells.insert((0, 0));
                        }
                        EditorTab::Sprites => editor.selected_sprite = Some(1),
                        EditorTab::Animations => editor.selected_animation = Some(5),
                    }
                }
            }
            vertices(context, &consumer, |ui| {
                ui.window("Preview panel test")
                    .position([40.0, 40.0], Condition::Always)
                    .size([400.0, 500.0], Condition::Always)
                    .build(|| editor.borrow_mut().preview_panel(ui, 0));
            })
        };
        for tab in [
            EditorTab::Tileset,
            EditorTab::Import,
            EditorTab::Sprites,
            EditorTab::Animations,
        ] {
            let _ = measure_preview(&mut context, tab, false);
            let empty = measure_preview(&mut context, tab, false);
            let with_target = measure_preview(&mut context, tab, true);
            assert_eq!(editor.borrow().preview.drawn_parts, 1);
            assert!(
                with_target != empty,
                "{tab:?}: target={with_target}, empty={empty}"
            );
        }
        {
            let mut e = editor.borrow_mut();
            e.active_tab = EditorTab::Tileset;
            e.sheet.selection.cells = (0..257).map(|col| (col, 0)).collect();
        }
        vertices(&mut context, &consumer, |ui| {
            ui.window("Preview panel test")
                .position([40.0, 40.0], Condition::Always)
                .size([400.0, 500.0], Condition::Always)
                .build(|| editor.borrow_mut().preview_panel(ui, 0));
        });
        assert_eq!(editor.borrow().preview.drawn_parts, 0);
        check_window_close_interactions(&mut context, &consumer);
        log::LogView::check_interactions(&mut context, &consumer);
        context
            .prepare_renderer_texture_reset(&consumer)
            .unwrap()
            .commit();
        drop(consumer);
        context.poll_snapshot_completions().unwrap();
    }
    #[test]
    #[ignore = "requires a display; run under Xvfb"]
    fn queued_replacement_registration_and_close() {
        if !crate::test_support::in_child_process(
            "ui::asset_library::tests::queued_replacement_registration_and_close",
            Some("RN_LIBRARY_GL_CHILD"),
            true,
            None,
        ) {
            return;
        }
        let fixture = crate::test_support::HiddenGl::new("library lifetime test");
        let gl = std::rc::Rc::clone(&fixture.gl);
        let mut imgui = crate::dev::context().unwrap();
        let mut renderer = GlowRenderer::with_shared_context(Rc::clone(&gl), &mut imgui).unwrap();
        let mut library = AssetLibrary::default();
        let default_grid = Grid::DEFAULT;
        library.pending.push(Action::Replace {
            image: crate::sprite::example().0,
            name: "cropped".into(),
            grid: default_grid,
        });
        library.apply_pending(&gl, &mut renderer);
        let sheet = &mut library.sheets[0].sheet;
        assert!(matches!(&sheet.source,SheetSource::Memory { name } if name=="cropped"));
        assert!(sheet.dirty && sheet.selection.cells.is_empty());
        sheet.meta.grid.offset = [3, 5];
        sheet.selection.apply(Some((0, 0)), false, false);
        let old = sheet.external.unwrap();
        let texture = sheet.texture.as_ref().unwrap().raw();
        let grid = sheet.meta.grid;
        library.pending.push(Action::Replace {
            image: crate::sprite::example().0,
            name: "second".into(),
            grid,
        });
        library.apply_pending(&gl, &mut renderer);
        assert_eq!(library.sheets.len(), 2);
        assert_eq!(library.sheets[1].sheet.meta.grid.offset, [3, 5]);
        assert!(library.sheets[1].sheet.selection.cells.is_empty());
        let latest = library.sheets[1].sheet.external.unwrap();
        let latest_texture = library.sheets[1].sheet.texture.as_ref().unwrap().raw();
        library.close(library.sheets[0].id, &mut renderer).unwrap();
        assert_eq!(library.sheets.len(), 1);
        assert!(
            renderer.unregister_external_texture(old).is_err(),
            "closed id must be unregistered"
        );
        use glow::HasContext;
        // SAFETY: the test's GL context is current.
        unsafe {
            assert!(!gl.is_texture(texture));
            assert!(
                gl.is_texture(latest_texture),
                "closing a sibling keeps this texture alive"
            );
        }
        library.close_all(&mut renderer).unwrap();
        assert!(library.sheets.is_empty());
        assert!(renderer.unregister_external_texture(latest).is_err());
        let temp = crate::test_support::TempDir::new("extract");
        let first = temp.0.join("first.png");
        let second = temp.0.join("second.png");
        let bytes = crate::sprite::example().0.to_png().unwrap();
        std::fs::write(&first, &bytes).unwrap();
        std::fs::write(&second, &bytes).unwrap();
        library
            .pending
            .extend([Action::Load(first.clone()), Action::Load(second.clone())]);
        library.apply_pending(&gl, &mut renderer);
        assert_eq!(
            library.sheets.len(),
            2,
            "different files get independent editors"
        );
        let first_id = library.sheets[0].id;
        let first_external = library.sheets[0].sheet.external;
        #[cfg(unix)]
        check_escaping_load_refusals(&mut library, &gl, &mut renderer);
        library.sheets[0].sheet.meta.grid.offset = [1, 0];
        library.sheets[0].focus_requested = false;
        std::fs::create_dir(temp.0.join("alias")).unwrap();
        library
            .pending
            .push(Action::Load(temp.0.join("alias/../first.png")));
        library.apply_pending(&gl, &mut renderer);
        assert_eq!(library.sheets.len(), 2);
        assert!(library.sheets[0].focus_requested);
        assert_eq!(
            library.sheets[0].sheet.external, first_external,
            "focus reuses the texture"
        );
        assert_eq!(
            library.sheets[0].sheet.meta.grid.offset,
            [1, 0],
            "focus retains edits"
        );
        let second_external = library.sheets[1].sheet.external;
        library.sheets[0].start_guess();
        let running = library.sheets[0].attach_test_batch(2);
        let finished = library.sheets[1].attach_test_batch(1);
        finished.send(Ok("done.png".into())).unwrap();
        drop(finished);
        library.pending.push(Action::Reload(first_id));
        library.apply_pending(&gl, &mut renderer);
        assert!(
            running.send(Ok("late.png".into())).is_err()
                && library.sheets[0]
                    .log
                    .iter()
                    .any(|line| line.contains("Region save cancelled")),
            "reload cancels the region save of the previous image"
        );
        assert!(
            library.sheets[1]
                .log
                .iter()
                .any(|line| line.contains("Saved 1 of 1 regions")),
            "every editor's region save is collected before the frame"
        );
        assert_eq!(
            library.sheets[1].sheet.external, second_external,
            "reload affects its own sheet"
        );
        assert_ne!(library.sheets[0].sheet.external, first_external);
        assert!(
            library.sheets[0].guess_job.is_none() && library.sheets[0].guess.is_none(),
            "reload cancels and clears results for the previous image"
        );
        assert!(
            library.sheets[0]
                .log
                .iter()
                .any(|line| line.contains("Image reloaded"))
        );
        let second_log = library.sheets[1].log.clone();
        let library_status = library.status.clone();
        library.sheets[0].sheet.meta.grid.tile = 0;
        library.pending.push(Action::Save(first_id));
        library.apply_pending(&gl, &mut renderer);
        assert_eq!(
            library.status, library_status,
            "sheet validation errors stay out of Library status"
        );
        assert_eq!(
            library.sheets[1].log, second_log,
            "a sibling never receives the error"
        );
        assert!(library.sheets[0].log.last().unwrap().contains("tile"));
        library.sheets[0].sheet.meta.grid.tile = 16;
        library.pending.push(Action::Save(first_id));
        library.apply_pending(&gl, &mut renderer);
        assert!(library.sheets[0].log.last().unwrap().contains("Saved"));
        library
            .pending
            .push(Action::Load(temp.0.join("missing.png")));
        library.apply_pending(&gl, &mut renderer);
        assert_ne!(
            library.status, library_status,
            "Load errors still use Library status"
        );
        library.close_all(&mut renderer).unwrap();
        renderer.shutdown(&mut imgui).unwrap();
    }

    #[cfg(unix)]
    fn check_escaping_load_refusals(
        library: &mut AssetLibrary,
        gl: &Rc<glow::Context>,
        renderer: &mut GlowRenderer,
    ) {
        use std::{fs, os::unix::fs::symlink};
        let links = crate::test_support::TempDir::new("extract");
        let outside = crate::test_support::TempDir::new("extract");
        let SheetSource::File(opened) = &library.sheets[0].sheet.source else {
            panic!("expected a file-backed editor")
        };
        let opened = opened.clone();
        let unopened = outside.0.join("unopened.png");
        let png = fs::read(&opened).unwrap();
        fs::write(&unopened, &png).unwrap();
        let targets = [unopened, opened];
        let aliases = [links.0.join("unopened.png"), links.0.join("opened\n\r.png")];
        for (target, alias) in targets.iter().zip(&aliases) {
            symlink(target, alias).unwrap();
        }
        let snapshot = |library: &AssetLibrary| {
            library
                .sheets
                .iter()
                .map(|e| {
                    (
                        e.id,
                        e.focus_requested,
                        match &e.sheet.source {
                            SheetSource::File(path) => Some(path.clone()),
                            SheetSource::Memory { .. } => None,
                        },
                        e.sheet.meta.clone(),
                        e.sheet.dirty,
                        e.sheet.imgui_id,
                        e.sheet.external,
                        e.log.clone(),
                    )
                })
                .collect::<Vec<_>>()
        };
        library.path_input = "keep this user input".into();
        for alias in &aliases {
            for editor in &mut library.sheets {
                editor.focus_requested = false;
            }
            let before = snapshot(library);
            let next_id = library.next_id;
            library.status = "before refusal".into();
            library.pending.push(Action::Load(alias.clone()));
            library.apply_pending(gl, renderer);
            assert!(
                library.status.starts_with("unsafe image path:"),
                "{}",
                library.status
            );
            assert!(
                library.status.contains(
                    &alias
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .replace('\n', "\\n")
                        .replace('\r', "\\r")
                )
            );
            assert!(!library.status.contains(['\n', '\r']));
            assert_eq!(library.path_input, "keep this user input");
            assert_eq!(
                snapshot(library),
                before,
                "refusal precedes upload and same-file focus"
            );
            assert_eq!(library.next_id, next_id);
            assert!(library.pending.is_empty());
            assert!(!store::sidecar_for(alias).exists());
        }
        for target in &targets {
            assert_eq!(fs::read(target).unwrap(), png);
            assert!(!store::sidecar_for(target).exists());
        }
        assert_eq!(fs::read_dir(&links.0).unwrap().count(), 2);
        assert_eq!(fs::read_dir(&outside.0).unwrap().count(), 1);
    }
}
