//! Independent feasibility spike; member rn-35g9.7lw replaces this with the shared App.
#[cfg(not(target_os = "emscripten"))]
fn main() {
    println!("Build this example for wasm32-unknown-emscripten.");
}

#[cfg(target_os = "emscripten")]
fn main() {
    web::run();
}

#[cfg(target_os = "emscripten")]
mod web {
    use dear_imgui_glow::GlowRenderer;
    use dear_imgui_rs::{Condition, Context};
    use dear_imgui_sdl3::Sdl3PlatformBackend;
    use glow::HasContext;
    use sdl3::video::{GLContext, GLProfile, Window};
    use std::ffi::c_void;

    unsafe extern "C" {
        fn emscripten_set_main_loop_arg(
            callback: extern "C" fn(*mut c_void),
            arg: *mut c_void,
            fps: i32,
            simulate_infinite_loop: i32,
        );
        fn emscripten_cancel_main_loop();
    }

    // Consumers drop before ImGui, then the GL context/window and SDL subsystems.
    struct Spike {
        renderer: GlowRenderer,
        platform: Sdl3PlatformBackend,
        imgui: Context,
        gl_context: GLContext,
        window: Window,
        events: sdl3::EventPump,
        _video: sdl3::VideoSubsystem,
        _sdl: sdl3::Sdl,
        frames: u32,
    }

    impl Spike {
        fn new() -> Self {
            let sdl = sdl3::init().expect("SDL init");
            let video = sdl.video().expect("SDL video");
            let attrs = video.gl_attr();
            attrs.set_context_profile(GLProfile::GLES);
            attrs.set_context_version(3, 0);
            attrs.set_double_buffer(true);
            let window = video
                .window("Emscripten spike", 960, 540)
                .opengl()
                .build()
                .expect("SDL window");
            let gl_context = window.gl_create_context().expect("GLES 3 context");
            window
                .gl_make_current(&gl_context)
                .expect("current context");
            // SAFETY: SDL's live, current GLES context supplies the function pointers.
            let gl = unsafe {
                glow::Context::from_loader_function(|name| {
                    video
                        .gl_get_proc_address(name)
                        .map_or(std::ptr::null(), |f| f as *const () as *const c_void)
                })
            };
            let mut imgui = Context::create();
            imgui
                .set_ini_filename(None::<std::path::PathBuf>)
                .expect("disable ini persistence");
            // SAFETY: Spike owns the window/context until after shutdown, makes the
            // context current for each frame and teardown, and enables no viewports.
            let platform = unsafe {
                Sdl3PlatformBackend::init_platform_for_opengl(&mut imgui, &window, &gl_context)
                    .expect("SDL platform backend")
            };
            let renderer = GlowRenderer::new(gl, &mut imgui).expect("glow renderer");
            let events = sdl.event_pump().expect("SDL event pump");
            Self {
                renderer,
                platform,
                imgui,
                gl_context,
                window,
                events,
                _video: video,
                _sdl: sdl,
                frames: 0,
            }
        }

        fn frame(&mut self) -> bool {
            self.window
                .gl_make_current(&self.gl_context)
                .expect("current frame context");
            for event in self.events.poll_iter() {
                self.platform
                    .process_event(&mut self.imgui, &event)
                    .expect("SDL event");
                if matches!(event, sdl3::event::Event::Quit { .. }) {
                    return false;
                }
            }
            self.platform.new_frame(&mut self.imgui).expect("SDL frame");
            let ui = self.imgui.frame();
            ui.window("Emscripten spike")
                .position([32.0, 32.0], Condition::Always)
                .size([500.0, 180.0], Condition::Always)
                .build(|| {
                    ui.text("Hello World");
                    ui.text("SDL3 + Dear ImGui + glow / WebGL 2");
                });
            let (width, height) = self.window.size_in_pixels();
            let gl = self.renderer.gl_context().expect("live glow context");
            // SAFETY: The owned context is current and dimensions come from SDL.
            unsafe {
                gl.viewport(0, 0, width as i32, height as i32);
                gl.clear_color(0.08, 0.10, 0.14, 1.0);
                gl.clear(glow::COLOR_BUFFER_BIT);
            }
            let pending = self.imgui.render(
                self.renderer
                    .renderer_consumer()
                    .expect("renderer consumer"),
            );
            self.renderer.render(pending).expect("render frame");
            self.window.gl_swap_window();
            self.frames += 1;
            if self.frames == 3 {
                println!("web-spike: rendered frames=3");
            }
            true
        }
    }

    impl Drop for Spike {
        fn drop(&mut self) {
            self.window
                .gl_make_current(&self.gl_context)
                .expect("current shutdown context");
            self.renderer
                .shutdown(&mut self.imgui)
                .expect("renderer shutdown");
            self.platform
                .shutdown(&mut self.imgui)
                .expect("platform shutdown");
        }
    }

    extern "C" fn tick(arg: *mut c_void) {
        // SAFETY: run transfers one Box to this loop; callbacks are serialized on
        // the browser main thread. Cancellation precedes recovering/dropping it.
        let spike = unsafe { &mut *arg.cast::<Spike>() };
        if !spike.frame() {
            unsafe {
                emscripten_cancel_main_loop();
                drop(Box::from_raw(arg.cast::<Spike>()));
            }
        }
    }

    pub fn run() {
        let spike = Box::into_raw(Box::new(Spike::new()));
        // SAFETY: The loop owns the heap allocation, with no borrowed stack data.
        // fps=0 follows requestAnimationFrame; cancellation reclaims the Box.
        unsafe { emscripten_set_main_loop_arg(tick, spike.cast(), 0, 1) };
    }
}
