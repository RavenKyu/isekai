pub struct DevTools {
    pub visible: bool,
    pub stats: bool,
    pub inspection: bool,
    pub settings: bool,
}
impl Default for DevTools {
    fn default() -> Self {
        Self {
            visible: false,
            stats: true,
            inspection: false,
            settings: false,
        }
    }
}
pub fn frame_stats_text(fps: f32) -> String {
    let ms = 1000.0 / fps;
    if fps.is_finite() && fps > 0.0 && ms.is_finite() {
        format!("{fps:.1} FPS\n{ms:.2} ms / frame")
    } else {
        "-- FPS\n-- ms / frame".into()
    }
}
impl DevTools {
    pub fn draw(&mut self, ctx: &egui::Context, fps: f32) {
        if !self.visible {
            return;
        }
        ctx.root_ui(|root| {
            egui::Panel::top("menu").show(root, |ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    ui.menu_button("View", |ui| {
                        ui.checkbox(&mut self.stats, "Frame stats");
                        ui.checkbox(&mut self.inspection, "egui Inspection");
                        ui.checkbox(&mut self.settings, "egui Settings");
                    });
                });
            });
        });
        egui::Window::new("Frame stats")
            .open(&mut self.stats)
            .show(ctx, |ui| {
                ui.label(frame_stats_text(fps));
            });
        egui::Window::new("egui Inspection")
            .open(&mut self.inspection)
            .show(ctx, |ui| ctx.inspection_ui(ui));
        egui::Window::new("egui Settings")
            .open(&mut self.settings)
            .show(ctx, |ui| ctx.settings_ui(ui));
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn stats() {
        assert_eq!(super::frame_stats_text(60.), "60.0 FPS\n16.67 ms / frame");
        for n in [0., -1., f32::NAN, f32::INFINITY, f32::MIN_POSITIVE] {
            assert_eq!(super::frame_stats_text(n), "-- FPS\n-- ms / frame");
        }
        assert!(super::frame_stats_text(10000.).contains("0.10 ms"));
    }
}

#[cfg(test)]
mod geometry_tests {
    use super::*;
    fn frame(ctx: &egui::Context, tools: &mut DevTools) -> usize {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280., 720.),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_pass(input, |ctx| tools.draw(ctx, 60.));
        output.textures_delta.clear();
        ctx.tessellate(output.shapes, output.pixels_per_point)
            .iter()
            .map(|p| match &p.primitive {
                egui::epaint::Primitive::Mesh(m) => m.vertices.len(),
                _ => 0,
            })
            .sum()
    }
    fn stable(ctx: &egui::Context, tools: &mut DevTools) -> usize {
        for _ in 0..5 {
            frame(ctx, tools);
        }
        frame(ctx, tools)
    }
    #[test]
    fn windows_and_visibility() {
        let ctx = crate::fonts::context();
        ctx.style_mut_of(egui::Theme::Dark, |s| s.animation_time = 0.);
        ctx.style_mut_of(egui::Theme::Light, |s| s.animation_time = 0.);
        let mut tools = DevTools::default();
        assert_eq!(stable(&ctx, &mut tools), 0);
        tools.visible = true;
        tools.stats = false;
        let menu = stable(&ctx, &mut tools);
        assert!(menu > 0);
        tools.stats = true;
        let stats = stable(&ctx, &mut tools);
        assert!(stats > menu);
        tools.inspection = true;
        assert!(stable(&ctx, &mut tools) > stats);
        tools.inspection = false;
        tools.settings = true;
        assert!(stable(&ctx, &mut tools) > stats);
        tools.settings = false;
        assert_eq!(stable(&ctx, &mut tools), stats);
        tools.visible = false;
        assert_eq!(stable(&ctx, &mut tools), 0);
        tools.visible = true;
        assert_eq!(stable(&ctx, &mut tools), stats);
    }
    #[test]
    fn hidden_frame_releases_text_focus() {
        let ctx = crate::fonts::context();
        let id = egui::Id::unique("text");
        let mut text = String::new();
        let mut output = ctx.run_pass(Default::default(), |ctx| {
            egui::Window::new("input").show(ctx, |ui| {
                ui.add(egui::TextEdit::singleline(&mut text).id(id))
                    .request_focus();
            });
        });
        output.textures_delta.clear();
        drop(output);
        assert!(ctx.egui_wants_keyboard_input());
        let key = isekai_core::input::KeyInput {
            key: isekai_core::input::Key::Backquote,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        };
        assert!(!crate::toggle::is_toggle(
            &key,
            ctx.egui_wants_keyboard_input()
        ));
        let mut tools = DevTools::default();
        stable(&ctx, &mut tools);
        assert!(!ctx.egui_wants_keyboard_input());
        assert!(crate::toggle::is_toggle(
            &key,
            ctx.egui_wants_keyboard_input()
        ));
    }
}
