//! Canonical subsystem role:
//! - subsystem: memory wait runtime
//! - owner layer: Layer 1
//! - semantic owner: `kernel-core`
//! - truth path role: canonical runtime handling for memory wait and wake
//!   behavior
//!
//! Canonical contract families handled here:
//! - memory wait contracts
//! - wait housekeeping contracts
//! - wake and blocking runtime contracts
//!
//! This module may mutate canonical memory-wait runtime state, but it must
//! remain subordinate to kernel runtime and VM truth.

use super::*;

#[path = "memory_wait_runtime/housekeeping.rs"]
mod housekeeping;
#[path = "memory_wait_runtime/ops.rs"]
mod ops;

pub(crate) use housekeeping::*;
pub(crate) use ops::*;
