//! Canonical subsystem role:
//! - subsystem: native game and compat-session control surface
//! - owner layer: Layer 3
//! - semantic owner: `userland-native`
//! - truth path role: operator-facing orchestration of game compat flows over
//!   canonical runtime and graphics/audio/input contracts
//!
//! Canonical contract families handled here:
//! - game session command contracts
//! - compat launch and quality-report contracts
//! - game runtime orchestration contracts
//!
//! This crate may orchestrate game and compat sessions, but it must not
//! redefine kernel truth, compat truth, or device/runtime ownership from lower
//! layers.

#![no_std]
extern crate alloc;

mod audio_command_support;
mod audio_plan_command_support;
mod audio_queue_command_support;
mod audio_status_command_support;
mod audio_status_render_support;
mod audio_submit_command_support;
mod audio_translation_args_support;
mod audio_translation_command_support;
mod audio_translation_execute_support;
mod command_audio_dispatch;
mod command_dispatch_support;
mod command_graphics_dispatch;
mod command_graphics_observe_dispatch;
mod command_graphics_submit_dispatch;
mod command_input_dispatch;
mod command_media_dispatch;
mod command_session_dispatch;
mod command_session_lifecycle_dispatch;
mod command_session_manifest_dispatch;
mod command_session_simulation_dispatch;
mod command_session_status_dispatch;
mod command_watch_dispatch;
mod compat_abi_observation_support;
mod compat_abi_observe_launch_support;
mod compat_abi_observe_verify_support;
mod compat_abi_payload_support;
mod compat_abi_process_resolve_support;
mod compat_abi_procfs_snapshot_support;
mod compat_abi_procfs_support;
mod compat_abi_session_observe_support;
mod compat_abi_support;
mod compat_loader_artifact_support;
mod compat_loader_launch_support;
mod compat_loader_observe_support;
mod compat_loader_support;
mod compat_smoke_support;
mod compat_support;
mod event_queue_bus_describe_support;
mod event_queue_describe_support;
mod event_queue_graphics_describe_support;
mod event_queue_network_describe_support;
mod event_queue_poll_support;
mod event_queue_resource_describe_support;
mod event_queue_support;
mod event_queue_wait_support;
mod gfx_command_support;
mod gfx_driver_read_command_support;
mod gfx_driver_read_render_support;
mod gfx_driver_request_complete_support;
mod gfx_driver_request_drain_support;
mod gfx_driver_request_read_support;
mod gfx_driver_runtime_support;
mod gfx_metadata_support;
mod gfx_observe_command_support;
mod gfx_plan_command_support;
mod gfx_present_runtime_support;
mod gfx_queue_command_support;
mod gfx_request_command_support;
mod gfx_request_common_support;
mod gfx_request_inspect_command_support;
mod gfx_request_inspect_render_support;
mod gfx_request_parse_support;
mod gfx_request_render_support;
mod gfx_request_retained_support;
mod gfx_runtime_support;
mod gfx_status_command_support;
mod gfx_status_device_render_support;
mod gfx_status_driver_render_support;
mod gfx_status_session_render_support;
mod gfx_submit_command_support;
mod gfx_submit_runtime_command_support;
mod gfx_translation_command_support;
mod input_command_support;
mod input_plan_command_support;
mod input_queue_command_support;
mod input_status_command_support;
mod input_status_render_support;
mod input_submit_command_support;
mod input_translation_args_support;
mod input_translation_command_support;
mod input_translation_execute_support;
mod line_output_support;
mod media_command_support;
mod runtime_audio_completion_support;
mod runtime_audio_state_support;
mod runtime_audio_support;
mod runtime_graphics_completion_support;
mod runtime_graphics_state_support;
mod runtime_graphics_support;
mod runtime_input_completion_support;
mod runtime_input_state_support;
mod runtime_input_support;
mod runtime_lane_support;
mod runtime_media_support;
mod runtime_payload_next_support;
mod runtime_payload_publish_support;
mod runtime_payload_support;
mod runtime_watch_graphics_support;
mod runtime_watch_lifecycle_support;
mod runtime_watch_poll_support;
mod runtime_watch_resource_support;
mod runtime_watch_support;
mod runtime_watch_token_support;
mod script_loader_support;
mod script_translate_support;
mod script_translation_encode_support;
mod script_translation_runtime_support;
mod session_abi_status_command_support;
mod session_artifact_cleanup_support;
mod session_bootstrap_support;
mod session_command_support;
mod session_detail_render_support;
mod session_identity_render_support;
mod session_lane_model_support;
mod session_launch_builder_support;
mod session_launch_command_support;
mod session_launch_device_support;
mod session_launch_identity_assign_support;
mod session_launch_identity_builder_support;
mod session_launch_identity_model_support;
mod session_launch_lane_build_support;
mod session_launch_lane_claim_support;
mod session_launch_lane_record_support;
mod session_launch_lane_support;
mod session_launch_loader_support;
mod session_launch_media_builder_support;
mod session_launch_rollback_support;
mod session_launch_runtime_builder_support;
mod session_launch_runtime_support;
mod session_launch_simulation_metrics_support;
mod session_launch_simulation_script_support;
mod session_launch_simulation_session_support;
mod session_launch_simulation_support;
mod session_launch_support;
mod session_lifecycle_command_support;
mod session_lifecycle_support;
mod session_loader_status_support;
mod session_lookup_support;
mod session_manifest_bootstrap_support;
mod session_manifest_command_support;
mod session_manifest_io_support;
mod session_manifest_render_support;
mod session_manifest_summary_render_support;
mod session_model_support;
mod session_pid_parse_support;
mod session_plan_render_support;
mod session_profile_command_support;
mod session_profile_loader_render_support;
mod session_profile_render_support;
mod session_profile_runtime_render_support;
mod session_profile_state_render_support;
mod session_query_support;
mod session_relaunch_command_support;
mod session_render_abi_line_support;
mod session_render_audio_support;
mod session_render_gfx_support;
mod session_render_identity_line_support;
mod session_render_identity_support;
mod session_render_input_support;
mod session_render_lane_support;
mod session_render_loader_line_support;
mod session_render_media_support;
mod session_render_runtime_support;
mod session_render_shim_line_support;
mod session_render_support;
mod session_resource_policy_support;
mod session_runtime_artifact_support;
mod session_runtime_bootstrap_abi_text_support;
mod session_runtime_bootstrap_env_text_support;
mod session_runtime_bootstrap_loader_text_support;
mod session_runtime_bootstrap_support;
mod session_runtime_bootstrap_text_support;
mod session_runtime_bootstrap_write_support;
mod session_runtime_dir_support;
mod session_runtime_path_support;
mod session_shutdown_support;
mod session_simulation_path_support;
mod session_state_model_support;
mod session_status_command_support;
mod session_stop_command_support;
mod session_summary_render_support;
mod session_support;
mod shell_support;
mod simulation_next_support;
mod simulation_report_support;
mod simulation_run_support;
mod simulation_support;
mod watch_command_args_support;
mod watch_command_lifecycle_support;
mod watch_command_status_support;
mod watch_command_support;

