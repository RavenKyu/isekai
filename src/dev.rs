//! The optional development layer owns every ImGui attachment.
use std::rc::Rc;

use dear_imgui_glow::GlowRenderer;
use dear_imgui_rs::Context;
use dear_imgui_sdl3::Sdl3PlatformBackend;
use sdl3::{
    event::Event,
    keyboard::{Mod, Scancode},
    video::{GLContext, Window},
};

use crate::app::AppResult;

// Declaration order tears down editor textures, renderer, platform, then ImGui. App owns this
// entire layer before its GL context/window and keeps them alive through teardown.
pub struct Dev {
    tools: crate::ui::DevTools,
    renderer: GlowRenderer,
    platform: Sdl3PlatformBackend,
    imgui: Context,
    #[cfg(not(target_os = "emscripten"))]
    gl: Rc<glow::Context>,
}

impl Dev {
    pub fn new(gl: &Rc<glow::Context>, window: &Window, gl_context: &GLContext) -> AppResult<Self> {
        let mut imgui = context()?;
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
            #[cfg(not(target_os = "emscripten"))]
            gl: Rc::clone(gl),
        })
    }

    pub fn process_event(&mut self, event: &Event) -> AppResult<()> {
        if is_toggle(event, self.imgui.io().want_text_input()) {
            self.tools.toggle();
        }
        #[cfg(not(target_os = "emscripten"))]
        if let Event::DropFile { filename, .. } = event {
            if !self.tools.is_visible() {
                self.tools.toggle();
            }
            self.tools.library.queue_drop(filename.into());
        }
        // The backend still receives every event, including the toggle key.
        self.platform.process_event(&mut self.imgui, event)?;
        Ok(())
    }

    pub fn begin_frame(&mut self) -> AppResult<()> {
        #[cfg(not(target_os = "emscripten"))]
        self.tools
            .library
            .apply_pending(&self.gl, &mut self.renderer);
        self.platform.new_frame(&mut self.imgui)?;
        self.tools.draw(self.imgui.frame());
        Ok(())
    }

    pub fn is_visible(&self) -> bool {
        self.tools.is_visible()
    }

    // Returns the frame's UI vertex count; reading it from draw data is free.
    pub fn render(&mut self) -> AppResult<usize> {
        let pending = self.imgui.render(self.renderer.renderer_consumer()?);
        let reconciled = self.renderer.reconcile_frame(pending)?;
        let vtx = reconciled.draw_data().total_vtx_count();
        self.renderer.render_reconciled(reconciled)?;
        Ok(vtx)
    }

    pub fn shutdown(&mut self) -> AppResult<()> {
        #[cfg(not(target_os = "emscripten"))]
        let library = self.tools.library.close_all(&mut self.renderer);
        // Always attempt both shutdowns, in renderer/platform order.
        let renderer = self.renderer.shutdown(&mut self.imgui);
        let platform = self.platform.shutdown(&mut self.imgui);
        #[cfg(not(target_os = "emscripten"))]
        library?;
        renderer?;
        platform?;
        Ok(())
    }
}

impl Drop for Dev {
    fn drop(&mut self) {
        #[cfg(not(target_os = "emscripten"))]
        let _ = self.tools.library.close_all(&mut self.renderer);
    }
}

// ImGui writes nothing to disk: no imgui.ini, and the Demo's "Log To File"
// becomes a no-op instead of appending imgui_log.txt to the working directory.
pub(crate) fn context() -> AppResult<Context> {
    let mut imgui = Context::create();
    imgui.set_ini_filename(None::<String>)?;
    imgui.set_log_filename(None::<String>)?;
    Ok(imgui)
}

// Inspect the previous frame's text-input request, independently of key capture.
pub(crate) fn is_toggle(event: &Event, want_text_input: bool) -> bool {
    let blocked =
        Mod::LCTRLMOD | Mod::RCTRLMOD | Mod::LALTMOD | Mod::RALTMOD | Mod::LGUIMOD | Mod::RGUIMOD;
    matches!(event, Event::KeyDown {
        scancode: Some(Scancode::Grave), repeat: false, keymod, ..
    } if !keymod.intersects(blocked) && !want_text_input)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use sdl3::keyboard::Keycode;

    pub(crate) fn key(repeat: bool, keymod: Mod) -> Event {
        Event::KeyDown {
            timestamp: 0,
            window_id: 1,
            keycode: Some(Keycode::Grave),
            scancode: Some(Scancode::Grave),
            keymod,
            repeat,
            which: 0,
            raw: 0,
        }
    }

    #[test]
    fn only_unmodified_nonrepeating_physical_grave_toggles_outside_text_input() {
        assert!(is_toggle(&key(false, Mod::NOMOD), false));
        assert!(is_toggle(&key(false, Mod::LSHIFTMOD), false));
        assert!(!is_toggle(&key(true, Mod::NOMOD), false));
        assert!(!is_toggle(&key(false, Mod::NOMOD), true));
        for modifier in [
            Mod::LCTRLMOD,
            Mod::RCTRLMOD,
            Mod::LALTMOD,
            Mod::RALTMOD,
            Mod::LGUIMOD,
            Mod::RGUIMOD,
        ] {
            assert!(!is_toggle(&key(false, modifier), false));
        }
        let mut different = key(false, Mod::NOMOD);
        if let Event::KeyDown { scancode, .. } = &mut different {
            *scancode = Some(Scancode::A);
        }
        assert!(!is_toggle(&different, false));
        if let Event::KeyDown { scancode, .. } = &mut different {
            *scancode = None;
        }
        assert!(!is_toggle(&different, false));
        // Physical key wins even when a layout maps it to a different character.
        if let Event::KeyDown {
            scancode, keycode, ..
        } = &mut different
        {
            *scancode = Some(Scancode::Grave);
            *keycode = Some(Keycode::A);
        }
        assert!(is_toggle(&different, false));
        assert!(!is_toggle(
            &Event::KeyUp {
                timestamp: 0,
                window_id: 1,
                keycode: Some(Keycode::Grave),
                scancode: Some(Scancode::Grave),
                keymod: Mod::NOMOD,
                repeat: false,
                which: 0,
                raw: 0
            },
            false
        ));
        assert!(!is_toggle(
            &Event::TextInput {
                timestamp: 0,
                window_id: 1,
                text: "`".into()
            },
            false
        ));
    }
}
