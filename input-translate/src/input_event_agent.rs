use crate::{KeyEvent, MouseEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEventType {
    Mouse(MouseEvent),
    Key(KeyEvent),
    Focus,
    Blur,
    Custom(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct InputEvent {
    pub event_type: InputEventType,
    pub target_id: u32,
    pub timestamp: u64,
}

impl InputEvent {
    pub fn mouse(event: MouseEvent, target_id: u32, timestamp: u64) -> Self {
        InputEvent {
            event_type: InputEventType::Mouse(event),
            target_id,
            timestamp,
        }
    }

    pub fn key(event: KeyEvent, target_id: u32, timestamp: u64) -> Self {
        InputEvent {
            event_type: InputEventType::Key(event),
            target_id,
            timestamp,
        }
    }

    pub fn focus(target_id: u32, timestamp: u64) -> Self {
        InputEvent {
            event_type: InputEventType::Focus,
            target_id,
            timestamp,
        }
    }

    pub fn blur(target_id: u32, timestamp: u64) -> Self {
        InputEvent {
            event_type: InputEventType::Blur,
            target_id,
            timestamp,
        }
    }

    pub fn custom(code: u32, target_id: u32, timestamp: u64, _data: &str) -> Self {
        InputEvent {
            event_type: InputEventType::Custom(code),
            target_id,
            timestamp,
        }
    }

    pub fn is_mouse(&self) -> bool {
        matches!(self.event_type, InputEventType::Mouse(_))
    }

    pub fn is_key(&self) -> bool {
        matches!(self.event_type, InputEventType::Key(_))
    }

    pub fn as_mouse(&self) -> Option<&MouseEvent> {
        match &self.event_type {
            InputEventType::Mouse(e) => Some(e),
            _ => None,
        }
    }

    pub fn as_key(&self) -> Option<&KeyEvent> {
        match &self.event_type {
            InputEventType::Key(e) => Some(e),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyboard_agent::{KeyCode, KeyEventType};
    use crate::mouse_agent::MouseEventType;

    #[test]
    fn input_event_mouse_creation() {
        let mouse_event = MouseEvent::new(MouseEventType::Move, 100, 200);
        let event = InputEvent::mouse(mouse_event, 1, 1000);

        assert!(event.is_mouse());
        assert!(!event.is_key());
        assert_eq!(event.target_id, 1);
        assert_eq!(event.timestamp, 1000);
    }

    #[test]
    fn input_event_key_creation() {
        let key_event = KeyEvent::new(KeyEventType::Press, KeyCode::A, 0x1E);
        let event = InputEvent::key(key_event, 2, 2000);

        assert!(!event.is_mouse());
        assert!(event.is_key());
        assert_eq!(event.target_id, 2);
    }

    #[test]
    fn input_event_as_mouse() {
        let mouse_event = MouseEvent::new(MouseEventType::Move, 50, 60);
        let event = InputEvent::mouse(mouse_event, 1, 1000);

        let mouse = event.as_mouse().unwrap();
        assert_eq!(mouse.x, 50);
        assert_eq!(mouse.y, 60);
    }

    #[test]
    fn input_event_as_key() {
        let key_event = KeyEvent::new(KeyEventType::Press, KeyCode::Enter, 0x1C);
        let event = InputEvent::key(key_event, 1, 1000);

        let key = event.as_key().unwrap();
        assert_eq!(key.key_code, KeyCode::Enter);
    }
}
