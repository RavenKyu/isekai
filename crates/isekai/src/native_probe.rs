//! Smoke-only diagnostics for the actual native event path.
use isekai_core::input::{Key, KeyInput};
#[derive(Default)]
pub struct NativeProbe {
    focused: bool,
    presented: bool,
    ready: bool,
    presses: u32,
    repeats: u32,
    releases: u32,
    toggles: u32,
}
impl NativeProbe {
    pub fn focused(&mut self, focused: bool) {
        self.focused = focused;
        eprintln!("isekai-smoke-stage: focused={focused}");
        self.ready();
    }
    pub fn presented(&mut self) {
        if !self.presented {
            self.presented = true;
            eprintln!("isekai-smoke-stage: first-frame");
            self.ready();
        }
    }
    fn ready(&mut self) {
        if self.focused && self.presented && !self.ready {
            self.ready = true;
            eprintln!("isekai-smoke: ready");
        }
    }
    pub fn key(&mut self, key: &KeyInput, toggled: bool) {
        if key.key != Key::Backquote {
            return;
        }
        if key.pressed {
            if key.repeat {
                self.repeats += 1;
            } else {
                self.presses += 1;
            }
        } else {
            self.releases += 1;
        }
        if self.presses + self.repeats + self.releases == 1 {
            eprintln!("isekai-smoke-stage: key-received");
        }
        if key.repeat && self.repeats == 1 {
            eprintln!("isekai-smoke-stage: repeat-received");
        }
        if toggled {
            self.toggles += 1;
            eprintln!("isekai-smoke-stage: tools-toggled count={}", self.toggles);
        }
        if !key.pressed {
            eprintln!(
                "isekai-smoke-input: presses={} repeats={} releases={} toggles={}",
                self.presses, self.repeats, self.releases, self.toggles
            );
        }
    }
    pub fn missing_stage(&self) -> &'static str {
        if !self.presented {
            "first-frame"
        } else if !self.focused {
            "focused"
        } else if self.presses == 0 {
            "key-received"
        } else if self.toggles == 0 && cfg!(feature = "dev") {
            "tools-toggled"
        } else {
            "completion"
        }
    }
}
