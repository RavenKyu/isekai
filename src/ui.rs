use dear_imgui_rs::{Condition, Ui, WindowFlags};

pub struct DevTools {
    frame_stats: bool,
    demo: bool,
    metrics: bool,
}

impl Default for DevTools {
    fn default() -> Self {
        Self {
            frame_stats: true,
            demo: false,
            metrics: false,
        }
    }
}

impl DevTools {
    pub fn draw(&mut self, ui: &Ui) {
        self.menu_bar(ui);
        if self.frame_stats {
            ui.window("Frame stats")
                .opened(&mut self.frame_stats)
                .position([12.0, 36.0], Condition::FirstUseEver)
                .flags(WindowFlags::ALWAYS_AUTO_RESIZE)
                .build(|| ui.text(frame_stats_text(ui.io().framerate())));
        }
        if self.demo {
            ui.show_demo_window(&mut self.demo);
        }
        if self.metrics {
            ui.show_metrics_window(&mut self.metrics);
        }
    }

    fn menu_bar(&mut self, ui: &Ui) {
        ui.main_menu_bar(|| {
            ui.menu("View", || {
                ui.menu_item_toggle("Frame stats", None::<&str>, &mut self.frame_stats, true);
                ui.menu_item_toggle("Dear ImGui Demo", None::<&str>, &mut self.demo, true);
                ui.menu_item_toggle("Dear ImGui Metrics", None::<&str>, &mut self.metrics, true);
            });
            // Add future Info/Settings entries here, with their window draw in draw().
        });
    }
}

fn frame_stats_text(framerate: f32) -> String {
    let milliseconds = 1000.0 / framerate;
    if framerate.is_finite() && framerate > 0.0 && milliseconds.is_finite() {
        format!("{framerate:.1} FPS\n{milliseconds:.2} ms / frame")
    } else {
        "-- FPS\n-- ms / frame".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dear_imgui_rs::{BackendFlags, Context, TextureId, render::TextureOp};

    #[test]
    fn statistics_handle_startup_and_invalid_rates() {
        assert_eq!(frame_stats_text(60.0), "60.0 FPS\n16.67 ms / frame");
        for rate in [
            0.0,
            -1.0,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::MIN_POSITIVE / 100.0,
        ] {
            assert_eq!(frame_stats_text(rate), "-- FPS\n-- ms / frame");
        }
        assert!(frame_stats_text(10_000.0).contains("0.10 ms"));
    }

    #[test]
    fn menus_and_each_window_contribute_geometry() {
        let mut context = Context::create();
        context.set_ini_filename(None::<String>).unwrap();
        context.io_mut().set_display_size([1280.0, 720.0]);
        context.io_mut().set_delta_time(1.0 / 60.0);
        context
            .io_mut()
            .set_backend_flags(BackendFlags::RENDERER_HAS_TEXTURES);
        let consumer = context.create_synchronous_renderer_consumer().unwrap();
        let mut tools = DevTools::default();
        assert!(tools.frame_stats && !tools.demo && !tools.metrics);
        let mut vertices = |tools: &mut DevTools| {
            let mut count = 0;
            // Warm up font textures and settle new windows before comparisons.
            for _ in 0..4 {
                tools.draw(context.frame());
                let pending = context.render(&consumer);
                let feedback = pending
                    .texture_requests()
                    .iter()
                    .map(|request| match request.operation() {
                        TextureOp::Create { .. } | TextureOp::Update { .. } => {
                            request.uploaded(TextureId::new(1)).unwrap()
                        }
                        TextureOp::Destroy => request.destroyed().unwrap(),
                    })
                    .collect::<Vec<_>>();
                let reconciled = pending.reconcile_texture_feedback(feedback).unwrap();
                count = reconciled.draw_data().total_vtx_count();
            }
            count
        };
        let baseline = vertices(&mut tools);
        tools.frame_stats = false;
        let menu_only = vertices(&mut tools);
        assert!(
            menu_only > 0 && baseline > menu_only,
            "baseline={baseline}, menu={menu_only}"
        );
        tools.frame_stats = true;
        tools.demo = true;
        assert!(vertices(&mut tools) > baseline);
        tools.demo = false;
        assert_eq!(vertices(&mut tools), baseline);
        tools.metrics = true;
        assert!(vertices(&mut tools) > baseline);
        tools.metrics = false;
        assert_eq!(vertices(&mut tools), baseline);
        context
            .prepare_renderer_texture_reset(&consumer)
            .unwrap()
            .commit();
        drop(consumer);
        context.poll_snapshot_completions().unwrap();
    }
}
