use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum KeyCode {
    // Letters
    A = 0,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    // Numbers
    D0,
    D1,
    D2,
    D3,
    D4,
    D5,
    D6,
    D7,
    D8,
    D9,
    // Function keys
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    // Special keys
    Escape,
    Enter,
    Backspace,
    Tab,
    Space,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Control,
    Alt,
    Shift,
    Super,
    Home,
    End,
    PageUp,
    PageDown,
    Delete,
    Insert,
    // Punctuation
    Minus,
    Equal,
    LeftBracket,
    RightBracket,
    Backslash,
    Semicolon,
    Quote,
    Comma,
    Period,
    Slash,
    // Numpad
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadEnter,
    NumpadAdd,
    NumpadSubtract,
    NumpadMultiply,
    NumpadDivide,
    // Media
    VolumeUp,
    VolumeDown,
    Mute,
    // Unknown
    Unknown,
}

impl KeyCode {
    pub fn from_scan_code(code: u8) -> Self {
        match code {
            0x1E => KeyCode::A,
            0x30 => KeyCode::B,
            0x2E => KeyCode::C,
            0x20 => KeyCode::D,
            0x12 => KeyCode::E,
            0x21 => KeyCode::F,
            0x22 => KeyCode::G,
            0x23 => KeyCode::H,
            0x17 => KeyCode::I,
            0x24 => KeyCode::J,
            0x25 => KeyCode::K,
            0x26 => KeyCode::L,
            0x32 => KeyCode::M,
            0x31 => KeyCode::N,
            0x18 => KeyCode::O,
            0x19 => KeyCode::P,
            0x10 => KeyCode::Q,
            0x13 => KeyCode::R,
            0x1F => KeyCode::S,
            0x14 => KeyCode::T,
            0x16 => KeyCode::U,
            0x2F => KeyCode::V,
            0x11 => KeyCode::W,
            0x2D => KeyCode::X,
            0x15 => KeyCode::Y,
            0x2C => KeyCode::Z,
            0x0B => KeyCode::D0,
            0x02 => KeyCode::D1,
            0x03 => KeyCode::D2,
            0x04 => KeyCode::D3,
            0x05 => KeyCode::D4,
            0x06 => KeyCode::D5,
            0x07 => KeyCode::D6,
            0x08 => KeyCode::D7,
            0x09 => KeyCode::D8,
            0x0A => KeyCode::D9,
            0x3B => KeyCode::F1,
            0x3C => KeyCode::F2,
            0x3D => KeyCode::F3,
            0x3E => KeyCode::F4,
            0x3F => KeyCode::F5,
            0x40 => KeyCode::F6,
            0x41 => KeyCode::F7,
            0x42 => KeyCode::F8,
            0x43 => KeyCode::F9,
            0x44 => KeyCode::F10,
            0x57 => KeyCode::F11,
            0x58 => KeyCode::F12,
            0x01 => KeyCode::Escape,
            0x1C => KeyCode::Enter,
            0x0E => KeyCode::Backspace,
            0x0F => KeyCode::Tab,
            0x39 => KeyCode::Space,
            0xC8 => KeyCode::ArrowUp,
            0xD0 => KeyCode::ArrowDown,
            0xCB => KeyCode::ArrowLeft,
            0xCD => KeyCode::ArrowRight,
            0x1D => KeyCode::Control,
            0x38 => KeyCode::Alt,
            0x2A | 0x36 => KeyCode::Shift,
            0x5B => KeyCode::Super,
            0xC7 => KeyCode::Home,
            0xCF => KeyCode::End,
            0xC9 => KeyCode::PageUp,
            0xD1 => KeyCode::PageDown,
            0xD3 => KeyCode::Delete,
            0xD2 => KeyCode::Insert,
            0x0C => KeyCode::Minus,
            0x0D => KeyCode::Equal,
            0x1A => KeyCode::LeftBracket,
            0x1B => KeyCode::RightBracket,
            0x2B => KeyCode::Backslash,
            0x27 => KeyCode::Semicolon,
            0x28 => KeyCode::Quote,
            0x33 => KeyCode::Comma,
            0x34 => KeyCode::Period,
            0x35 => KeyCode::Slash,
            0x52 => KeyCode::Numpad0,
            0x4F => KeyCode::Numpad1,
            0x50 => KeyCode::Numpad2,
            0x51 => KeyCode::Numpad3,
            0x4B => KeyCode::Numpad4,
            0x4C => KeyCode::Numpad5,
            0x4D => KeyCode::Numpad6,
            0x47 => KeyCode::Numpad7,
            0x48 => KeyCode::Numpad8,
            0x49 => KeyCode::Numpad9,
            0x9C => KeyCode::NumpadEnter,
            0x4E => KeyCode::NumpadAdd,
            0x4A => KeyCode::NumpadSubtract,
            0x37 => KeyCode::NumpadMultiply,
            0xB5 => KeyCode::NumpadDivide,
            0xAE => KeyCode::VolumeUp,
            0xAF => KeyCode::VolumeDown,
            0xA0 => KeyCode::Mute,
            _ => KeyCode::Unknown,
        }
    }

