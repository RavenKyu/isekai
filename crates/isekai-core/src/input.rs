#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Backquote,
    Other,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Modifiers {
    pub control: bool,
    pub alt: bool,
    pub super_key: bool,
    pub shift: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct KeyInput {
    pub key: Key,
    pub pressed: bool,
    pub repeat: bool,
    pub modifiers: Modifiers,
}
