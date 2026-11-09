use dear_imgui_rs::{StyleColor, StyleVar, Ui, WindowFlags};

pub(super) const LIMIT: usize = 500;
pub(super) const HEIGHT: f32 = 120.0;

fn timestamp() -> String {
    let mut ticks = 0;
    let mut date = sdl3::sys::time::SDL_DateTime::default();
    // SAFETY: both SDL functions write into live, correctly typed stack values.
    // They need no SDL subsystem and permit calls from any thread.
    if unsafe {
        sdl3::sys::time::SDL_GetCurrentTime(&mut ticks)
            && sdl3::sys::time::SDL_TimeToDateTime(ticks, &mut date, true)
    } {
        format!("{:02}:{:02}:{:02}", date.hour, date.minute, date.second)
    } else {
        "??:??:??".into()
    }
}

#[derive(Clone, Default, PartialEq, Debug)]
pub struct SheetLog {
    lines: Vec<String>,
    generation: u64,
    repeat: Option<(&'static str, u64)>,
}

impl std::ops::Deref for SheetLog {
    type Target = [String];
    fn deref(&self) -> &Self::Target {
        &self.lines
    }
}

impl SheetLog {
    fn clear(&mut self) {
        self.lines.clear();
        self.repeat = None;
        self.generation = self.generation.wrapping_add(1);
    }
}

pub(super) fn append(log: &mut SheetLog, message: String) {
    log.repeat = None;
    let lines = &mut log.lines;
    let time = timestamp();
    // One timestamp per physical line; embedded NULs cannot truncate InputText.
    for line in message.replace('\0', " ").lines() {
        lines.push(format!("{time} {}", line.trim_end_matches('\r')));
    }
    if lines.len() > LIMIT {
        lines.drain(..lines.len() - LIMIT);
    }
    log.generation = log.generation.wrapping_add(1);
}

// Only calls to this function can continue a run. Every ordinary append,
// including an error or save, terminates it even when its text happens to match.
pub(super) fn append_repeat(log: &mut SheetLog, message: &'static str) {
    if let Some((previous, count)) = log.repeat
        && previous == message
    {
        let count = count.saturating_add(1);
        *log.lines.last_mut().expect("repeat run has a line") =
            format!("{} {message} (x{count})", timestamp());
        log.repeat = Some((message, count));
        log.generation = log.generation.wrapping_add(1);
    } else {
        append(log, message.into());
        log.repeat = Some((message, 1));
    }
}

#[derive(Default)]
pub(super) struct LogView {
    text: String,
    seen: Option<u64>,
    width: f32,
    line_height: f32,
    #[cfg(test)]
    joins: usize,
    #[cfg(test)]
    measurements: usize,
    #[cfg(test)]
    metrics: [f32; 4],
    #[cfg(test)]
    clear_center: [f32; 2],
    #[cfg(test)]
    scroll_max_x: f32,
}
impl LogView {
    pub fn draw(&mut self, lines: &mut SheetLog, ui: &Ui) {
        ui.separator();
        ui.text("Log");
        ui.same_line();
        if ui.small_button("Clear") {
            lines.clear();
        }
        #[cfg(test)]
        {
            let min = ui.item_rect_min();
            let size = ui.item_rect_size();
            self.clear_center = [min[0] + size[0] * 0.5, min[1] + size[1] * 0.5];
        }
        let changed = self.seen != Some(lines.generation);
        if changed {
            self.text = lines.join("\n");
            self.seen = Some(lines.generation);
            #[cfg(test)]
            {
                self.joins += 1;
            }
        }
        let line_height = ui.text_line_height();
        if changed || self.line_height != line_height {
            self.width = ui.calc_text_size(&self.text)[0] + 16.0;
            self.line_height = line_height;
            #[cfg(test)]
            {
                self.measurements += 1;
            }
        }
        let _background = ui.push_style_color(StyleColor::ChildBg, [0.0, 0.0, 0.0, 1.0]);
        let _frame = ui.push_style_color(StyleColor::FrameBg, [0.0, 0.0, 0.0, 1.0]);
        let _hover = ui.push_style_color(StyleColor::FrameBgHovered, [0.0, 0.0, 0.0, 1.0]);
        let _active = ui.push_style_color(StyleColor::FrameBgActive, [0.0, 0.0, 0.0, 1.0]);
        let _text = ui.push_style_color(StyleColor::Text, [1.0; 4]);
        let _padding = ui.push_style_var(StyleVar::FramePadding([4.0, 4.0]));
        ui.child_window("Sheet log")
            .size([0.0, HEIGHT])
            .flags(WindowFlags::HORIZONTAL_SCROLLBAR)
            .build(ui, || {
                let at_bottom = ui.scroll_y() >= ui.scroll_max_y() - 1.0;
                #[cfg(test)]
                {
                    self.scroll_max_x = ui.scroll_max_x();
                    let pos = ui.window_pos();
                    let size = ui.window_size();
                    self.metrics = [
                        ui.scroll_y(),
                        ui.scroll_max_y(),
                        pos[0] + size[0] * 0.5,
                        pos[1] + size[1] * 0.5,
                    ];
                }
                // The outer child owns scrolling. A full-height input permits
                // selecting/copying across lines without a second vertical scrollbar.
                let width = self.width.max(ui.content_region_avail()[0]);
                let height = (lines.len().max(1) as f32 * ui.text_line_height()) + 16.0;
                // Any frame that shows (or copies) a stale cache fails here,
                // including the frame in which Clear ran.
                #[cfg(test)]
                assert_eq!(self.text, lines.join("\n"), "log widget shows stale text");
                ui.input_text_multiline("##log text", &mut self.text, [width, height])
                    .read_only(true)
                    .build();
                if changed && at_bottom {
                    ui.set_scroll_here_y(1.0);
                }
            });
    }
}

#[cfg(test)]
impl LogView {
    pub(super) fn check_interactions(
        context: &mut dear_imgui_rs::Context,
        consumer: &dear_imgui_rs::render::SynchronousRendererConsumer,
    ) {
        use dear_imgui_rs::{Condition, Key, MouseButton};
        use std::{cell::RefCell, rc::Rc};
        struct Clipboard(Rc<RefCell<String>>);
        impl dear_imgui_rs::ClipboardBackend for Clipboard {
            fn get(&mut self) -> Option<String> {
                Some(self.0.borrow().clone())
            }
            fn set(&mut self, value: &str) {
                *self.0.borrow_mut() = value.into();
            }
        }
        let clipboard = Rc::new(RefCell::new(String::new()));
        context.set_clipboard_backend(Clipboard(Rc::clone(&clipboard)));
        let view = RefCell::new(Self::default());
        let lines = RefCell::new(SheetLog::default());
        let render = |context: &mut dear_imgui_rs::Context| {
            super::tests::vertices(context, consumer, |ui| {
                ui.window("Log interactions")
                    .size([600.0, 300.0], Condition::Always)
                    .position([20.0, 20.0], Condition::Always)
                    .build(|| view.borrow_mut().draw(&mut lines.borrow_mut(), ui));
            });
        };
        for i in 0..40 {
            append(&mut lines.borrow_mut(), format!("event {i}"));
        }
        render(context);
        assert_eq!(view.borrow().joins, 1);
        assert_eq!(view.borrow().measurements, 1);
        let [y, max, x, mouse_y] = view.borrow().metrics;
        assert!(
            max > 0.0 && y >= max - 1.0,
            "new lines follow the bottom: {y}/{max}"
        );
        context.io_mut().add_mouse_pos_event([x, mouse_y]);
        render(context);
        context.io_mut().add_mouse_wheel_event([0.0, 5.0]);
        render(context);
        let [before, max, ..] = view.borrow().metrics;
        assert!(before < max - 1.0, "wheel scrolls the outer log child");
        append(&mut lines.borrow_mut(), "while reading older lines".into());
        render(context);
        assert_eq!(view.borrow().joins, 2);
        assert_eq!(view.borrow().measurements, 2);
        let after = view.borrow().metrics[0];
        assert!(
            (before - after).abs() < 1.0,
            "new log must keep reading position: {before} -> {after}"
        );
        append(
            &mut lines.borrow_mut(),
            "wide ".to_owned() + &"w".repeat(300),
        );
        render(context);
        assert!(
            view.borrow().scroll_max_x > 0.0,
            "long lines grow the horizontal scroll range"
        );
        let (joins, measurements, width) = {
            let view = view.borrow();
            (view.joins, view.measurements, view.width)
        };
        let scale = context.style().font_scale_main();
        context.style_mut().set_font_scale_main(scale * 2.0);
        render(context);
        {
            let view = view.borrow();
            assert_eq!(view.joins, joins, "a font change keeps the joined text");
            assert_eq!(
                view.measurements,
                measurements + 1,
                "a font change re-measures the width once"
            );
            assert!(
                view.width > width * 1.5,
                "width follows the font: {width} -> {}",
                view.width
            );
        }
        context.style_mut().set_font_scale_main(scale);
        render(context);
        for i in 0..LIMIT {
            append(&mut lines.borrow_mut(), format!("new {i}"));
        }
        render(context);
        assert!(
            !view.borrow().text.contains("event 0"),
            "eviction refreshes cached text"
        );
        append_repeat(&mut lines.borrow_mut(), "Tile metadata updated");
        render(context);
        append_repeat(&mut lines.borrow_mut(), "Tile metadata updated");
        render(context);
        assert!(view.borrow().text.ends_with("Tile metadata updated (x2)"));
        context
            .io_mut()
            .add_mouse_button_event(MouseButton::Left, true);
        render(context);
        context
            .io_mut()
            .add_mouse_button_event(MouseButton::Left, false);
        render(context);
        context.io_mut().add_key_event(Key::ModCtrl, true);
        context.io_mut().add_key_event(Key::A, true);
        render(context);
        context.io_mut().add_key_event(Key::A, false);
        render(context);
        context.io_mut().add_key_event(Key::C, true);
        render(context);
        context.io_mut().add_key_event(Key::C, false);
        context.io_mut().add_key_event(Key::ModCtrl, false);
        render(context);
        assert_eq!(
            *clipboard.borrow(),
            lines.borrow().join("\n"),
            "read-only text can be selected and copied"
        );
        context.io_mut().add_key_event(Key::Backspace, true);
        render(context);
        context.io_mut().add_key_event(Key::Backspace, false);
        render(context);
        assert_eq!(
            view.borrow().text,
            lines.borrow().join("\n"),
            "typing cannot edit the log"
        );
        let clear = view.borrow().clear_center;
        context.io_mut().add_mouse_pos_event(clear);
        render(context);
        context
            .io_mut()
            .add_mouse_button_event(MouseButton::Left, true);
        render(context);
        context
            .io_mut()
            .add_mouse_button_event(MouseButton::Left, false);
        render(context);
        assert!(lines.borrow().is_empty(), "Clear empties only this log");
        assert!(
            view.borrow().text.is_empty(),
            "Clear updates the cached text"
        );
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    #[test]
    fn generation_tracks_all_mutations_including_eviction_and_clear() {
        let mut log = SheetLog::default();
        assert_eq!(log.generation, 0);
        append(&mut log, "first".into());
        assert_eq!(log.generation, 1);
        for i in 1..LIMIT {
            append(&mut log, format!("{i}"));
        }
        let before = log.generation;
        let _ = log.len();
        assert_eq!(log.generation, before);
        append(&mut log, "eviction".into());
        assert_eq!(log.generation, before + 1);
        assert_eq!(log.len(), LIMIT);
        assert!(!log[0].ends_with(" first"));
        log.clear();
        assert_eq!(log.generation, before + 2);
        assert!(log.is_empty());
    }

    #[test]
    fn repeated_edits_preserve_prior_lines_and_stop_at_other_messages() {
        let mut log = SheetLog::default();
        append(&mut log, "Saved x".into());
        for i in 1..498 {
            append(&mut log, format!("event {i}"));
        }
        append_repeat(&mut log, "Grid updated");
        let generation = log.generation;
        for _ in 1..20 {
            append_repeat(&mut log, "Grid updated");
        }
        assert_eq!(log.len(), 499);
        assert!(log[0].ends_with(" Saved x"));
        assert!(log.last().unwrap().ends_with("Grid updated (x20)"));
        assert_eq!(log.generation, generation + 19);
        append(&mut log, "validation error".into());
        append_repeat(&mut log, "Grid updated");
        assert!(log[497].ends_with("Grid updated (x20)"));
        assert!(log[498].ends_with("validation error"));
        assert!(log[500 - 1].ends_with("Grid updated"));
    }

    #[test]
    fn clear_ends_a_run_and_each_log_keeps_its_own_run() {
        let mut log = SheetLog::default();
        append_repeat(&mut log, "Animation updated");
        append_repeat(&mut log, "Animation updated");
        log.clear();
        append_repeat(&mut log, "Animation updated");
        assert_eq!(log.len(), 1);
        assert!(log[0].ends_with(" Animation updated"), "{:?}", log[0]);
        let mut other = SheetLog::default();
        append(&mut other, "Saved other".into());
        append_repeat(&mut other, "Animation updated");
        append_repeat(&mut log, "Animation updated");
        assert!(log[0].ends_with(" Animation updated (x2)"), "{:?}", log[0]);
        assert_eq!(other.len(), 2);
        assert!(other[0].ends_with(" Saved other"));
        assert!(other[1].ends_with(" Animation updated"));
    }

    #[test]
    fn logs_keep_the_latest_500_timestamped_lines_and_normalize_control_characters() {
        let mut lines = SheetLog::default();
        for i in 0..LIMIT + 3 {
            append(&mut lines, format!("line {i}"));
        }
        assert_eq!(lines.len(), LIMIT);
        assert!(lines[0].ends_with(" line 3"));
        assert!(lines.last().unwrap().ends_with(" line 502"));
        let prefix = &lines[0][..8];
        assert_eq!(&prefix[2..3], ":");
        assert_eq!(&prefix[5..6], ":");
        assert!(prefix.replace(':', "").chars().all(|c| c.is_ascii_digit()));
        append(&mut lines, "first\r\nsecond\0last".into());
        assert_eq!(lines.len(), LIMIT);
        assert!(lines[LIMIT - 2].ends_with(" first"));
        assert!(lines[LIMIT - 1].ends_with(" second last"));
    }
}
