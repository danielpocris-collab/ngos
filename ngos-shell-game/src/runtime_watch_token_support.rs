use ngos_game_compat_runtime::CompatLaneKind;

use crate::{GameCompatLaneRuntime, GameCompatSession};

pub fn game_watch_token(session: &GameCompatSession, lane: &GameCompatLaneRuntime) -> u64 {
    ((session.pid & 0xffff_ffff) << 32) | (lane.resource_id as u64 & 0xffff_ffff)
}

pub fn parse_game_lane_kind(value: &str) -> Option<CompatLaneKind> {
    match value {
        "graphics" => Some(CompatLaneKind::Graphics),
        "audio" => Some(CompatLaneKind::Audio),
        "input" => Some(CompatLaneKind::Input),
        _ => None,
    }
}
