use ngos_game_compat_runtime::{GameCompatManifest, GameSessionPlan};

use crate::{GameCompatLaneRuntime, GameCompatSession, default_session_device_paths};

pub fn build_game_session_media(
    session: &mut GameCompatSession,
    manifest: &GameCompatManifest,
    _plan: &GameSessionPlan,
    lanes: alloc::vec::Vec<GameCompatLaneRuntime>,
) {
    let device_paths = default_session_device_paths();
    session.graphics_device_path = device_paths.graphics_device_path;
    session.graphics_driver_path = device_paths.graphics_driver_path;
    session.graphics_source_api = manifest.graphics.source_api;
    session.graphics_translation = manifest.graphics_translation_plan();
    session.graphics_profile = manifest.graphics.profile.clone();
    session.audio_device_path = device_paths.audio_device_path;
    session.audio_driver_path = device_paths.audio_driver_path;
    session.audio_profile = manifest.audio.profile.clone();
    session.input_device_path = device_paths.input_device_path;
    session.input_driver_path = device_paths.input_driver_path;
    session.input_profile = manifest.input.profile.clone();
    session.lanes = lanes;
}
