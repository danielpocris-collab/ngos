#![allow(dead_code)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::let_and_return)]
#![allow(clippy::if_same_then_else)]

mod serial {
    pub fn write_bytes(_bytes: &[u8]) {}
}

#[path = "../src/keyboard.rs"]
mod keyboard;

use std::sync::{Mutex, OnceLock};

fn keyboard_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[test]
fn canonical_input_handles_editing_modifiers_and_special_sequences() {
    let _guard = keyboard_test_lock().lock().unwrap();
    keyboard::reset_state();
    keyboard::inject_scancodes(&[
        0x1e, 0x30, 0x0e, 0x2e, 0x3a, 0x20, 0x2a, 0x21, 0xaa, 0x1c, 0xe0, 0x48, 0xe0, 0x4b,
    ]);

    let mut bytes = Vec::new();
    while let Some(byte) = keyboard::read_byte_nonblocking() {
        bytes.push(byte);
    }

    assert_eq!(bytes, b"acDf\n\x1b[A\x1b[D");
}

#[test]
fn keyboard_event_stream_tracks_press_and_release_state() {
    let _guard = keyboard_test_lock().lock().unwrap();
    keyboard::reset_state();
    keyboard::inject_scancodes(&[0x2a, 0x1d, 0xe0, 0x38, 0xb8, 0x9d, 0xaa]);

    let mut events = Vec::new();
    while let Some(event) = keyboard::read_event_nonblocking() {
        events.push(event);
    }

    assert_eq!(events.len(), 6);
    assert_eq!(events[0].code, keyboard::KeyCode::LeftShift);
    assert_eq!(events[0].action, keyboard::KeyAction::Press);
    assert_eq!(events[1].code, keyboard::KeyCode::LeftControl);
    assert_eq!(events[2].code, keyboard::KeyCode::RightAlt);
    assert_eq!(events[3].action, keyboard::KeyAction::Release);
    assert_eq!(events[4].action, keyboard::KeyAction::Release);
    assert_eq!(events[5].action, keyboard::KeyAction::Release);
}
