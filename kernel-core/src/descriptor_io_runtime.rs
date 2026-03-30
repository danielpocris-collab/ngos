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
