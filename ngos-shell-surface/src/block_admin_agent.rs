use alloc::format;
use alloc::vec::Vec;

use ngos_shell_types::{parse_u64_arg, resolve_shell_path};
use ngos_shell_vfs::{shell_write_all, write_line};
use ngos_user_abi::{
    BlockRightsMask, CapabilityToken, ConfidentialityLevel, ExitCode, IntegrityLevel, IntegrityTag,
    IntegrityTagKind, NATIVE_BLOCK_IO_MAGIC, NATIVE_BLOCK_IO_OP_READ, NATIVE_BLOCK_IO_OP_WRITE,
    NATIVE_BLOCK_IO_VERSION, NativeBlockIoRequest, ProvenanceOriginKind, ProvenanceTag,
    SecurityLabel, SyscallBackend,
};
use ngos_user_runtime::Runtime;

const BLOCK_REQUEST_OBJECT_ID: u64 = 0x5354_4f52_4147_4530;
const BLOCK_REQUEST_SUBJECT_ID: u64 = 1;

pub(crate) fn try_handle_block_admin_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    line: &str,
    last_status: &mut i32,
) -> Option<Result<(), ExitCode>> {
    if let Some(rest) = line.strip_prefix("blk-read ") {
        let mut parts = rest.split_whitespace();
        let Some(device_path) = parts.next() else {
            let _ = write_line(runtime, "usage: blk-read <device> <sector> [sector-count]");
            return Some(Err(2));
        };
        let Some(sector) = parse_u64_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: blk-read <device> <sector> [sector-count]");
            return Some(Err(2));
        };
        let sector_count = parts
            .next()
            .and_then(|token| parse_u64_arg(Some(token)))
            .map(|count| count as u32)
            .unwrap_or(1);
        *last_status = match submit_block_read(
            runtime,
            &resolve_shell_path(cwd, device_path),
            sector,
            sector_count,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("blk-write ") {
        let mut parts = rest.split_whitespace();
        let Some(device_path) = parts.next() else {
            let _ = write_line(runtime, "usage: blk-write <device> <sector> <hex-bytes>");
            return Some(Err(2));
        };
        let Some(sector) = parse_u64_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: blk-write <device> <sector> <hex-bytes>");
            return Some(Err(2));
        };
        let Some(payload_token) = parts.next() else {
            let _ = write_line(runtime, "usage: blk-write <device> <sector> <hex-bytes>");
            return Some(Err(2));
        };
        let Ok(payload) = decode_block_write_payload(payload_token) else {
            let _ = write_line(runtime, "usage: blk-write <device> <sector> <hex-bytes>");
            return Some(Err(2));
        };
        *last_status = match submit_block_write(
            runtime,
            &resolve_shell_path(cwd, device_path),
            sector,
            &payload,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    None
}

fn submit_block_read<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    sector: u64,
    sector_count: u32,
) -> Result<(), ExitCode> {
    let record = runtime.inspect_device(device_path).map_err(|_| 246)?;
    if record.block_size == 0 {
        return Err(246);
    }
    let rights = BlockRightsMask::READ.union(BlockRightsMask::SUBMIT);
    let request = build_block_request(
        NATIVE_BLOCK_IO_OP_READ,
        sector,
        sector_count,
        record.block_size,
        rights,
    );
    let fd = runtime.open_path(device_path).map_err(|_| 234)?;
    shell_write_all(runtime, fd, encode_block_request_bytes(&request))?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!(
            "blk-read device={} sector={} sectors={} block-size={}",
            device_path, sector, sector_count, record.block_size
        ),
    )
}

fn submit_block_write<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    sector: u64,
    payload: &[u8],
) -> Result<(), ExitCode> {
    let record = runtime.inspect_device(device_path).map_err(|_| 246)?;
    let block_size = record.block_size as usize;
    if block_size == 0 {
        return Err(246);
    }
    let sector_count = payload.len().div_ceil(block_size);
    let sector_count = u32::try_from(sector_count).map_err(|_| 247)?;
    let mut padded_payload = payload.to_vec();
    padded_payload.resize(sector_count as usize * block_size, 0);
    let rights = BlockRightsMask::WRITE.union(BlockRightsMask::SUBMIT);
    let request = build_block_request(
        NATIVE_BLOCK_IO_OP_WRITE,
        sector,
        sector_count,
        record.block_size,
        rights,
    );
    let request_bytes = encode_block_request_with_payload(&request, &padded_payload);
    let fd = runtime.open_path(device_path).map_err(|_| 234)?;
    shell_write_all(runtime, fd, &request_bytes)?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!(
            "blk-write device={} sector={} sectors={} bytes={} block-size={}",
            device_path,
            sector,
            sector_count,
            payload.len(),
            record.block_size
        ),
    )
}

