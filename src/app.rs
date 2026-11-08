use std::{error::Error, rc::Rc};

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
    SmokeComplete(crate::smoke::Report),
}

// Fields drop in declaration order: the dev attachments and ImGui must be gone before
// GLContext/Window, and SDL must outlive everything that uses it.
pub struct App {
    #[cfg(feature = "dev")]
    dev: crate::dev::Dev,
    gl: Rc<glow::Context>,
    events: sdl3::EventPump,
    smoke: Option<crate::smoke::Smoke>,
    gl_context: GLContext,
    window: Window,
    _video: sdl3::VideoSubsystem,
    _sdl: sdl3::Sdl,
}

impl App {
    pub fn new(smoke: Option<crate::smoke::Smoke>) -> AppResult<Box<Self>> {
        let sdl = sdl3::init()?;
        let video = sdl.video()?;
        let attr = video.gl_attr();
        #[cfg(not(target_os = "emscripten"))]
        {
            attr.set_context_profile(GLProfile::Core);
            attr.set_context_version(3, 3);
        }
        #[cfg(target_os = "emscripten")]
        {
            attr.set_context_profile(GLProfile::GLES);
            attr.set_context_version(3, 0);
        }
        attr.set_double_buffer(true);
        let window = video
            .window("rusty-nails", 1280, 720)
            .opengl()
            .resizable()
            .build()?;
        let gl_context = window.gl_create_context()?;
        window.gl_make_current(&gl_context)?;
        // SDL leaves the interval at 0, and on Wayland swaps wait only when asked,
        // so the native loop would otherwise spin. RAF paces the web loop, and SDL
        // would need a main loop there. Without vsync support the app still draws.
        #[cfg(not(target_os = "emscripten"))]
        let _ = video.gl_set_swap_interval(sdl3::video::SwapInterval::VSync);
        // SAFETY: SDL has made this live GL context current on this thread.
        // The function table is used only while App owns that context and window.
        let gl = Rc::new(unsafe {
            glow::Context::from_loader_function(|name| {
                video
                    .gl_get_proc_address(name)
                    .map_or(std::ptr::null(), |f| f as *const () as *const _)
            })
        });
        let events = sdl.event_pump()?;
        #[cfg(feature = "dev")]
        let dev = crate::dev::Dev::new(&gl, &window, &gl_context)?;
        Ok(Box::new(Self {
            #[cfg(feature = "dev")]
            dev,
            gl,
            events,
            smoke,
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
            // Smoke-only observation proves the probe reached SDL even without
            // the dev layer. It never changes interactive non-dev behavior.
            if matches!(
                event,
                Event::KeyDown {
                    scancode: Some(sdl3::keyboard::Scancode::Grave),
                    repeat: false,
                    ..
                }
            ) && let Some(smoke) = &mut self.smoke
            {
                smoke.observe_key_probe();
            }
            #[cfg(feature = "dev")]
            self.dev.process_event(&event)?;
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
            if self.smoke.is_some() {
                return Err("window closed before smoke completed".into());
            }
            return Ok(Flow::Quit);
        }
        #[cfg(feature = "dev")]
        self.dev.begin_frame()?;
        let (width, height) = self.window.size_in_pixels();
        // SAFETY: the owned context is current; these calls use no client pointers.
        unsafe {
            self.gl.viewport(0, 0, width as i32, height as i32);
            self.gl.clear_color(0.0, 0.0, 0.0, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }
        #[cfg(feature = "dev")]
        let visible = self.dev.is_visible();
        #[cfg(not(feature = "dev"))]
        let visible = false;
        let measure = self
            .smoke
            .as_ref()
            .is_some_and(|smoke| smoke.is_last_frame(visible));
        #[cfg(feature = "dev")]
        let vtx = self.dev.render()?;
        #[cfg(not(feature = "dev"))]
        let vtx = 0;
        let lit = if measure {
            self.lit_pixels(width, height)?
        } else {
            0
        };
        self.window.gl_swap_window();
        if let Some(smoke) = &mut self.smoke {
            match smoke.finish_frame(sdl3::timer::ticks(), visible, vtx, lit)? {
                crate::smoke::Step::Complete(report) => return Ok(Flow::SmokeComplete(report)),
                crate::smoke::Step::SendKeys => {
                    #[cfg(not(target_os = "emscripten"))]
                    self.push_smoke_keys()?;
                }
                crate::smoke::Step::Continue => {}
            }
        }
        Ok(Flow::Continue)
    }

    #[cfg(not(target_os = "emscripten"))]
    fn push_smoke_keys(&self) -> AppResult<()> {
        use sdl3::keyboard::{Keycode, Mod, Scancode};
        let events = self._sdl.event()?;
        for repeat in [false, true] {
            events.push_event(Event::KeyDown {
                timestamp: 0,
                window_id: self.window.id(),
                keycode: Some(Keycode::Grave),
                scancode: Some(Scancode::Grave),
                keymod: Mod::NOMOD,
                repeat,
                which: 0,
                raw: 0,
            })?;
        }
        events.push_event(Event::KeyUp {
            timestamp: 0,
            window_id: self.window.id(),
            keycode: Some(Keycode::Grave),
            scancode: Some(Scancode::Grave),
            keymod: Mod::NOMOD,
            repeat: false,
            which: 0,
            raw: 0,
        })?;
        Ok(())
    }

    fn lit_pixels(&self, width: u32, height: u32) -> AppResult<usize> {
        let bytes = (width as usize)
            .checked_mul(height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .filter(|n| *n > 0)
            .ok_or("invalid framebuffer dimensions")?;
        let mut pixels = vec![0; bytes];
        // SAFETY: frame made the owned GL context current. The buffer contains
        // width*height RGBA8 bytes; no pixel pack buffer is bound. Read the back
        // buffer synchronously after rendering and before swapping it away.
        unsafe {
            self.gl.bind_buffer(glow::PIXEL_PACK_BUFFER, None);
            self.gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
            self.gl.read_pixels(
                0,
                0,
                width as i32,
                height as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut pixels)),
            );
            let error = self.gl.get_error();
            if error != glow::NO_ERROR {
                return Err(format!("GL readback failed: 0x{error:x}").into());
            }
        }
        // The clear color is opaque black. Alpha is not evidence of drawing.
        Ok(pixels
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|pixel| pixel[..3] != [0, 0, 0])
            .count())
    }

    // The web driver deliberately retains App until the page is destroyed.
    #[cfg_attr(target_os = "emscripten", allow(dead_code))]
    // Consume the same heap-owned App used by the native and browser drivers.
    #[allow(clippy::boxed_local)]
    pub fn shutdown(self: Box<Self>) -> AppResult<()> {
        self.window.gl_make_current(&self.gl_context)?;
        #[cfg(feature = "dev")]
        {
            let mut app = self;
            app.dev.shutdown()?;
        }
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
        let mut app = App::new(None).unwrap();
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
