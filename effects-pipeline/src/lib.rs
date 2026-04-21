#![cfg_attr(not(test), no_std)]

//! Canonical subsystem role:
//! - subsystem: visual effects support pipeline
//! - owner layer: presentation support layer
//! - semantic owner: `effects-pipeline`
//! - truth path role: visual effect composition support for user-facing
//!   rendering flows
//!
//! Canonical contract families defined here:
//! - effect pipeline contracts
//! - backdrop/gradient/shadow support contracts
//! - temporal and translucency support contracts
//!
//! This crate may define visual effect support behavior, but it must not
//! redefine kernel, runtime, or subsystem truth.

extern crate alloc;

use alloc::{format, string::String};
use ngos_gfx_translate::RgbaColor;

mod backdrop_agent;
mod effect_pipeline_agent;
mod gradient_agent;
mod shadow_agent;
mod temporal_agent;
mod translucency_agent;

pub use backdrop_agent::BackdropSpec;
pub use effect_pipeline_agent::{Effect, EffectPipeline, PipelineInspect};
pub use gradient_agent::{GradientDirection, GradientSpec, GradientStop};
pub use shadow_agent::ShadowSpec;
pub use temporal_agent::{AccentPulse, TemporalSpec, TemporalState};
pub use translucency_agent::TranslucencySpec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl EffectRect {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        EffectRect {
            x,
            y,
            width,
            height,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectError {
    ZeroDimensions,
    InsufficientGradientStops { count: usize },
    BackdropRequiresBlur,
    InvalidTemporalStride,
    EmptyPipeline,
}

impl EffectError {
    pub fn describe(&self) -> String {
        match self {
            Self::ZeroDimensions => String::from("effect rect has zero dimensions"),
            Self::InsufficientGradientStops { count } => {
                format!("gradient requires at least 2 stops, got {}", count)
            }
            Self::BackdropRequiresBlur => String::from("backdrop effect requires blur_radius > 0"),
            Self::InvalidTemporalStride => String::from("temporal stride must be between 1 and 16"),
            Self::EmptyPipeline => String::from("empty pipeline: no effects to compile"),
        }
    }
}

pub(crate) fn lerp_color(a: RgbaColor, b: RgbaColor, t: u8) -> RgbaColor {
    RgbaColor {
        r: lerp_u8(a.r, b.r, t),
        g: lerp_u8(a.g, b.g, t),
        b: lerp_u8(a.b, b.b, t),
        a: lerp_u8(a.a, b.a, t),
    }
}

fn lerp_u8(a: u8, b: u8, t: u8) -> u8 {
    let a = a as u32;
    let b = b as u32;
    let t = t as u32;
    ((a * (255 - t) + b * t) / 255) as u8
}
