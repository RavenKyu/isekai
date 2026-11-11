use crate::{
    cli::Command,
    playback::Playback,
    smoke::{Smoke, Step},
};
use isekai_core::input::{Key, Modifiers};
use isekai_render::{Gpu, Readback, SpriteBatch, SpritePipeline, SurfaceTarget, lit_count};
use std::sync::Arc;
use web_time::Instant;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};
struct Render {
    #[cfg(feature = "dev")]
    dev: isekai_editor::DevShell,
    playback: Option<Playback>,
    batch: SpriteBatch,
    pipeline: SpritePipeline,
    target: SurfaceTarget,
    gpu: Gpu,
    window: Arc<Window>,
}
pub struct App {
    render: Option<Render>,
    smoke: Option<Smoke>,
    command: Option<Command>,
    started: Instant,
    modifiers: Modifiers,
    pub result: Result<Option<String>, String>,
    done: bool,
}
impl App {
    pub fn new(smoke: Option<Smoke>, command: Option<Command>) -> Self {
        Self {
            render: None,
            smoke,
            command,
            started: Instant::now(),
            modifiers: Modifiers::default(),
            result: Ok(None),
            done: false,
        }
    }
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: String) {
        if !self.done {
            self.result = Err(error);
            self.done = true;
        }
        event_loop.exit();
    }
    fn init(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("isekai")
                        .with_inner_size(winit::dpi::PhysicalSize::new(1280, 720)),
                )
                .map_err(|e| e.to_string())?,
        );
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle_from_env(
                Box::new(event_loop.owned_display_handle()),
            ));
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| e.to_string())?;
        let gpu = pollster::block_on(Gpu::new(instance, Some(&surface)))?;
        let size = window.inner_size();
        let target = SurfaceTarget::new(&gpu, surface, size.width, size.height)?;
        let pipeline = SpritePipeline::new(&gpu);
        let batch = SpriteBatch::new(&gpu);
        let playback = match self.command.take() {
            Some(Command::Play { source, animation }) => Some(Playback::open_native(
                std::path::Path::new(&source),
                animation.as_deref(),
                &gpu,
                &pipeline,
            )?),
            None => None,
        };
        #[cfg(feature = "dev")]
        let dev = isekai_editor::DevShell::new(&gpu, &window);
        self.render = Some(Render {
            #[cfg(feature = "dev")]
            dev,
            playback,
            batch,
            pipeline,
            target,
            gpu,
            window,
        });
        Ok(())
    }
    fn frame(&mut self) -> Result<(), String> {
        let now_ms = self.started.elapsed().as_millis() as u64;
        let r = self.render.as_mut().unwrap();
        let Some(surface) = r.target.acquire(&r.gpu)? else {
            return Ok(());
        };
        r.batch.clear();
        if let Some(play) = r.playback.as_mut() {
            play.fill(
                now_ms,
                r.target.config.width,
                r.target.config.height,
                &mut r.batch,
            )?;
        }
        r.batch.upload(&r.gpu);
        let mut encoder = r.gpu.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("isekai frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &r.target.frame.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            if let Some(play) = &r.playback {
                r.batch.draw(&mut pass, &r.pipeline, &[&play.texture]);
            }
        }
        #[cfg(feature = "dev")]
        let vtx = r.dev.paint(
            &r.gpu,
            &r.window,
            now_ms as f64 / 1000.0,
            &mut encoder,
            &r.target.frame,
        );
        #[cfg(not(feature = "dev"))]
        let vtx = 0;
        #[cfg(feature = "dev")]
        let visible = r.dev.tools.visible;
        #[cfg(not(feature = "dev"))]
        let visible = false;
        let view = surface.texture.create_view(&Default::default());
        r.target.blit(&mut encoder, &view);
        r.gpu.queue.submit([encoder.finish()]);
        let lit = if self
            .smoke
            .as_ref()
            .is_some_and(|s| s.is_last_frame(visible))
        {
            let read = Readback::begin(&r.gpu, &r.target.frame);
            r.gpu
                .device
                .poll(wgpu::PollType::wait_indefinitely())
                .map_err(|e| e.to_string())?;
            lit_count(&read.try_finish().ok_or("readback did not finish")??)
        } else {
            0
        };
        r.gpu.queue.present(surface);
        if let Some(smoke) = self.smoke.as_mut() {
            match smoke.finish_frame(now_ms, visible, vtx, lit)? {
                Step::SendKeys => eprintln!("isekai-smoke: ready"),
                Step::Complete(report) => {
                    self.result = Ok(Some(report.line(r.gpu.backend())));
                    self.done = true;
                }
                Step::Continue => {}
            }
        }
        Ok(())
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.render.is_none()
            && let Err(e) = self.init(event_loop)
        {
            self.fail(event_loop, e);
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if self.done {
            return;
        }
        #[cfg(feature = "dev")]
        if let Some(r) = self.render.as_mut() {
            if let WindowEvent::KeyboardInput { event, .. } = &event {
                r.dev.on_key(&crate::input::convert(event, self.modifiers));
            }
            r.dev.on_window_event(&r.window, &event);
        }
        match event {
            WindowEvent::CloseRequested => {
                if self.smoke.is_some() {
                    self.fail(event_loop, "window closed before smoke completed".into());
                } else {
                    self.done = true;
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(r) = self.render.as_mut() {
                    r.target.resize(&r.gpu, size.width, size.height);
                }
            }
            WindowEvent::ModifiersChanged(m) => {
                let m = m.state();
                self.modifiers = Modifiers {
                    control: m.control_key(),
                    alt: m.alt_key(),
                    super_key: m.super_key(),
                    shift: m.shift_key(),
                };
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let key = crate::input::convert(&event, self.modifiers);
                if key.key == Key::Backquote
                    && key.pressed
                    && !key.repeat
                    && let Some(smoke) = self.smoke.as_mut()
                {
                    smoke.observe_key_probe();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.frame() {
                    self.fail(event_loop, e);
                }
                if self.done {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if !self.done
            && let Some(r) = &self.render
        {
            r.window.request_redraw();
        }
    }
}
