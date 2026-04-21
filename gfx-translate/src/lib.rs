#![cfg_attr(not(test), no_std)]

//! Canonical subsystem role:
//! - subsystem: graphics translation support
//! - owner layer: support translation layer
//! - semantic owner: `gfx-translate`
//! - truth path role: translation and scripting support for graphics-oriented
//!   runtime/userland flows
//!
//! Canonical contract families defined here:
//! - frame script contracts
//! - draw command translation contracts
//! - render command support contracts
//!
//! This crate may define graphics translation support, but it must not
//! redefine kernel, device-runtime, or product-level graphics truth.

extern crate alloc;

use alloc::{format, string::String};

mod frame_profile_agent;
mod frame_script_agent;
mod gfx_translator;
mod render_command_agent;

pub use frame_profile_agent::FrameProfile;
pub use frame_script_agent::{EncodedFrame, FrameScript};
pub use gfx_translator::{
    ForeignDrawCmd, ForeignFrameScript, GfxTranslateError, GfxTranslator, SourceApi,
};
pub use render_command_agent::{
    BlendMode, DrawOp, DrawOpClass, FontFamily, RenderPassClass, RgbaColor,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameScriptError {
    MissingField(&'static str),
    InvalidLine(String),
    InvalidValue { key: String, value: String },
}

impl FrameScriptError {
    pub fn describe(&self) -> String {
        match self {
            Self::MissingField(field) => format!("missing field {field}"),
            Self::InvalidLine(line) => format!("invalid line {line}"),
            Self::InvalidValue { key, value } => format!("invalid value {key}={value}"),
        }
    }
}
