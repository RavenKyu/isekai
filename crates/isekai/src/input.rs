use isekai_core::input::{Key, KeyInput, Modifiers};
pub fn convert(event: &winit::event::KeyEvent, modifiers: Modifiers) -> KeyInput {
    KeyInput {
        key: if event.physical_key
            == winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Backquote)
        {
            Key::Backquote
        } else {
            Key::Other
        },
        pressed: event.state.is_pressed(),
        repeat: event.repeat,
        modifiers,
    }
}
