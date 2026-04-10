use alloc::format;
use alloc::string::String;
use alloc::vec;

use ngos_gfx_translate::EncodedFrame;
use ngos_shell_vfs::shell_write_all;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{parse_gfx_payload_translation_metadata, write_line};

const GPU_PRESENT_OPCODE: u32 = 0x4750_0001;

pub fn shell_gpu_submit<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    payload: &str,
) -> Result<(), ExitCode> {
    let (source_api, translation) = parse_gfx_payload_translation_metadata(payload);
    let fd = runtime.open_path(device_path).map_err(|_| 234)?;
    shell_write_all(runtime, fd, payload.as_bytes())?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!(
            "gpu-submit device={} bytes={} source-api={} translation={} payload={}",
            device_path,
            payload.len(),
            source_api,
            translation,
            payload
        ),
    )
}

fn encode_graphics_present_payload(encoded: &EncodedFrame) -> String {
    let mut lines = vec![
        format!("frame={}", encoded.frame_tag),
        format!("queue={}", encoded.queue),
        format!("present-mode={}", encoded.present_mode),
        format!("completion={}", encoded.completion),
    ];
    if let Some(source_api) = encoded.source_api.as_deref() {
        lines.push(format!("source-api={source_api}"));
    }
    if let Some(translation) = encoded.translation_label.as_deref() {
        lines.push(format!("translation={translation}"));
    }
    lines.join("\n")
}

pub fn shell_gpu_present_encoded<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    encoded: &EncodedFrame,
) -> Result<(), ExitCode> {
    let present_payload = encode_graphics_present_payload(encoded);
    let response = runtime
        .present_gpu_frame(device_path, present_payload.as_bytes())
        .map_err(|_| 246)?;
    let source_api = encoded.source_api.as_deref().unwrap_or("-");
    let translation = encoded.translation_label.as_deref().unwrap_or("-");
    write_line(
        runtime,
        &format!(
            "gpu-present device={} opcode=0x{:08x} response=0x{:08x} source-api={} translation={} frame={}",
            device_path, GPU_PRESENT_OPCODE, response, source_api, translation, encoded.frame_tag
        ),
    )
}
