pub mod toggle;
pub mod tools;
use isekai_render::{FRAME_FORMAT, FrameTarget, Gpu};
use winit::{event::WindowEvent, window::Window};
pub struct DevShell {
    pub ctx: egui::Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
    pub tools: tools::DevTools,
    previous_time: Option<f64>,
}
impl DevShell {
    pub fn new(gpu: &Gpu, window: &Window) -> Self {
        let ctx = egui::Context::default();
        let state = egui_winit::State::new(
            ctx.clone(),
            egui::ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            window.theme(),
            Some(gpu.device.limits().max_texture_dimension_2d as usize),
        );
        let renderer = egui_wgpu::Renderer::new(&gpu.device, FRAME_FORMAT, Default::default());
        Self {
            ctx,
            state,
            renderer,
            tools: Default::default(),
            previous_time: None,
        }
    }
    pub fn on_key(&mut self, key: &isekai_core::input::KeyInput) {
        if toggle::is_toggle(key, self.ctx.egui_wants_keyboard_input()) {
            self.tools.visible = !self.tools.visible;
        }
    }
    pub fn on_window_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        self.state.on_window_event(window, event).repaint
    }
    pub fn paint(
        &mut self,
        gpu: &Gpu,
        window: &Window,
        now: f64,
        encoder: &mut wgpu::CommandEncoder,
        frame: &FrameTarget,
    ) -> usize {
        let fps = self.previous_time.map_or(0., |t| (1. / (now - t)) as f32);
        self.previous_time = Some(now);
        let mut input = self.state.take_egui_input(window);
        input.time = Some(now);
        let mut output = self.ctx.run_pass(input, |ctx| self.tools.draw(ctx, fps));
        self.state
            .handle_platform_output(window, output.platform_output);
        let jobs = self.ctx.tessellate(output.shapes, output.pixels_per_point);
        let vtx = jobs
            .iter()
            .map(|p| match &p.primitive {
                egui::epaint::Primitive::Mesh(m) => m.vertices.len(),
                _ => 0,
            })
            .sum();
        for (id, deltas) in &output.textures_delta.set {
            for delta in deltas {
                self.renderer
                    .update_texture(&gpu.device, &gpu.queue, *id, delta);
            }
        }
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [frame.width, frame.height],
            pixels_per_point: output.pixels_per_point,
        };
        let callbacks =
            self.renderer
                .update_buffers(&gpu.device, &gpu.queue, encoder, &jobs, &screen);
        gpu.queue.submit(callbacks);
        {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("isekai egui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            self.renderer
                .render(&mut pass.forget_lifetime(), &jobs, &screen);
        }
        for id in &output.textures_delta.free {
            self.renderer.free_texture(id);
        }
        output.textures_delta.clear();
        vtx
    }
}
