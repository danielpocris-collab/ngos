//! Canonical subsystem role:
//! - subsystem: descriptor I/O runtime
//! - owner layer: Layer 1
//! - semantic owner: `kernel-core`
//! - truth path role: canonical runtime execution surface for descriptor I/O
//!   and readiness behavior
//!
//! Canonical contract families handled here:
//! - descriptor access contracts
//! - descriptor read/write runtime contracts
//! - readiness runtime contracts
//!
//! This module may execute canonical descriptor I/O behavior, but it must
//! remain subordinate to the descriptor model and kernel runtime truth.

use super::*;

#[path = "descriptor_io_runtime/access.rs"]
mod access;
#[path = "descriptor_io_runtime/ops.rs"]
mod ops;
#[path = "descriptor_io_runtime/readiness.rs"]
mod readiness;

pub(crate) use access::*;
pub(crate) use ops::*;
pub(crate) use readiness::*;