use ngos_game_compat_runtime::graphics_api_name;

pub(crate) use audio_status_render_support::render_game_audio_status;
pub(crate) use audio_translation_args_support::{
    AudioTranslationArgs, parse_audio_translation_args,
};
pub(crate) use audio_translation_execute_support::execute_audio_translation;
pub(crate) use command_audio_dispatch::try_handle_game_audio_command;
pub use command_dispatch_support::try_handle_game_agent_command;
pub(crate) use command_graphics_dispatch::try_handle_game_graphics_command;
pub(crate) use command_graphics_observe_dispatch::try_handle_game_graphics_observe_command;
pub(crate) use command_graphics_submit_dispatch::try_handle_game_graphics_submit_command;
pub(crate) use command_input_dispatch::try_handle_game_input_command;
pub(crate) use command_media_dispatch::try_handle_game_media_command;
pub(crate) use command_session_dispatch::try_handle_game_session_command;
pub(crate) use command_session_lifecycle_dispatch::try_handle_game_session_lifecycle_command;
pub(crate) use command_session_manifest_dispatch::try_handle_game_session_manifest_command;
pub(crate) use command_session_simulation_dispatch::try_handle_game_session_simulation_command;
pub(crate) use command_session_status_dispatch::try_handle_game_session_status_command;
pub(crate) use command_watch_dispatch::try_handle_game_watch_command;
pub use compat_abi_support::{
    GameCompatAbiSessionObservation, GameCompatAbiSessionObservationError,
    GameCompatLaunchAbiObservationError, GameCompatLaunchedAbiObservation, game_compat_abi_payload,
    game_compat_observe_abi_session, game_compat_proc_probe_snapshot,
    game_compat_resolve_process_pid, game_launch_and_observe_abi_session,
};
pub use compat_loader_support::{
    GameCompatLaunchLoaderObservationError, GameCompatLaunchedLoaderObservation,
    GameCompatLoaderSessionObservation, GameCompatLoaderSessionObservationError,
    game_compat_loader_artifact_snapshot, game_compat_loader_session_snapshot,
    game_compat_observe_loader_session, game_launch_and_observe_loader_session,
};
pub use compat_smoke_support::{
    run_native_compat_abi_boot_smoke, run_native_compat_foreign_boot_smoke,
    run_native_compat_foreign_loader_boot_smoke, run_native_compat_loader_boot_smoke,
};
pub use compat_support::{
    game_cleanup_generated_paths, game_cleanup_sessions_and_paths, game_stop_sessions,
};
pub use gfx_command_support::{
    handle_game_gfx_driver_read, handle_game_gfx_next, handle_game_gfx_plan,
    handle_game_gfx_request, handle_game_gfx_status, handle_game_gfx_submit,
};
pub(crate) use gfx_driver_request_complete_support::complete_graphics_driver_request;
pub(crate) use gfx_driver_request_read_support::read_graphics_driver_request_record;
pub use gfx_runtime_support::{
    drain_graphics_driver_requests, gpu_request_kind_name, gpu_request_state_name,
    parse_gfx_payload_translation_metadata, shell_gpu_present_encoded, shell_gpu_submit,
    summarize_graphics_deep_ops,
};
pub(crate) use gfx_status_device_render_support::render_game_gfx_device_status;
pub(crate) use gfx_status_driver_render_support::render_game_gfx_driver_status;
pub(crate) use gfx_status_session_render_support::render_game_gfx_session_status;
pub(crate) use input_status_render_support::render_game_input_status;
pub(crate) use input_translation_args_support::{
    InputTranslationArgs, parse_input_translation_args,
};
pub(crate) use input_translation_execute_support::execute_input_translation;
pub use media_command_support::{
    handle_game_audio_next, handle_game_audio_plan, handle_game_audio_status,
    handle_game_audio_submit, handle_game_input_next, handle_game_input_plan,
    handle_game_input_status, handle_game_input_submit,
};
pub(crate) use runtime_audio_completion_support::game_submit_mix_completion;
pub(crate) use runtime_audio_state_support::game_record_submitted_mix;
pub(crate) use runtime_graphics_completion_support::game_submit_frame_completion;
pub(crate) use runtime_graphics_state_support::game_record_submitted_frame;
pub use runtime_graphics_support::game_submit_frame;
pub(crate) use runtime_input_completion_support::game_submit_input_completion;
pub(crate) use runtime_input_state_support::game_record_submitted_input;
pub use runtime_lane_support::{game_session_lane, game_session_lane_mut};
pub use runtime_media_support::{game_submit_input, game_submit_mix};
pub use runtime_payload_support::{game_next_payload, game_publish_runtime_payload};
pub use runtime_watch_support::{
    game_poll_all_watches, game_start_watch, game_stop_watch, game_wait_watch, game_watch_token,
    parse_game_lane_kind,
};
pub use script_translate_support::{
    game_encode_frame, game_encode_input, game_encode_mix, game_load_frame_script,
    game_load_input_script, game_load_mix_script, handle_game_audio_translate,
    handle_game_gfx_translate, handle_game_input_translate,
};
pub use session_bootstrap_support::{
    game_apply_resource_policy, game_ensure_dir_tree, game_manifest_load, game_plan_contract_kind,
    game_plan_resource_kind, game_render_manifest, game_render_plan, game_write_runtime_bootstrap,
};
pub use session_command_support::{
    handle_game_abi_status, handle_game_launch, handle_game_loader_status, handle_game_manifest,
    handle_game_plan, handle_game_relaunch, handle_game_session_profile, handle_game_sessions,
    handle_game_status, handle_game_stop,
};
pub(crate) use session_launch_device_support::default_session_device_paths;
pub(crate) use session_launch_identity_builder_support::{
    build_game_session_identity, new_empty_game_session,
};
pub(crate) use session_launch_lane_claim_support::{
    activate_and_claim_game_lane, create_game_lane_contract, create_game_lane_resource,
};
pub(crate) use session_launch_lane_record_support::pending_game_lane_record;
pub(crate) use session_launch_loader_support::{
    build_loader_dll_overrides, build_loader_env_overrides,
};
pub(crate) use session_launch_media_builder_support::build_game_session_media;
pub(crate) use session_launch_runtime_builder_support::{
    SessionRuntimePaths, build_game_session_runtime,
};
pub(crate) use session_launch_simulation_support::{
    ensure_simulation_session, run_simulation_frames, write_simulation_start,
};
pub use session_lifecycle_support::{
    game_cleanup_session_artifacts, game_launch_session, game_stop_session,
    shell_cleanup_game_sessions,
};
pub use session_query_support::{
    find_game_session, find_game_session_mut, game_session_missing, game_simulation_key,
    game_simulation_manifest_path, parse_game_pid_arg, parse_game_pid_script_args,
};
pub(crate) use session_render_abi_line_support::render_session_abi_line;
pub(crate) use session_render_identity_line_support::render_session_identity_line;
pub(crate) use session_render_loader_line_support::render_session_loader_line;
pub(crate) use session_render_shim_line_support::render_session_shim_line;
pub use session_support::{
    GameCompatLaneRuntime, GameCompatSession, game_render_session, game_render_session_summary,
    game_render_watch_summary,
};
pub(crate) use shell_support::{StackLineBuffer, shell_wait_event_queue, write_line};
pub use simulation_support::{GameQualityReport, handle_game_next, handle_game_simulate};
pub use watch_command_support::{
    handle_game_watch_poll_all, handle_game_watch_start, handle_game_watch_status,
    handle_game_watch_status_all, handle_game_watch_stop, handle_game_watch_wait,
};

pub const PROGRAM_NAME: &str = "ngos-userland-native";

// --- Front 3: Loader / launcher compatibil ---
