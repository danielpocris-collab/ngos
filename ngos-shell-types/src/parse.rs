//! Parsing utilities for shell argument tokens.

/// Parse an optional string token as a `u64`.
pub fn parse_u64_arg(token: Option<&str>) -> Option<u64> {
    token.and_then(|value| value.parse::<u64>().ok())
}

/// Parse an optional string token as a `usize`.
pub fn parse_usize_arg(token: Option<&str>) -> Option<usize> {
    token.and_then(|value| value.parse::<usize>().ok())
}

/// Parse an optional string token as a `u16`.
pub fn parse_u16_arg(token: Option<&str>) -> Option<u16> {
    token.and_then(|value| value.parse::<u16>().ok())
}

/// Parse an optional string token as an `i64`.
pub fn parse_i64_arg(token: Option<&str>) -> Option<i64> {
    token.and_then(|value| value.parse::<i64>().ok())
}