fn build_block_request(
    op: u16,
    sector: u64,
    sector_count: u32,
    block_size: u32,
    rights: BlockRightsMask,
) -> NativeBlockIoRequest {
    let (capability, label, provenance, integrity) =
        default_block_request_security(BLOCK_REQUEST_OBJECT_ID, BLOCK_REQUEST_SUBJECT_ID, rights);
    NativeBlockIoRequest {
        magic: NATIVE_BLOCK_IO_MAGIC,
        version: NATIVE_BLOCK_IO_VERSION,
        op,
        sector,
        sector_count,
        block_size,
        rights,
        capability,
        label,
        provenance,
        integrity,
    }
}

fn encode_block_request_with_payload(request: &NativeBlockIoRequest, payload: &[u8]) -> Vec<u8> {
    let mut bytes =
        Vec::with_capacity(core::mem::size_of::<NativeBlockIoRequest>() + payload.len());
    bytes.extend_from_slice(encode_block_request_bytes(request));
    bytes.extend_from_slice(payload);
    bytes
}

fn decode_block_write_payload(token: &str) -> Result<Vec<u8>, ()> {
    let raw = token.strip_prefix("0x").unwrap_or(token);
    let mut compact = Vec::with_capacity(raw.len());
    for byte in raw.bytes() {
        if matches!(byte, b'_' | b'-' | b':') {
            continue;
        }
        compact.push(byte);
    }
    if compact.is_empty() || compact.len() % 2 != 0 {
        return Err(());
    }
    let mut payload = Vec::with_capacity(compact.len() / 2);
    let mut index = 0;
    while index < compact.len() {
        let high = decode_hex_nibble(compact[index]).ok_or(())?;
        let low = decode_hex_nibble(compact[index + 1]).ok_or(())?;
        payload.push((high << 4) | low);
        index += 2;
    }
    Ok(payload)
}

fn decode_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn encode_block_request_bytes(request: &NativeBlockIoRequest) -> &[u8] {
    unsafe {
        core::slice::from_raw_parts(
            (request as *const NativeBlockIoRequest).cast::<u8>(),
            core::mem::size_of::<NativeBlockIoRequest>(),
        )
    }
}

fn default_block_request_security(
    object_id: u64,
    subject_id: u64,
    rights: BlockRightsMask,
) -> (CapabilityToken, SecurityLabel, ProvenanceTag, IntegrityTag) {
    let label = SecurityLabel::new(ConfidentialityLevel::Internal, IntegrityLevel::Verified);
    let integrity = IntegrityTag::zeroed(IntegrityTagKind::None);
    let capability = CapabilityToken {
        object_id,
        rights,
        issuer_id: subject_id,
        subject_id,
        generation: 1,
        revocation_epoch: 1,
        delegation_depth: 0,
        delegated: 0,
        nonce: object_id ^ subject_id ^ rights.0,
        expiry_epoch: u64::MAX,
        authenticator: integrity,
    };
    let provenance = ProvenanceTag {
        origin_kind: ProvenanceOriginKind::Subject,
        reserved0: 0,
        origin_id: subject_id,
        parent_origin_id: 0,
        parent_measurement: [0; 32],
        edge_id: object_id,
        measurement: integrity,
    };
    (capability, label, provenance, integrity)
}

#[cfg(test)]
mod tests {
    extern crate std;

    use alloc::vec;
    use core::cell::RefCell;

    use ngos_user_abi::{
        Errno, NativeDeviceRecord, SYS_CLOSE, SYS_INSPECT_DEVICE, SYS_OPEN_PATH, SYS_WRITE,
        SYS_WRITEV, SyscallFrame, SyscallReturn, UserIoVec,
    };

    use super::*;

    struct BlockWriteBackend {
        device_writes: RefCell<Vec<Vec<u8>>>,
        stdout: RefCell<Vec<Vec<u8>>>,
    }

    impl Default for BlockWriteBackend {
        fn default() -> Self {
            Self {
                device_writes: RefCell::new(Vec::new()),
                stdout: RefCell::new(Vec::new()),
            }
        }
    }

