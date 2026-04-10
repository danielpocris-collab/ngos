use crate::keyboard_agent::KeyEventType;
use crate::mouse_agent::MouseEventType;
use crate::{InputEvent, KeyEvent, KeyboardState, MouseEvent, MouseState};
use alloc::vec::Vec;

pub trait InputTarget {
    fn id(&self) -> u32;
    fn contains_point(&self, x: i32, y: i32) -> bool;
    fn on_input_event(&mut self, event: &InputEvent);
}

pub struct InputDispatcher {
    mouse_state: MouseState,
    keyboard_state: KeyboardState,
    targets: Vec<u32>,
    focused_target: Option<u32>,
    hover_target: Option<u32>,
    timestamp: u64,
}

impl InputDispatcher {
    pub fn new() -> Self {
        InputDispatcher {
            mouse_state: MouseState::new(),
            keyboard_state: KeyboardState::new(),
            targets: Vec::new(),
            focused_target: None,
            hover_target: None,
            timestamp: 0,
        }
    }

    pub fn register_target(&mut self, id: u32) {
        if !self.targets.contains(&id) {
            self.targets.push(id);
        }
    }

    pub fn unregister_target(&mut self, id: u32) {
        self.targets.retain(|&tid| tid != id);
        if self.focused_target == Some(id) {
            self.focused_target = None;
        }
        if self.hover_target == Some(id) {
            self.hover_target = None;
        }
    }

    pub fn set_focused(&mut self, id: Option<u32>) {
        self.focused_target = id;
    }

    pub fn focused_target(&self) -> Option<u32> {
        self.focused_target
    }

    pub fn mouse_state(&self) -> &MouseState {
        &self.mouse_state
    }

    pub fn keyboard_state(&self) -> &KeyboardState {
        &self.keyboard_state
    }

    pub fn dispatch_mouse<T: InputTarget>(
        &mut self,
        event: MouseEvent,
        targets: &mut [T],
    ) -> Option<u32> {
        self.timestamp += 1;

        match event.event_type {
            MouseEventType::Move => {
                self.mouse_state.update_position(event.x, event.y);

                let old_hover = self.hover_target;
                self.hover_target = self.find_target_at(event.x, event.y, targets);

                if old_hover != self.hover_target {
                    if let Some(old_id) = old_hover {
                        let leave_event = InputEvent::mouse(
                            MouseEvent::new(MouseEventType::Leave, event.x, event.y),
                            old_id,
                            self.timestamp,
                        );
                        if let Some(target) = targets.iter_mut().find(|t| t.id() == old_id) {
                            target.on_input_event(&leave_event);
                        }
                    }

                    if let Some(new_id) = self.hover_target {
                        let enter_event = InputEvent::mouse(
                            MouseEvent::new(MouseEventType::Enter, event.x, event.y),
                            new_id,
                            self.timestamp,
                        );
                        if let Some(target) = targets.iter_mut().find(|t| t.id() == new_id) {
                            target.on_input_event(&enter_event);
                        }
                    }
                }
            }

            MouseEventType::ButtonPress(button) => {
                self.mouse_state.press_button(button);
            }

            MouseEventType::ButtonRelease(button) => {
                self.mouse_state.release_button(button);
            }

            _ => {}
        }

        if let Some(target_id) = self.hover_target.or(self.focused_target) {
            let input_event = InputEvent::mouse(event, target_id, self.timestamp);
            if let Some(target) = targets.iter_mut().find(|t| t.id() == target_id) {
                target.on_input_event(&input_event);
            }
            return Some(target_id);
        }

        None
    }

    pub fn dispatch_key<T: InputTarget>(
        &mut self,
        event: KeyEvent,
        targets: &mut [T],
    ) -> Option<u32> {
        self.timestamp += 1;

        match event.event_type {
            KeyEventType::Press => {
                self.keyboard_state.press_key(event.key_code);
            }
            KeyEventType::Release => {
                self.keyboard_state.release_key(event.key_code);
            }
            _ => {}
        }

        if let Some(target_id) = self.focused_target {
            let input_event = InputEvent::key(event, target_id, self.timestamp);
            if let Some(target) = targets.iter_mut().find(|t| t.id() == target_id) {
                target.on_input_event(&input_event);
            }
            return Some(target_id);
        }

        None
    }

    fn find_target_at<T: InputTarget>(&self, x: i32, y: i32, targets: &[T]) -> Option<u32> {
        for target in targets.iter().rev() {
            if target.contains_point(x, y) {
                return Some(target.id());
            }
        }
        None
    }

    pub fn tick(&mut self) {
        self.timestamp += 1;
    }
}

impl Default for InputDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyboard_agent::{KeyCode, KeyEventType};
    use crate::mouse_agent::{MouseButton, MouseEventType};

    struct TestTarget {
        id: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        received_events: Vec<InputEvent>,
    }

    impl TestTarget {
        fn new(id: u32, x: i32, y: i32, w: i32, h: i32) -> Self {
            TestTarget {
                id,
                x,
                y,
                w,
                h,
                received_events: Vec::new(),
            }
        }
    }

    impl InputTarget for TestTarget {
        fn id(&self) -> u32 {
            self.id
        }

        fn contains_point(&self, x: i32, y: i32) -> bool {
            x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
        }

        fn on_input_event(&mut self, event: &InputEvent) {
            self.received_events.push(event.clone());
        }
    }

    #[test]
    fn dispatcher_registers_targets() {
        let mut dispatcher = InputDispatcher::new();
        dispatcher.register_target(1);
        dispatcher.register_target(2);

        assert!(dispatcher.targets.contains(&1));
        assert!(dispatcher.targets.contains(&2));
    }

    #[test]
    fn dispatcher_finds_target_at_point() {
        let mut dispatcher = InputDispatcher::new();
        let mut targets = [
            TestTarget::new(1, 0, 0, 100, 100),
            TestTarget::new(2, 50, 50, 100, 100),
        ];

        let event = MouseEvent::new(MouseEventType::Move, 75, 75);
        dispatcher.dispatch_mouse(event, &mut targets);

        assert_eq!(dispatcher.hover_target, Some(2));
    }

    #[test]
    fn dispatcher_tracks_mouse_position() {
        let mut dispatcher = InputDispatcher::new();
        let mut targets: [TestTarget; 0] = [];

        let event = MouseEvent::new(MouseEventType::Move, 100, 200);
        dispatcher.dispatch_mouse(event, &mut targets);

        assert_eq!(dispatcher.mouse_state().x, 100);
        assert_eq!(dispatcher.mouse_state().y, 200);
    }

    #[test]
    fn dispatcher_tracks_keyboard_state() {
        let mut dispatcher = InputDispatcher::new();
        let mut targets: [TestTarget; 0] = [];

        let event = KeyEvent::new(KeyEventType::Press, KeyCode::A, 0x1E);
        dispatcher.dispatch_key(event, &mut targets);

        assert!(dispatcher.keyboard_state().is_key_pressed(KeyCode::A));
    }

    #[test]
    fn dispatcher_sets_focus() {
        let mut dispatcher = InputDispatcher::new();
        dispatcher.set_focused(Some(42));

        assert_eq!(dispatcher.focused_target(), Some(42));

        dispatcher.set_focused(None);
        assert_eq!(dispatcher.focused_target(), None);
    }
}
