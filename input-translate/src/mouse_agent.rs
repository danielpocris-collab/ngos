#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEventType {
    Move,
    ButtonPress(MouseButton),
    ButtonRelease(MouseButton),
    WheelUp,
    WheelDown,
    Enter,
    Leave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseEvent {
    pub event_type: MouseEventType,
    pub x: i32,
    pub y: i32,
    pub modifiers: u8,
}

impl MouseEvent {
    pub fn new(event_type: MouseEventType, x: i32, y: i32) -> Self {
        MouseEvent {
            event_type,
            x,
            y,
            modifiers: 0,
        }
    }

    pub fn with_modifiers(mut self, modifiers: u8) -> Self {
        self.modifiers = modifiers;
        self
    }
}

#[derive(Debug, Clone)]
pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub buttons_pressed: u8,
    pub cursor_visible: bool,
}

impl MouseState {
    pub fn new() -> Self {
        MouseState {
            x: 0,
            y: 0,
            buttons_pressed: 0,
            cursor_visible: true,
        }
    }

    pub fn is_button_pressed(&self, button: MouseButton) -> bool {
        let mask = match button {
            MouseButton::Left => 1 << 0,
            MouseButton::Right => 1 << 1,
            MouseButton::Middle => 1 << 2,
            MouseButton::Back => 1 << 3,
            MouseButton::Forward => 1 << 4,
        };
        self.buttons_pressed & mask != 0
    }

    pub fn press_button(&mut self, button: MouseButton) {
        let mask = match button {
            MouseButton::Left => 1 << 0,
            MouseButton::Right => 1 << 1,
            MouseButton::Middle => 1 << 2,
            MouseButton::Back => 1 << 3,
            MouseButton::Forward => 1 << 4,
        };
        self.buttons_pressed |= mask;
    }

    pub fn release_button(&mut self, button: MouseButton) {
        let mask = match button {
            MouseButton::Left => 1 << 0,
            MouseButton::Right => 1 << 1,
            MouseButton::Middle => 1 << 2,
            MouseButton::Back => 1 << 3,
            MouseButton::Forward => 1 << 4,
        };
        self.buttons_pressed &= !mask;
    }

    pub fn update_position(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }
}

impl Default for MouseState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_state_button_press_release() {
        let mut state = MouseState::new();
        assert!(!state.is_button_pressed(MouseButton::Left));

        state.press_button(MouseButton::Left);
        assert!(state.is_button_pressed(MouseButton::Left));

        state.release_button(MouseButton::Left);
        assert!(!state.is_button_pressed(MouseButton::Left));
    }

    #[test]
    fn mouse_event_creation() {
        let event = MouseEvent::new(MouseEventType::Move, 100, 200);
        assert_eq!(event.x, 100);
        assert_eq!(event.y, 200);
        assert_eq!(event.modifiers, 0);

        let event = event.with_modifiers(0x02);
        assert_eq!(event.modifiers, 0x02);
    }

    #[test]
    fn mouse_event_types() {
        let _ = MouseEvent::new(MouseEventType::ButtonPress(MouseButton::Right), 50, 50);
        let _ = MouseEvent::new(MouseEventType::WheelUp, 0, 0);
        let _ = MouseEvent::new(MouseEventType::Enter, 10, 10);
    }
}