    impl SyscallBackend for BlockWriteBackend {
        unsafe fn syscall(&self, frame: SyscallFrame) -> SyscallReturn {
            match frame.number {
                SYS_INSPECT_DEVICE => {
                    let record_ptr = frame.arg2 as *mut NativeDeviceRecord;
                    unsafe {
                        record_ptr.write(NativeDeviceRecord {
                            class: 0,
                            state: 0,
                            reserved0: 0,
                            queue_depth: 0,
                            queue_capacity: 0,
                            submitted_requests: 0,
                            completed_requests: 0,
                            total_latency_ticks: 0,
                            max_latency_ticks: 0,
                            total_queue_wait_ticks: 0,
                            max_queue_wait_ticks: 0,
                            link_up: 1,
                            reserved1: 0,
                            block_size: 512,
                            reserved2: 0,
                            capacity_bytes: 512 * 32,
                            last_completed_request_id: 0,
                            last_completed_frame_tag: [0; 64],
                            last_completed_source_api_name: [0; 24],
                            last_completed_translation_label: [0; 32],
                            last_terminal_request_id: 0,
                            last_terminal_state: 0,
                            reserved3: 0,
                            last_terminal_frame_tag: [0; 64],
                            last_terminal_source_api_name: [0; 24],
                            last_terminal_translation_label: [0; 32],
                        });
                    }
                    SyscallReturn::ok(0)
                }
                SYS_OPEN_PATH => SyscallReturn::ok(9),
                SYS_WRITE => {
                    let bytes = unsafe {
                        core::slice::from_raw_parts(frame.arg1 as *const u8, frame.arg2).to_vec()
                    };
                    if frame.arg0 == 9 {
                        self.device_writes.borrow_mut().push(bytes);
                    } else {
                        self.stdout.borrow_mut().push(bytes);
                    }
                    SyscallReturn::ok(frame.arg2)
                }
                SYS_WRITEV => {
                    let iovecs = unsafe {
                        core::slice::from_raw_parts(frame.arg1 as *const UserIoVec, frame.arg2)
                    };
                    let mut bytes = Vec::new();
                    for iovec in iovecs {
                        let chunk = unsafe {
                            core::slice::from_raw_parts(iovec.base as *const u8, iovec.len)
                        };
                        bytes.extend_from_slice(chunk);
                    }
                    self.stdout.borrow_mut().push(bytes);
                    SyscallReturn::ok(iovecs.iter().map(|iovec| iovec.len).sum())
                }
                SYS_CLOSE => SyscallReturn::ok(0),
                _ => SyscallReturn::err(Errno::Inval),
            }
        }
    }

    #[test]
    fn decode_block_write_payload_accepts_prefixed_hex_with_separators() {
        let payload = decode_block_write_payload("0x41:42_43-44").unwrap();
        assert_eq!(payload, vec![0x41, 0x42, 0x43, 0x44]);
    }

    #[test]
    fn decode_block_write_payload_rejects_invalid_hex() {
        assert!(decode_block_write_payload("").is_err());
        assert!(decode_block_write_payload("0").is_err());
        assert!(decode_block_write_payload("xx").is_err());
    }

    #[test]
    fn blk_write_emits_write_request_with_padded_payload() {
        let runtime = Runtime::new(BlockWriteBackend::default());
        let mut last_status = -1;

        let handled = try_handle_block_admin_command(
            &runtime,
            "/dev/storage0",
            "blk-write /dev/storage0 7 41424344",
            &mut last_status,
        );

        assert_eq!(handled, Some(Ok(())));
        assert_eq!(last_status, 0);

        let writes = runtime.backend().device_writes.borrow();
        assert_eq!(writes.len(), 1);
        let request_bytes = &writes[0];
        let header_size = core::mem::size_of::<NativeBlockIoRequest>();
        assert_eq!(request_bytes.len(), header_size + 512);

        let request =
            unsafe { (request_bytes.as_ptr() as *const NativeBlockIoRequest).read_unaligned() };
        assert_eq!(request.op, NATIVE_BLOCK_IO_OP_WRITE);
        assert_eq!(request.sector, 7);
        assert_eq!(request.sector_count, 1);
        assert_eq!(request.block_size, 512);
        assert!(request.rights.contains(BlockRightsMask::WRITE));
        assert!(request.rights.contains(BlockRightsMask::SUBMIT));
        assert_eq!(&request_bytes[header_size..header_size + 4], b"ABCD");
        assert!(
            request_bytes[header_size + 4..]
                .iter()
                .all(|byte| *byte == 0)
        );

        let stdout = runtime.backend().stdout.borrow();
        assert!(stdout.iter().any(|line| {
            core::str::from_utf8(line).unwrap().contains(
                "blk-write device=/dev/storage0 sector=7 sectors=1 bytes=4 block-size=512",
            )
        }));
    }
}
