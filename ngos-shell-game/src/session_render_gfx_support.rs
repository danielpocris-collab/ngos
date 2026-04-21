use alloc::format;
use alloc::string::String;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, write_line};

pub fn render_session_gfx<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
) -> Result<(), ExitCode> {
    write_line(
        runtime,
        &format!(
            "game.session.gfx pid={} device={} driver={} profile={} submitted={} frames={} presented={} last-frame={} gfx-queue={} present-mode={} completion={} completion-observed={} deep-ops={} ops={} bytes={}",
            session.pid,
            session.graphics_device_path,
            session.graphics_driver_path,
            session.graphics_profile,
            session.submitted_frames,
            session.presented_frames,
            session.last_presented,
            session
                .last_frame_tag
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_graphics_queue
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_present_mode
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_completion_mode
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_completion_observed
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_graphics_deep_ops
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session.last_frame_op_count,
            session.last_frame_payload_bytes
        ),
    )
}
