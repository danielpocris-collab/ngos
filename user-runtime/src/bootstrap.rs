//! Canonical subsystem role:
//! - subsystem: user-runtime bootstrap surface
//! - owner layer: Layer 2
//! - semantic owner: `user-runtime`
//! - truth path role: direct re-export of canonical bootstrap ABI into the
//!   native user runtime
//!
//! Canonical contract families handled here:
//! - bootstrap transport contracts
//! - first-user bootstrap runtime contracts
//!
//! This module may expose the canonical bootstrap ABI to user-runtime callers,
//! but it must not reinterpret or redefine the underlying bootstrap truth.

pub use ngos_user_abi::bootstrap::*;
