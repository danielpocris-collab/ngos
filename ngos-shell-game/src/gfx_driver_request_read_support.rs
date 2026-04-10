use alloc::string::{String, ToString};
use alloc::vec;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

pub struct GraphicsDriverRequestRecord {
    pub request_id: u64,
    pub opcode: Option<u32>,
    pub payload: String,
}

pub fn read_graphics_driver_request_record<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
) -> Result<Option<GraphicsDriverRequestRecord>, ExitCode> {
    if let Ok(record) = runtime.inspect_driver(driver_path)
        && record.queued_requests == 0
        && record.in_flight_requests == 0
    {
        return Ok(None);
    }
    let fd = runtime.open_path(driver_path).map_err(|_| 234)?;
    let mut buffer = vec![0u8; 4096];
    let count = runtime.read(fd, &mut buffer).map_err(|_| 238)?;
    runtime.close(fd).map_err(|_| 240)?;
    if count == 0 {
        return Ok(None);
    }
    buffer.truncate(count);
    let prefix_len = buffer
        .iter()
        .position(|byte| *byte == b'\n')
        .map(|index| index + 1)
        .unwrap_or(buffer.len());
    let header = core::str::from_utf8(&buffer[..prefix_len]).map_err(|_| 239)?;
    let payload = core::str::from_utf8(&buffer[prefix_len..]).map_err(|_| 239)?;
    let request_id = header
        .strip_prefix("request:")
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or(239)?;
    let opcode = header
        .split_whitespace()
        .find_map(|part| part.strip_prefix("opcode=Some("))
        .and_then(|value| value.strip_suffix(')'))
        .and_then(|value| value.parse::<u32>().ok());
    Ok(Some(GraphicsDriverRequestRecord {
        request_id,
        opcode,
        payload: payload.to_string(),
    }))
}
