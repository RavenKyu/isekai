use dear_imgui_rs::{Condition, Ui};

pub const HELLO_TEXT: &str = "Hello World";

pub fn hello_world(ui: &Ui) {
    hello_window(ui, || ui.text(HELLO_TEXT));
}

fn hello_window(ui: &Ui, contents: impl FnOnce()) {
    ui.window("Hello")
        .size([360.0, 160.0], Condition::FirstUseEver)
        .build(contents);
}

#[cfg(test)]
mod tests {
    use super::*;
    use dear_imgui_rs::{BackendFlags, Context, TextureId, render::TextureOp};

    #[test]
    fn hello_text_adds_vertices_to_the_same_empty_window() {
        assert_eq!(HELLO_TEXT, "Hello World");
        let mut context = Context::create();
        context.set_ini_filename(None::<String>).unwrap();
        context.io_mut().set_display_size([1280.0, 720.0]);
        context.io_mut().set_delta_time(1.0 / 60.0);
        context
            .io_mut()
            .set_backend_flags(BackendFlags::RENDERER_HAS_TEXTURES);
        let consumer = context.create_synchronous_renderer_consumer().unwrap();
        let mut vertices = Vec::new();
        // Warm up font textures and the window before comparing identical frames.
        for with_text in [true, true, false, true] {
            let ui = context.frame();
            if with_text {
                hello_world(ui);
            } else {
                hello_window(ui, || {});
            }
            let pending = context.render(&consumer);
            // This CPU-only recorder acknowledges uploads; no GL or platform is used.
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
            vertices.push(reconciled.draw_data().total_vtx_count());
        }
        assert!(
            vertices[3] > vertices[2],
            "text must add vertices: {vertices:?}"
        );
        context
            .prepare_renderer_texture_reset(&consumer)
            .unwrap()
            .commit();
        drop(consumer);
        context.poll_snapshot_completions().unwrap();
    }
}
