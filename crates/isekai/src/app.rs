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
pub(crate) struct Render {
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
    pending_read: Option<(Readback, u64, bool, usize)>,
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<UserEvent>>,
    #[cfg(target_arch = "wasm32")]
    initializing: bool,
    #[cfg(target_arch = "wasm32")]
    pending_events: Vec<(WindowId, WindowEvent)>,
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
            pending_read: None,
            #[cfg(target_arch = "wasm32")]
            proxy: None,
            #[cfg(target_arch = "wasm32")]
            initializing: false,
            #[cfg(target_arch = "wasm32")]
            pending_events: Vec::new(),
        }
    }
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: String) {
        if !self.done {
            self.result = Err(error);
            self.done = true;
        }
        #[cfg(target_arch = "wasm32")]
        if let Err(reason) = &self.result {
            crate::web::failure(reason);
        }
        #[cfg(not(target_arch = "wasm32"))]
        event_loop.exit();
        #[cfg(target_arch = "wasm32")]
        let _ = event_loop;
    }
    #[cfg(not(target_arch = "wasm32"))]
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
        self.render = Some(Render::new(window, gpu, surface, self.command.take())?);
        Ok(())
    }
    fn frame(&mut self) -> Result<(), String> {
        if let Some((read, now, visible, vtx)) = &self.pending_read {
            self.render
                .as_ref()
                .unwrap()
                .gpu
                .device
                .poll(wgpu::PollType::Poll)
                .map_err(|e| e.to_string())?;
            let Some(pixels) = read.try_finish() else {
                return Ok(());
            };
            let (now, visible, vtx) = (*now, *visible, *vtx);
            self.pending_read = None;
            return self.finish_smoke(now, visible, vtx, lit_count(&pixels?));
        }
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
        r.gpu.queue.present(surface);
        if self
            .smoke
            .as_ref()
            .is_some_and(|s| s.is_last_frame(visible))
        {
            self.pending_read = Some((
                Readback::begin(&r.gpu, &r.target.frame),
                now_ms,
                visible,
                vtx,
            ));
            #[cfg(not(target_arch = "wasm32"))]
            r.gpu
                .device
                .poll(wgpu::PollType::wait_indefinitely())
                .map_err(|e| e.to_string())?;
            return Ok(());
        }
        self.finish_smoke(now_ms, visible, vtx, 0)?;
        Ok(())
    }
    fn finish_smoke(
        &mut self,
        now: u64,
        visible: bool,
        vtx: usize,
        lit: usize,
    ) -> Result<(), String> {
        if let Some(smoke) = self.smoke.as_mut() {
            match smoke.finish_frame(now, visible, vtx, lit)? {
                Step::SendKeys => {
                    #[cfg(not(target_arch = "wasm32"))]
                    eprintln!("isekai-smoke: ready");
                    #[cfg(target_arch = "wasm32")]
                    crate::web::ready();
                }
                Step::Complete(report) => {
                    let line = report.line(self.render.as_ref().unwrap().gpu.backend());
                    #[cfg(target_arch = "wasm32")]
                    crate::web::output(&line);
                    self.result = Ok(Some(line));
                    self.done = true;
                }
                Step::Continue => {}
            }
        } else {
            #[cfg(target_arch = "wasm32")]
            crate::web::ready();
        }
        Ok(())
    }
    #[cfg(target_arch = "wasm32")]
    pub fn set_proxy(&mut self, proxy: winit::event_loop::EventLoopProxy<UserEvent>) {
        self.proxy = Some(proxy);
    }
}
pub(crate) enum UserEvent {
    #[cfg(target_arch = "wasm32")]
    Ready(Result<Render, String>),
}
impl Render {
    fn new(
        window: Arc<Window>,
        gpu: Gpu,
        surface: wgpu::Surface<'static>,
        command: Option<Command>,
    ) -> Result<Self, String> {
        let size = window.inner_size();
        let target = SurfaceTarget::new(&gpu, surface, size.width, size.height)?;
        let pipeline = SpritePipeline::new(&gpu);
        let batch = SpriteBatch::new(&gpu);
        let playback = match command {
            Some(Command::Play { source, animation }) => {
                #[cfg(not(target_arch = "wasm32"))]
                let playback = Playback::open_native(
                    std::path::Path::new(&source),
                    animation.as_deref(),
                    &gpu,
                    &pipeline,
                )?;
                #[cfg(target_arch = "wasm32")]
                let playback = {
                    if source != "example" {
                        return Err(format!("no embedded sprite sheet named {source:?}"));
                    }
                    let (image, sheet) = isekai_core::sprite::example();
                    Playback::from_image(image, sheet, animation.as_deref(), &gpu, &pipeline)?
                };
                Some(playback)
            }
            None => None,
        };
        #[cfg(feature = "dev")]
        let dev = isekai_editor::DevShell::new(&gpu, &window);
        Ok(Render {
            #[cfg(feature = "dev")]
            dev,
            playback,
            batch,
            pipeline,
            target,
            gpu,
            window,
        })
    }
}
impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[cfg(not(target_arch = "wasm32"))]
        if self.render.is_none()
            && let Err(e) = self.init(event_loop)
        {
            self.fail(event_loop, e);
        }
        #[cfg(target_arch = "wasm32")]
        if self.render.is_none() && !self.initializing {
            self.initializing = true;
            let result = crate::web::window(event_loop);
            match result {
                Err(e) => self.fail(event_loop, e),
                Ok(window) => {
                    let proxy = self.proxy.as_ref().unwrap().clone();
                    let command = self.command.take();
                    wasm_bindgen_futures::spawn_local(async move {
                        let result = async {
                            let (gpu, surface) = crate::web::gpu(window.clone()).await?;
                            Render::new(window, gpu, surface, command)
                        }
                        .await;
                        let _ = proxy.send_event(UserEvent::Ready(result));
                    });
                }
            }
        }
    }
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (event_loop, event);
        #[cfg(target_arch = "wasm32")]
        match event {
            UserEvent::Ready(result) => match result {
                Ok(render) => {
                    self.render = Some(render);
                    #[cfg(target_arch = "wasm32")]
                    for (id, event) in std::mem::take(&mut self.pending_events) {
                        self.window_event(event_loop, id, event);
                    }
                }
                Err(e) => self.fail(event_loop, e),
            },
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if self.done {
            return;
        }
        #[cfg(target_arch = "wasm32")]
        if self.render.is_none() {
            self.pending_events.push((_id, event));
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
                #[cfg(not(target_arch = "wasm32"))]
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
