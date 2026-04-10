use alloc::format;
use alloc::string::{String, ToString};

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, write_line};

pub fn render_session_input<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
) -> Result<(), ExitCode> {
    write_line(
        runtime,
        &format!(
            "game.session.input pid={} device={} driver={} profile={} input-batches={} input-frame={} input-family={} input-layout={} input-key-table={} pointer-capture={} input-delivery={} input-delivery-observed={} input-ops={} input-bytes={} input-token={}",
            session.pid,
            session.input_device_path,
            session.input_driver_path,
            session.input_profile,
            session.submitted_input_batches,
            session
                .last_input_frame_tag
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_input_family
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_input_layout
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_input_key_table
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_pointer_capture
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_input_delivery_mode
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_input_delivery_observed
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session.last_input_op_count,
            session.last_input_payload_bytes,
            session
                .last_input_invoke_token
                .map(|token| token.to_string())
                .unwrap_or_else(|| String::from("pending"))
        ),
    )
}