    pub fn to_char(self, shift: bool) -> Option<char> {
        match self {
            KeyCode::A => Some(if shift { 'A' } else { 'a' }),
            KeyCode::B => Some(if shift { 'B' } else { 'b' }),
            KeyCode::C => Some(if shift { 'C' } else { 'c' }),
            KeyCode::D => Some(if shift { 'D' } else { 'd' }),
            KeyCode::E => Some(if shift { 'E' } else { 'e' }),
            KeyCode::F => Some(if shift { 'F' } else { 'f' }),
            KeyCode::G => Some(if shift { 'G' } else { 'g' }),
            KeyCode::H => Some(if shift { 'H' } else { 'h' }),
            KeyCode::I => Some(if shift { 'I' } else { 'i' }),
            KeyCode::J => Some(if shift { 'J' } else { 'j' }),
            KeyCode::K => Some(if shift { 'K' } else { 'k' }),
            KeyCode::L => Some(if shift { 'L' } else { 'l' }),
            KeyCode::M => Some(if shift { 'M' } else { 'm' }),
            KeyCode::N => Some(if shift { 'N' } else { 'n' }),
            KeyCode::O => Some(if shift { 'O' } else { 'o' }),
            KeyCode::P => Some(if shift { 'P' } else { 'p' }),
            KeyCode::Q => Some(if shift { 'Q' } else { 'q' }),
            KeyCode::R => Some(if shift { 'R' } else { 'r' }),
            KeyCode::S => Some(if shift { 'S' } else { 's' }),
            KeyCode::T => Some(if shift { 'T' } else { 't' }),
            KeyCode::U => Some(if shift { 'U' } else { 'u' }),
            KeyCode::V => Some(if shift { 'V' } else { 'v' }),
            KeyCode::W => Some(if shift { 'W' } else { 'w' }),
            KeyCode::X => Some(if shift { 'X' } else { 'x' }),
            KeyCode::Y => Some(if shift { 'Y' } else { 'y' }),
            KeyCode::Z => Some(if shift { 'Z' } else { 'z' }),
            KeyCode::D0 => Some(if shift { ')' } else { '0' }),
            KeyCode::D1 => Some(if shift { '!' } else { '1' }),
            KeyCode::D2 => Some(if shift { '@' } else { '2' }),
            KeyCode::D3 => Some(if shift { '#' } else { '3' }),
            KeyCode::D4 => Some(if shift { '$' } else { '4' }),
            KeyCode::D5 => Some(if shift { '%' } else { '5' }),
            KeyCode::D6 => Some(if shift { '^' } else { '6' }),
            KeyCode::D7 => Some(if shift { '&' } else { '7' }),
            KeyCode::D8 => Some(if shift { '*' } else { '8' }),
            KeyCode::D9 => Some(if shift { '(' } else { '9' }),
            KeyCode::Space => Some(' '),
            KeyCode::Minus => Some(if shift { '_' } else { '-' }),
            KeyCode::Equal => Some(if shift { '+' } else { '=' }),
            KeyCode::Comma => Some(if shift { '<' } else { ',' }),
            KeyCode::Period => Some(if shift { '>' } else { '.' }),
            KeyCode::Slash => Some(if shift { '?' } else { '/' }),
            KeyCode::Semicolon => Some(if shift { ':' } else { ';' }),
            KeyCode::Quote => Some(if shift { '"' } else { '\'' }),
            KeyCode::LeftBracket => Some(if shift { '{' } else { '[' }),
            KeyCode::RightBracket => Some(if shift { '}' } else { ']' }),
            KeyCode::Backslash => Some(if shift { '|' } else { '\\' }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEventType {
    Press,
    Release,
    Repeat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    pub event_type: KeyEventType,
    pub key_code: KeyCode,
    pub scan_code: u8,
    pub modifiers: ModifierKeys,
}

impl KeyEvent {
    pub fn new(event_type: KeyEventType, key_code: KeyCode, scan_code: u8) -> Self {
        KeyEvent {
            event_type,
            key_code,
            scan_code,
            modifiers: ModifierKeys::empty(),
        }
    }

    pub fn with_modifiers(mut self, modifiers: ModifierKeys) -> Self {
        self.modifiers = modifiers;
        self
    }

    pub fn to_char(self) -> Option<char> {
        self.key_code
            .to_char(self.modifiers.contains(ModifierKeys::SHIFT))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModifierKeys {
    bits: u8,
}

impl ModifierKeys {
    pub const SHIFT: Self = ModifierKeys { bits: 1 << 0 };
    pub const CONTROL: Self = ModifierKeys { bits: 1 << 1 };
    pub const ALT: Self = ModifierKeys { bits: 1 << 2 };
    pub const SUPER: Self = ModifierKeys { bits: 1 << 3 };

    pub const fn empty() -> Self {
        ModifierKeys { bits: 0 }
    }

    pub const fn all() -> Self {
        ModifierKeys { bits: 0x0F }
    }

    pub fn contains(self, other: Self) -> bool {
        self.bits & other.bits == other.bits
    }

    pub fn is_empty(self) -> bool {
        self.bits == 0
    }

    pub fn insert(&mut self, other: Self) {
        self.bits |= other.bits;
    }

    pub fn remove(&mut self, other: Self) {
        self.bits &= !other.bits;
    }
}

#[derive(Debug, Clone)]
pub struct KeyboardState {
    pub pressed_keys: Vec<KeyCode>,
    pub modifiers: ModifierKeys,
    pub repeat_delay: u32,
    pub repeat_rate: u32,
}

impl KeyboardState {
    pub fn new() -> Self {
        KeyboardState {
            pressed_keys: Vec::new(),
            modifiers: ModifierKeys::empty(),
            repeat_delay: 250,
            repeat_rate: 30,
        }
    }

    pub fn press_key(&mut self, key: KeyCode) {
        if !self.pressed_keys.contains(&key) {
            self.pressed_keys.push(key);
        }
        match key {
            KeyCode::Shift => self.modifiers.insert(ModifierKeys::SHIFT),
            KeyCode::Control => self.modifiers.insert(ModifierKeys::CONTROL),
            KeyCode::Alt => self.modifiers.insert(ModifierKeys::ALT),
            KeyCode::Super => self.modifiers.insert(ModifierKeys::SUPER),
            _ => {}
        }
    }

    pub fn release_key(&mut self, key: KeyCode) {
        self.pressed_keys.retain(|&k| k != key);
        match key {
            KeyCode::Shift => self.modifiers.remove(ModifierKeys::SHIFT),
            KeyCode::Control => self.modifiers.remove(ModifierKeys::CONTROL),
            KeyCode::Alt => self.modifiers.remove(ModifierKeys::ALT),
            KeyCode::Super => self.modifiers.remove(ModifierKeys::SUPER),
            _ => {}
        }
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.pressed_keys.contains(&key)
    }
}

impl Default for KeyboardState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_code_from_scan_code() {
        assert_eq!(KeyCode::from_scan_code(0x1E), KeyCode::A);
        assert_eq!(KeyCode::from_scan_code(0x02), KeyCode::D1);
        assert_eq!(KeyCode::from_scan_code(0x1C), KeyCode::Enter);
        assert_eq!(KeyCode::from_scan_code(0xFF), KeyCode::Unknown);
    }

    #[test]
    fn key_code_to_char() {
        assert_eq!(KeyCode::A.to_char(false), Some('a'));
        assert_eq!(KeyCode::A.to_char(true), Some('A'));
        assert_eq!(KeyCode::D1.to_char(false), Some('1'));
        assert_eq!(KeyCode::D1.to_char(true), Some('!'));
        assert_eq!(KeyCode::Enter.to_char(false), None);
    }

    #[test]
    fn modifier_keys_operations() {
        let mut mods = ModifierKeys::empty();
        assert!(mods.is_empty());

        mods.insert(ModifierKeys::SHIFT);
        assert!(mods.contains(ModifierKeys::SHIFT));

        mods.remove(ModifierKeys::SHIFT);
        assert!(!mods.contains(ModifierKeys::SHIFT));
    }

    #[test]
    fn keyboard_state_press_release() {
        let mut kb = KeyboardState::new();
        kb.press_key(KeyCode::A);
        assert!(kb.is_key_pressed(KeyCode::A));

        kb.release_key(KeyCode::A);
        assert!(!kb.is_key_pressed(KeyCode::A));
    }

    #[test]
    fn keyboard_state_modifiers() {
        let mut kb = KeyboardState::new();
        kb.press_key(KeyCode::Shift);
        assert!(kb.modifiers.contains(ModifierKeys::SHIFT));

        kb.release_key(KeyCode::Shift);
        assert!(!kb.modifiers.contains(ModifierKeys::SHIFT));
    }

    #[test]
    fn key_event_to_char() {
        let event = KeyEvent::new(KeyEventType::Press, KeyCode::A, 0x1E);
        assert_eq!(event.to_char(), Some('a'));

        let event = event.with_modifiers(ModifierKeys::SHIFT);
        assert_eq!(event.to_char(), Some('A'));
    }
}
