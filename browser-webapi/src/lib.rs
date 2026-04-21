//! Canonical subsystem role:
//! - subsystem: browser Web API support
//! - owner layer: application support layer
//! - semantic owner: `browser-webapi`
//! - truth path role: browser-facing Web API support for browser application
//!   flows
//!
//! Canonical contract families defined here:
//! - Web API support contracts
//! - browser host API bridging contracts
//!
//! This crate may define browser Web API support behavior, but it must not
//! redefine kernel, runtime, or product-level OS truth.

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
