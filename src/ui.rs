#[cfg(not(target_os = "emscripten"))]
pub mod asset_library;

use dear_imgui_rs::{Condition, Ui, WindowFlags};

pub struct DevTools {
    #[cfg(not(target_os = "emscripten"))]
    pub library: asset_library::AssetLibrary,
    visible: bool,
    frame_stats: bool,
    demo: bool,
    metrics: bool,
}

impl Default for DevTools {
    fn default() -> Self {
        Self {
            #[cfg(not(target_os = "emscripten"))]
            library: asset_library::AssetLibrary::default(),
            visible: false,
            frame_stats: true,
            demo: false,
            metrics: false,
        }
    }
}

impl DevTools {
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn draw(&mut self, ui: &Ui, now_ms: u64) {
        #[cfg(target_os = "emscripten")]
        let _ = now_ms;
        if !self.visible {
            return;
        }
        self.menu_bar(ui);
        #[cfg(not(target_os = "emscripten"))]
        self.library.draw(ui, now_ms);
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
                ui.menu_item_toggle_no_shortcut("Frame stats", &mut self.frame_stats, true);
                ui.menu_item_toggle_no_shortcut("Dear ImGui Demo", &mut self.demo, true);
                ui.menu_item_toggle_no_shortcut("Dear ImGui Metrics", &mut self.metrics, true);
            });
            #[cfg(not(target_os = "emscripten"))]
            ui.menu("Assets", || {
                ui.menu_item_toggle_no_shortcut("Sprite Library", &mut self.library.open, true);
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
        // The app's own context setup: no ImGui path may write a file.
        let mut context = crate::dev::context().unwrap();
        assert!(context.io().ini_filename().is_none() && context.io().log_filename().is_none());
        context.io_mut().set_display_size([1280.0, 720.0]);
        context.io_mut().set_delta_time(1.0 / 60.0);
        context
            .io_mut()
            .set_backend_flags(BackendFlags::RENDERER_HAS_TEXTURES);
        let consumer = context.create_synchronous_renderer_consumer().unwrap();
        let mut tools = DevTools::default();
        assert!(tools.frame_stats && !tools.demo && !tools.metrics);
        fn record(
            context: &mut Context,
            consumer: &dear_imgui_rs::render::SynchronousRendererConsumer,
            draw: impl FnOnce(&Ui),
        ) -> usize {
            draw(context.frame());
            let pending = context.render(consumer);
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
            reconciled.draw_data().total_vtx_count()
        }
        fn vertices(
            context: &mut Context,
            consumer: &dear_imgui_rs::render::SynchronousRendererConsumer,
            tools: &mut DevTools,
        ) -> usize {
            let mut count = 0;
            // Warm up font textures and settle new windows before comparisons.
            for _ in 0..4 {
                count = record(context, consumer, |ui| tools.draw(ui, 0));
            }
            count
        }
        assert!(!tools.is_visible());
        assert_eq!(
            vertices(&mut context, &consumer, &mut tools),
            0,
            "new tools must draw no UI"
        );
        tools.toggle();
        let baseline = vertices(&mut context, &consumer, &mut tools);
        tools.frame_stats = false;
        let menu_only = vertices(&mut context, &consumer, &mut tools);
        assert!(
            menu_only > 0 && baseline > menu_only,
            "baseline={baseline}, menu={menu_only}"
        );
        tools.frame_stats = true;
        tools.demo = true;
        assert!(vertices(&mut context, &consumer, &mut tools) > baseline);
        tools.demo = false;
        assert_eq!(vertices(&mut context, &consumer, &mut tools), baseline);
        tools.metrics = true;
        assert!(vertices(&mut context, &consumer, &mut tools) > baseline);
        tools.metrics = false;
        assert_eq!(vertices(&mut context, &consumer, &mut tools), baseline);
        tools.toggle();
        assert_eq!(vertices(&mut context, &consumer, &mut tools), 0);
        tools.toggle();
        assert_eq!(
            vertices(&mut context, &consumer, &mut tools),
            baseline,
            "window states survive hiding"
        );
        // Exercise real ImGui text focus rather than assigning want_text_input.
        let mut text = String::new();
        for _ in 0..3 {
            record(&mut context, &consumer, |ui| {
                ui.window("Text input test").build(|| {
                    ui.set_keyboard_focus_here();
                    ui.input_text("Text", &mut text).build();
                });
            });
        }
        assert!(
            context.io().want_text_input(),
            "focused InputText requests text input"
        );
        assert!(!crate::dev::is_toggle(
            &crate::dev::tests::key(false, sdl3::keyboard::Mod::NOMOD),
            context.io().want_text_input()
        ));
        // Hidden frames omit InputText and release its active ID/IME request.
        tools.toggle();
        assert_eq!(vertices(&mut context, &consumer, &mut tools), 0);
        assert!(!context.io().want_text_input());
        assert!(crate::dev::is_toggle(
            &crate::dev::tests::key(false, sdl3::keyboard::Mod::NOMOD),
            context.io().want_text_input()
        ));
        context
            .prepare_renderer_texture_reset(&consumer)
            .unwrap()
            .commit();
        drop(consumer);
        context.poll_snapshot_completions().unwrap();
    }
}
