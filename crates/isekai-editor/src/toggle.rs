use isekai_core::input::{Key, KeyInput};
pub fn is_toggle(input: &KeyInput, wants_keyboard: bool) -> bool {
    input.key == Key::Backquote
        && input.pressed
        && !input.repeat
        && !input.modifiers.control
        && !input.modifiers.alt
        && !input.modifiers.super_key
        && !wants_keyboard
}
#[cfg(test)]
mod tests {
    use super::*;
    use isekai_core::input::Modifiers;
    #[test]
    fn toggle_table() {
        let base = KeyInput {
            key: Key::Backquote,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::default(),
        };
        assert!(is_toggle(&base, false));
        assert!(!is_toggle(&base, true));
        for key in [
            KeyInput {
                key: Key::Other,
                ..base
            },
            KeyInput {
                pressed: false,
                ..base
            },
            KeyInput {
                repeat: true,
                ..base
            },
            KeyInput {
                modifiers: Modifiers {
                    control: true,
                    ..Default::default()
                },
                ..base
            },
            KeyInput {
                modifiers: Modifiers {
                    alt: true,
                    ..Default::default()
                },
                ..base
            },
            KeyInput {
                modifiers: Modifiers {
                    super_key: true,
                    ..Default::default()
                },
                ..base
            },
        ] {
            assert!(!is_toggle(&key, false));
        }
        assert!(is_toggle(
            &KeyInput {
                modifiers: Modifiers {
                    shift: true,
                    ..Default::default()
                },
                ..base
            },
            false
        ));
    }
}
