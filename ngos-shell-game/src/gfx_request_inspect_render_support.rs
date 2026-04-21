use alloc::format;

use ngos_shell_proc::fixed_text_field;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, gpu_request_kind_name, gpu_request_state_name, write_line};

pub fn write_retained_gfx_request<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
    session: &GameCompatSession,
    request_id: u64,
) -> Result<(), ExitCode> {
    let record = runtime
        .inspect_device_request(request_id)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "game.gfx.request pid={} request={} driver={} api={} translation={} outcome=retained kind={} state={} opcode=0x{:08x} buffer={} payload={} response={} submitted={} started={} completed={} frame={} request-api={} request-translation={}",
            pid,
            request_id,
            session.graphics_driver_path,
            session.graphics_translation.source_api_name,
            session.graphics_translation.translation,
            gpu_request_kind_name(record.kind),
            gpu_request_state_name(record.state),
            record.opcode as u32,
            record.buffer_id,
            record.payload_len,
            record.response_len,
            record.submitted_tick,
            record.started_tick,
            record.completed_tick,
            fixed_text_field(&record.frame_tag),
            fixed_text_field(&record.source_api_name),
            fixed_text_field(&record.translation_label)
        ),
    )
}
