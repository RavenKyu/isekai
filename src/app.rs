use std::{error::Error, rc::Rc};

use dear_imgui_glow::GlowRenderer;
use dear_imgui_rs::Context;
use dear_imgui_sdl3::Sdl3PlatformBackend;
use glow::HasContext;
use sdl3::{
    event::{Event, WindowEvent},
    video::{GLContext, GLProfile, Window},
};

pub type AppResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug, PartialEq, Eq)]
pub enum Flow {
    Continue,
    Quit,
}

// Fields drop in declaration order: attachments and ImGui must be gone before
// GLContext/Window, and SDL must outlive everything that uses it.
pub struct App {
    renderer: GlowRenderer,
    platform: Sdl3PlatformBackend,
    imgui: Context,
    gl: Rc<glow::Context>,
    events: sdl3::EventPump,
    gl_context: GLContext,
    window: Window,
    _video: sdl3::VideoSubsystem,
    _sdl: sdl3::Sdl,
}

impl App {
    pub fn new() -> AppResult<Box<Self>> {
        let sdl = sdl3::init()?;
        let video = sdl.video()?;
        let attr = video.gl_attr();
        attr.set_context_profile(GLProfile::Core);
        attr.set_context_version(3, 3);
        attr.set_double_buffer(true);
        let window = video
            .window("rusty-nails", 1280, 720)
            .opengl()
            .resizable()
            .build()?;
        let gl_context = window.gl_create_context()?;
        window.gl_make_current(&gl_context)?;
        // SAFETY: SDL has made this live GL 3.3 context current on this thread.
        // The function table is used only while App owns that context and window.
        let gl = Rc::new(unsafe {
            glow::Context::from_loader_function(|name| {
                video
                    .gl_get_proc_address(name)
                    .map_or(std::ptr::null(), |f| f as *const () as *const _)
            })
        });
        let events = sdl.event_pump()?;
        let mut imgui = Context::create();
        imgui.set_ini_filename(None::<String>)?;
        // Attach Glow first: the SDL C++ backend advertises X11 viewport bits
        // that this single-viewport Rust build cannot republish through Glow's
        // validated set_backend_flags API. Preserve SDL's ownership of its flags.
        let mut renderer = GlowRenderer::with_shared_context(Rc::clone(&gl), &mut imgui)?;
        // SAFETY: on initialization failure locals keep window/context alive
        // through ImGui teardown. On success App's field order provides that
        // lifetime. frame/shutdown restore this GL context before renderer work,
        // and App stays on the initializing thread.
        let platform = match unsafe {
            Sdl3PlatformBackend::init_platform_for_opengl(&mut imgui, &window, &gl_context)
        } {
            Ok(platform) => platform,
            Err(error) => {
                renderer.shutdown(&mut imgui)?;
                return Err(error.into());
            }
        };
        Ok(Box::new(Self {
            renderer,
            platform,
            imgui,
            gl,
            events,
            gl_context,
            window,
            _video: video,
            _sdl: sdl,
        }))
    }

    pub fn frame(&mut self) -> AppResult<Flow> {
        self.window.gl_make_current(&self.gl_context)?;
        let mut quit = false;
        for event in self.events.poll_iter() {
            self.platform.process_event(&mut self.imgui, &event)?;
            quit |= match event {
                Event::Quit { .. } => true,
                Event::Window {
                    window_id,
                    win_event: WindowEvent::CloseRequested,
                    ..
                } => window_id == self.window.id(),
                _ => false,
            };
        }
        if quit {
            return Ok(Flow::Quit);
        }
        self.platform.new_frame(&mut self.imgui)?;
        crate::ui::hello_world(self.imgui.frame());
        let (width, height) = self.window.size_in_pixels();
        // SAFETY: the owned context is current; these calls use no client pointers.
        unsafe {
            self.gl.viewport(0, 0, width as i32, height as i32);
            self.gl.clear_color(0.0, 0.0, 0.0, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }
        let pending = self.imgui.render(self.renderer.renderer_consumer()?);
        self.renderer.render(pending)?;
        self.window.gl_swap_window();
        Ok(Flow::Continue)
    }

    pub fn shutdown(mut self: Box<Self>) -> AppResult<()> {
        self.window.gl_make_current(&self.gl_context)?;
        // Always attempt both shutdowns, in renderer/platform order.
        let renderer = self.renderer.shutdown(&mut self.imgui);
        let platform = self.platform.shutdown(&mut self.imgui);
        renderer?;
        platform?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Run each variant in a separate test process: SDL's platform session is global.
    #[test]
    #[ignore = "requires a display; run under Xvfb"]
    fn event_shutdown() {
        let mut app = App::new().unwrap();
        app.frame().unwrap();
        let event = if std::env::var_os("RN_TEST_CLOSE").is_some() {
            Event::Window {
                timestamp: 0,
                window_id: app.window.id(),
                win_event: WindowEvent::CloseRequested,
            }
        } else {
            Event::Quit { timestamp: 0 }
        };
        app._sdl.event().unwrap().push_event(event).unwrap();
        assert_eq!(app.frame().unwrap(), Flow::Quit);
        app.shutdown().unwrap();
    }
}
