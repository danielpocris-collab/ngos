use alloc::format;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{
    complete_graphics_driver_request, parse_gfx_payload_translation_metadata,
    read_graphics_driver_request_record, write_line,
};

const GPU_PRESENT_OPCODE: u32 = 0x4750_0001;

pub fn drain_graphics_driver_requests<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
    frame_tag: &str,
    frame_payload: &str,
) -> Result<(), ExitCode> {
    while let Some(request) = read_graphics_driver_request_record(runtime, driver_path)? {
        let response_payload = match request.opcode {
            Some(GPU_PRESENT_OPCODE) => frame_tag,
            _ if !request.payload.is_empty() => request.payload.as_str(),
            _ => frame_payload,
        };
        let (source_api, translation) = parse_gfx_payload_translation_metadata(response_payload);
        complete_graphics_driver_request(
            runtime,
            driver_path,
            request.request_id,
            response_payload,
        )?;
        write_line(
            runtime,
            &format!(
                "gpu-complete driver={} bytes={} source-api={} translation={} payload={}",
                driver_path,
                response_payload.len(),
                source_api,
                translation,
                response_payload
            ),
        )?;
    }
    Ok(())
}
