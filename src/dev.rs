//! The optional development layer owns every ImGui attachment.
use std::rc::Rc;

use dear_imgui_glow::GlowRenderer;
use dear_imgui_rs::Context;
use dear_imgui_sdl3::Sdl3PlatformBackend;
use sdl3::{
    event::Event,
    video::{GLContext, Window},
};

use crate::app::AppResult;

// Declaration order tears down renderer, platform, then ImGui. App owns this
// entire layer before its GL context/window and keeps them alive through teardown.
pub struct Dev {
    renderer: GlowRenderer,
    platform: Sdl3PlatformBackend,
    imgui: Context,
    tools: crate::ui::DevTools,
}

impl Dev {
    pub fn new(gl: &Rc<glow::Context>, window: &Window, gl_context: &GLContext) -> AppResult<Self> {
        let mut imgui = Context::create();
        imgui.set_ini_filename(None::<String>)?;
        // Attach Glow first: the SDL C++ backend advertises X11 viewport bits
        // that this single-viewport Rust build cannot republish through Glow's
        // validated set_backend_flags API. Preserve SDL's ownership of its flags.
        let mut renderer = GlowRenderer::with_shared_context(Rc::clone(gl), &mut imgui)?;
        // SAFETY: App::new keeps its local window/context alive through teardown
        // on failure. On success App declares this layer before those resources.
        // App restores the context before frame/shutdown, on the initializing thread.
        let platform = match unsafe {
            Sdl3PlatformBackend::init_platform_for_opengl(&mut imgui, window, gl_context)
        } {
            Ok(platform) => platform,
            Err(error) => {
                renderer.shutdown(&mut imgui)?;
                return Err(error.into());
            }
        };
        Ok(Self {
            renderer,
            platform,
            imgui,
            tools: crate::ui::DevTools::default(),
        })
    }

    pub fn process_event(&mut self, event: &Event) -> AppResult<()> {
        self.platform.process_event(&mut self.imgui, event)?;
        Ok(())
    }

    pub fn begin_frame(&mut self) -> AppResult<()> {
        self.platform.new_frame(&mut self.imgui)?;
        self.tools.draw(self.imgui.frame());
        Ok(())
    }

    pub fn render(&mut self) -> AppResult<usize> {
        let pending = self.imgui.render(self.renderer.renderer_consumer()?);
        let reconciled = self.renderer.reconcile_frame(pending)?;
        let vtx = reconciled.draw_data().total_vtx_count();
        self.renderer.render_reconciled(reconciled)?;
        Ok(vtx)
    }

    pub fn shutdown(&mut self) -> AppResult<()> {
        // Always attempt both shutdowns, in renderer/platform order.
        let renderer = self.renderer.shutdown(&mut self.imgui);
        let platform = self.platform.shutdown(&mut self.imgui);
        renderer?;
        platform?;
        Ok(())
    }
}
