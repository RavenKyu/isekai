mod clips;
mod edit;
mod extract;
mod model;
mod preview;
mod regions;
mod sprites;
mod tiles;
mod view;
pub use model::*;

use crate::{
    raster::{self, GridGuess, RgbaImage},
    sprite::{Grid, SpriteSheet, store, texture::SheetTexture},
};
use dear_imgui_glow::{ExternalTextureId, GlowRenderer};
use dear_imgui_rs::{Condition, TextureId, Ui};
use std::{path::PathBuf, rc::Rc};

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
pub enum Action {
    Load(PathBuf),
    Reload,
    Close,
    Save,
    Replace { image: RgbaImage, name: String },
}
pub struct SpriteLibrary {
    pub open: bool,
    pub path_input: String,
    pub status: String,
    pub assets: Vec<PathBuf>,
    pub key_color: String,
    pub use_key: bool,
    pub zoom: u32,
    pub show_grid: bool,
    pub guess: Option<GridGuess>,
    pub sheet: Option<OpenSheet>,
    pub pending: Vec<Action>,
    tile_draft: tiles::TileDraft,
    assets_scanned: bool,
    selected_sprite: Option<String>,
    selected_clip: Option<String>,
    suggestions: Vec<raster::CellRect>,
    clip_draft: clips::ClipDraft,
    preview: preview::PreviewState,
}
impl Default for SpriteLibrary {
    fn default() -> Self {
        Self {
            open: false,
            path_input: String::new(),
            status: String::new(),
            assets: vec![],
            key_color: "ff00ff".into(),
            use_key: false,
            zoom: 2,
            show_grid: true,
            guess: None,
            sheet: None,
            pending: vec![],
            tile_draft: Default::default(),
            assets_scanned: false,
            selected_sprite: None,
            selected_clip: None,
            suggestions: Vec::new(),
            clip_draft: Default::default(),
            preview: Default::default(),
        }
    }
}
impl SpriteLibrary {
    pub fn queue_drop(&mut self, path: PathBuf) {
        self.open = true;
        self.pending.push(Action::Load(path));
    }
    pub fn apply_pending(&mut self, gl: &Rc<glow::Context>, renderer: &mut GlowRenderer) {
        for action in std::mem::take(&mut self.pending) {
            if let Err(error) = self.apply_action(action, gl, renderer) {
                self.status = error.replace(['\n', '\r'], " ");
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
    fn apply_action(
        &mut self,
        action: Action,
        gl: &Rc<glow::Context>,
        renderer: &mut GlowRenderer,
    ) -> Result<(), String> {
        match action {
            Action::Load(path) => {
                let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
                let sidecar = store::sidecar_for(&path);
                // try_exists propagates permission errors rather than treating them
                // as a new sidecar and risking an accidental overwrite.
                let meta = if sidecar.try_exists().map_err(|e| e.to_string())? {
                    Some(store::load(&sidecar).map_err(|e| e.to_string())?.sheet)
                } else {
                    None
                };
                let (image, meta, dirty) = open_sheet(&path, &bytes, self.key()?, meta)?;
                let opened = OpenSheet::prepare(
                    SheetSource::File(path.clone()),
                    image,
                    meta,
                    dirty,
                    gl,
                    renderer,
                )?;
                self.install(opened, renderer)?;
                self.path_input = path.to_string_lossy().into_owned();
                self.status = format!("Opened {}", path.display());
            }
            Action::Replace { image, name } => {
                let grid = self.sheet.as_ref().map_or(
                    Grid {
                        tile: 16,
                        offset: [0, 0],
                        spacing: 0,
                    },
                    |s| s.meta.grid,
                );
                let meta = SpriteSheet::new_for(&format!("{name}.png"), grid);
                let opened = OpenSheet::prepare(
                    SheetSource::Memory { name },
                    image,
                    meta,
                    true,
                    gl,
                    renderer,
                )?;
                self.install(opened, renderer)?;
                self.status = "Memory image: save the image first".into();
            }
            Action::Reload => {
                let Some(sheet) = &self.sheet else {
                    return Ok(());
                };
                let SheetSource::File(path) = &sheet.source else {
                    return Err("save the image first".into());
                };
                let (image, meta, _) = open_sheet(
                    path,
                    &std::fs::read(path).map_err(|e| e.to_string())?,
                    self.key()?,
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
                self.install(opened, renderer)?;
                self.status = "Image reloaded; metadata retained".into();
            }
            Action::Close => {
                self.close_all(renderer)?;
                self.status = "Sheet closed".into();
            }
            Action::Save => {
                if let Some(sheet) = &self.sheet {
                    self.tile_draft.validate(&sheet.meta)?;
                }
                self.clip_draft.validate()?;
                let Some(sheet) = &mut self.sheet else {
                    return Ok(());
                };
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
                self.status = format!("Saved {}", sidecar.display());
            }
        }
        Ok(())
    }
    fn install(
        &mut self,
        mut opened: OpenSheet,
        renderer: &mut GlowRenderer,
    ) -> Result<(), String> {
        if let Err(error) = self.close_all(renderer) {
            if let Some(id) = opened.external.take() {
                let _ = renderer.unregister_external_texture(id);
            }
            return Err(error);
        }
        self.sheet = Some(opened);
        self.selected_sprite = None;
        self.selected_clip = None;
        self.suggestions.clear();
        self.clip_draft = Default::default();
        self.preview = Default::default();
        self.guess = None;
        self.tile_draft = Default::default();
        Ok(())
    }
    pub fn close_all(&mut self, renderer: &mut GlowRenderer) -> Result<(), String> {
        if let Some(sheet) = &mut self.sheet
            && let Some(id) = sheet.external
        {
            renderer
                .unregister_external_texture(id)
                .map_err(|e| e.to_string())?;
            sheet.external = None;
        }
        self.sheet = None;
        self.guess = None;
        self.tile_draft = Default::default();
        Ok(())
    }
    pub fn draw(&mut self, ui: &Ui, now_ms: u64) {
        if !self.open {
            return;
        }
        if !self.assets_scanned {
            self.refresh_assets();
            self.assets_scanned = true;
        }
        let mut open = self.open;
        ui.window("Sprite Library")
            .opened(&mut open)
            .size([1000.0, 650.0], Condition::FirstUseEver)
            .position([140.0, 40.0], Condition::FirstUseEver)
            .build(|| {
                self.import_panel(ui);
                ui.text_wrapped(&self.status);
                let Some(sheet) = &self.sheet else {
                    return;
                };
                let name = match &sheet.source {
                    SheetSource::File(path) => path
                        .file_name()
                        .map_or_else(|| "?".into(), |s| s.to_string_lossy()),
                    SheetSource::Memory { name } => name.as_str().into(),
                };
                ui.separator();
                ui.text(format!("{}{}", name, if sheet.dirty { " *" } else { "" }));
                let memory = matches!(sheet.source, SheetSource::Memory { .. });
                let dirty = sheet.dirty;
                {
                    let _disabled = ui.begin_disabled_with_cond(memory);
                    if ui.button("Save") {
                        self.pending.push(Action::Save);
                    }
                    ui.same_line();
                    if ui.button("Reload image") {
                        self.pending.push(Action::Reload);
                    }
                }
                ui.same_line();
                if ui.button("Close") {
                    if dirty {
                        ui.open_popup("Discard changes?");
                    } else {
                        self.pending.push(Action::Close);
                    }
                }
                if memory {
                    ui.text("Memory image: save the image first");
                    regions::memory_header(self, ui);
                }
                ui.modal_popup("Discard changes?", || {
                    ui.text("Discard unsaved changes?");
                    if ui.button("Discard") {
                        self.pending.push(Action::Close);
                        ui.close_current_popup();
                    }
                    ui.same_line();
                    if ui.button("Cancel") {
                        ui.close_current_popup();
                    }
                });
                self.grid_panel(ui);
                let overlays = self.sprite_overlays();
                regions::sheet_view(self, ui, overlays);
                if let Some(_tabs) = ui.tab_bar("Sprite library tabs") {
                    if let Some(_tab) = ui.tab_item("Tiles") {
                        self.tiles_tab(ui);
                    }
                    if let Some(_tab) = ui.tab_item("Sprites") {
                        self.sprites_tab(ui);
                    }
                    if let Some(_tab) = ui.tab_item("Clips") {
                        self.clips_tab(ui);
                    }
                    if let Some(_tab) = ui.tab_item("Preview") {
                        self.preview_tab(ui, now_ms);
                    }
                    regions::tab(self, ui);
                    extract::tab(self, ui);
                }
            });
        self.open = open;
    }
    fn refresh_assets(&mut self) {
        match std::env::current_dir().map(|p| p.join("assets")) {
            Ok(root) => match list_assets(&root) {
                Ok(files) => self.assets = files,
                Err(error) => {
                    self.assets.clear();
                    self.status = error;
                }
            },
            Err(error) => self.status = error.to_string(),
        }
    }
    fn import_panel(&mut self, ui: &Ui) {
        ui.input_text("PNG path", &mut self.path_input).build();
        if ui.button("Load") {
            self.pending
                .push(Action::Load(PathBuf::from(&self.path_input)));
        }
        ui.same_line();
        if ui.button("Browse...")
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("PNG", &["png"])
                .pick_file()
        {
            self.pending.push(Action::Load(path));
        }
        ui.same_line();
        if ui.button("Refresh assets/") {
            self.refresh_assets();
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
                }
            });
    }
    fn grid_panel(&mut self, ui: &Ui) {
        let Some(sheet) = &mut self.sheet else {
            return;
        };
        let mut grid = sheet.meta.grid;
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
        if ui.button("Guess") {
            self.guess = Some(raster::guess_grid(
                &sheet.image,
                &raster::DEFAULT_GUESS_CANDIDATES,
                true,
            ));
        }
        ui.same_line();
        ui.checkbox("Show grid", &mut self.show_grid);
        ui.same_line();
        ui.set_next_item_width(160.0);
        ui.slider("Zoom", 1, 16, &mut self.zoom);
        if let Some(guess) = &self.guess {
            for score in &guess.scores {
                ui.text(format!(
                    "{} px offset ({}, {}): {}/{} unique ({:.3})",
                    score.tile,
                    score.offset.0,
                    score.offset.1,
                    score.unique,
                    score.total,
                    score.ratio
                ));
            }
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
            match sheet.set_grid(grid) {
                Ok(()) => {
                    self.tile_draft = Default::default();
                    extract::invalidate();
                    self.status = "Grid updated".into();
                }
                Err(error) => self.status = error,
            }
        }
        let cells = sheet.meta.grid.cells(sheet.image.width, sheet.image.height);
        ui.text(format!(
            "{} x {} cells, leftover {} x {} px",
            cells.cols, cells.rows, cells.leftover_right, cells.leftover_bottom
        ));
    }
}
impl OpenSheet {
    fn set_grid(&mut self, grid: Grid) -> Result<(), String> {
        let mut candidate = self.meta.clone();
        candidate.grid = grid;
        candidate
            .validate_bounds(self.image.width, self.image.height)
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
    pub(super) fn context() -> (Context, SynchronousRendererConsumer) {
        let mut context = crate::dev::context().unwrap();
        context.io_mut().set_display_size([1400.0, 900.0]);
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
    fn library_geometry_in_its_own_process() {
        // Dear ImGui permits one owning thread per process. Keep the original
        // UI test unchanged and run this second context in a separate process.
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "ui::sprite_library::tests::hidden_library_grid_and_zoom_geometry",
                "--ignored",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    #[test]
    #[ignore = "run by library_geometry_in_its_own_process"]
    fn hidden_library_grid_and_zoom_geometry() {
        // Same serialized CPU ImGui context style as the existing UI tests.
        let (mut context, consumer) = context();
        let library = std::cell::RefCell::new(SpriteLibrary::default());
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
        library.borrow_mut().sheet = Some(fake_sheet());
        library.borrow_mut().show_grid = false;
        let no_grid = vertices(&mut context, &consumer, |ui| {
            library.borrow_mut().draw(ui, 0)
        });
        library.borrow_mut().show_grid = true;
        let grid = vertices(&mut context, &consumer, |ui| {
            library.borrow_mut().draw(ui, 0)
        });
        assert!(grid > no_grid, "grid={grid}, no_grid={no_grid}");
        library.borrow_mut().zoom = 2;
        let size = std::cell::Cell::new([0.0; 2]);
        vertices(&mut context, &consumer, |ui| {
            ui.window("image dimensions").build(|| {
                let lib = library.borrow();
                let sheet = lib.sheet.as_ref().unwrap();
                ui.image(
                    sheet.imgui_id,
                    [
                        sheet.image.width as f32 * lib.zoom as f32,
                        sheet.image.height as f32 * lib.zoom as f32,
                    ],
                );
                size.set(ui.item_rect_size());
            });
        });
        assert_eq!(size.get(), [128.0, 32.0]);
        library.borrow_mut().open = false;
        assert_eq!(
            vertices(&mut context, &consumer, |ui| library
                .borrow_mut()
                .draw(ui, 0)),
            0
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
        }];
        let two = [
            one[0],
            crate::sprite::format::Part {
                col: 1,
                row: 0,
                dx: 16,
                dy: 0,
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
        if std::env::var_os("RN_LIBRARY_GL_CHILD").is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .env("RN_LIBRARY_GL_CHILD", "1")
                .args([
                    "--exact",
                    "ui::sprite_library::tests::queued_replacement_registration_and_close",
                    "--ignored",
                ])
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let sdl = sdl3::init().unwrap();
        let video = sdl.video().unwrap();
        video
            .gl_attr()
            .set_context_profile(sdl3::video::GLProfile::Core);
        video.gl_attr().set_context_version(3, 3);
        let window = video
            .window("library lifetime test", 64, 64)
            .opengl()
            .hidden()
            .build()
            .unwrap();
        let context = window.gl_create_context().unwrap();
        window.gl_make_current(&context).unwrap();
        // SAFETY: the test keeps this context current through library/renderer teardown.
        let gl = Rc::new(unsafe {
            glow::Context::from_loader_function(|name| {
                video
                    .gl_get_proc_address(name)
                    .map_or(std::ptr::null(), |f| f as *const () as *const _)
            })
        });
        let mut imgui = crate::dev::context().unwrap();
        let mut renderer = GlowRenderer::with_shared_context(Rc::clone(&gl), &mut imgui).unwrap();
        let mut library = SpriteLibrary::default();
        library.pending.push(Action::Replace {
            image: crate::sprite::example().0,
            name: "cropped".into(),
        });
        library.apply_pending(&gl, &mut renderer);
        let sheet = library.sheet.as_mut().unwrap();
        assert!(matches!(&sheet.source,SheetSource::Memory { name } if name=="cropped"));
        assert!(sheet.dirty && sheet.selection.cells.is_empty());
        sheet.meta.grid.offset = [3, 5];
        sheet.selection.apply(Some((0, 0)), false, false);
        let old = sheet.external.unwrap();
        let texture = sheet.texture.as_ref().unwrap().raw();
        library.pending.push(Action::Replace {
            image: crate::sprite::example().0,
            name: "second".into(),
        });
        library.apply_pending(&gl, &mut renderer);
        assert_eq!(library.sheet.as_ref().unwrap().meta.grid.offset, [3, 5]);
        assert!(library.sheet.as_ref().unwrap().selection.cells.is_empty());
        assert!(
            renderer.unregister_external_texture(old).is_err(),
            "replacement must unregister old id"
        );
        use glow::HasContext;
        // SAFETY: current context; the first texture should have been deleted.
        unsafe {
            assert!(!gl.is_texture(texture));
        }
        let latest = library.sheet.as_ref().unwrap().external.unwrap();
        library.close_all(&mut renderer).unwrap();
        assert!(renderer.unregister_external_texture(latest).is_err());
        renderer.shutdown(&mut imgui).unwrap();
    }
}
