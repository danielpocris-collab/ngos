#![cfg_attr(not(test), no_std)]

//! Canonical subsystem role:
//! - subsystem: input translation support
//! - owner layer: support translation layer
//! - semantic owner: `input-translate`
//! - truth path role: translation and scripting support for input-oriented
//!   runtime/userland flows
//!
//! Canonical contract families defined here:
//! - input event translation contracts
//! - keyboard/mouse support contracts
//! - foreign-input translation contracts
//!
//! This crate may define input translation support, but it must not redefine
//! kernel, device-runtime, or product-level input truth.

extern crate alloc;

mod input_dispatcher;
mod input_event_agent;
mod input_script_agent;
pub mod keyboard_agent;
pub mod mouse_agent;

pub use input_dispatcher::{InputDispatcher, InputTarget};
pub use input_event_agent::{InputEvent, InputEventType};
pub use input_script_agent::{
    EncodedInput, ForeignInputApi, InputScript, InputScriptError, InputTranslateError,
    InputTranslator,
};
pub use keyboard_agent::{KeyCode, KeyEvent, KeyEventType, KeyboardState, ModifierKeys};
pub use mouse_agent::{MouseButton, MouseEvent, MouseEventType, MouseState};
