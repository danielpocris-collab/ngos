use alloc::format;
use alloc::string::{String, ToString};

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, write_line};

pub fn render_session_audio<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
) -> Result<(), ExitCode> {
    write_line(
        runtime,
        &format!(
            "game.session.audio pid={} device={} driver={} profile={} audio-batches={} audio-stream={} audio-route={} audio-latency={} audio-spatialization={} audio-completion={} audio-completion-observed={} audio-ops={} audio-bytes={} audio-token={}",
            session.pid,
            session.audio_device_path,
            session.audio_driver_path,
            session.audio_profile,
            session.submitted_audio_batches,
            session
                .last_audio_stream_tag
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_audio_route
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_audio_latency_mode
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_audio_spatialization
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_audio_completion_mode
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_audio_completion_observed
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session.last_audio_op_count,
            session.last_audio_payload_bytes,
            session
                .last_audio_invoke_token
                .map(|token| token.to_string())
                .unwrap_or_else(|| String::from("pending"))
        ),
    )
}
