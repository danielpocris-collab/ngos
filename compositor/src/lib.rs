#![cfg_attr(not(test), no_std)]

//! Canonical subsystem role:
//! - subsystem: compositor support
//! - owner layer: presentation support layer
//! - semantic owner: `compositor`
//! - truth path role: surface composition support for user-facing `ngos`
//!   presentation flows
//!
//! Canonical contract families defined here:
//! - compositor contracts
//! - surface and stack support contracts
//! - chrome/composition support contracts
//!
//! This crate may define composition support behavior, but it must not
//! redefine kernel, runtime, or subsystem truth.

extern crate alloc;

mod chrome_agent;
mod composition_agent;
mod compositor_agent;
mod surface_agent;
mod surface_stack_agent;

pub use chrome_agent::{ChromeStyle, chrome_ops_for_window};
pub use composition_agent::compose_surface;
pub use compositor_agent::{Compositor, CompositorError, CompositorInspect};
pub use surface_agent::{Surface, SurfaceError, SurfaceId, SurfaceRect, SurfaceRole};
pub use surface_stack_agent::{StackInspect, SurfaceStack};
