use super::*;

pub(crate) fn encode_block_request_bytes(request: &NativeBlockIoRequest) -> &[u8] {
    unsafe {
        core::slice::from_raw_parts(
            (request as *const NativeBlockIoRequest).cast::<u8>(),
            core::mem::size_of::<NativeBlockIoRequest>(),
        )
    }
}

pub(crate) fn default_block_request_security(
    object_id: u64,
    subject_id: u64,
    rights: ngos_user_abi::BlockRightsMask,
) -> (
    ngos_user_abi::CapabilityToken,
    ngos_user_abi::SecurityLabel,
    ngos_user_abi::ProvenanceTag,
    ngos_user_abi::IntegrityTag,
) {
    let label = ngos_user_abi::SecurityLabel::new(
        ngos_user_abi::ConfidentialityLevel::Internal,
        ngos_user_abi::IntegrityLevel::Verified,
    );
    let integrity = ngos_user_abi::IntegrityTag::zeroed(ngos_user_abi::IntegrityTagKind::None);
    let capability = ngos_user_abi::CapabilityToken {
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
    let provenance = ngos_user_abi::ProvenanceTag {
        origin_kind: ngos_user_abi::ProvenanceOriginKind::Subject,
        reserved0: 0,
        origin_id: subject_id,
        parent_origin_id: 0,
        parent_measurement: [0; 32],
        edge_id: object_id,
        measurement: integrity,
    };
    (capability, label, provenance, integrity)
}

pub(crate) fn try_decode_block_request(bytes: &[u8]) -> Option<NativeBlockIoRequest> {
    if bytes.len() < core::mem::size_of::<NativeBlockIoRequest>() {
        return None;
    }
    let request = unsafe { (bytes.as_ptr() as *const NativeBlockIoRequest).read_unaligned() };
    if request.magic != NATIVE_BLOCK_IO_MAGIC || request.version != NATIVE_BLOCK_IO_VERSION {
        return None;
    }
    Some(request)
}

pub(crate) fn storage_submit_block_write<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    sector: u64,
    payload: &[u8; 512],
) -> Result<(), ExitCode> {
    let record = runtime.inspect_device(device_path).map_err(|_| 246)?;
    let rights =
        ngos_user_abi::BlockRightsMask::WRITE.union(ngos_user_abi::BlockRightsMask::SUBMIT);
    let (capability, label, provenance, integrity) =
        default_block_request_security(0x5354_4f52_4147_4530, 1, rights);
    let request = NativeBlockIoRequest {
        magic: NATIVE_BLOCK_IO_MAGIC,
        version: NATIVE_BLOCK_IO_VERSION,
        op: ngos_user_abi::NATIVE_BLOCK_IO_OP_WRITE,
        sector,
        sector_count: 1,
        block_size: record.block_size,
        rights,
        capability,
        label,
        provenance,
        integrity,
    };
    let fd = runtime.open_path(device_path).map_err(|_| 234)?;
    let mut request_payload =
        Vec::with_capacity(core::mem::size_of::<NativeBlockIoRequest>() + payload.len());
    request_payload.extend_from_slice(encode_block_request_bytes(&request));
    request_payload.extend_from_slice(payload);
    shell_write_all(runtime, fd, &request_payload)?;
    runtime.close(fd).map_err(|_| 240)?;
    Ok(())
}

pub(crate) fn storage_complete_driver_request<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
    completion: &[u8],
) -> Result<NativeBlockIoRequest, ExitCode> {
    let driver_fd = runtime.open_path(driver_path).map_err(|_| 234)?;
    if runtime.poll(driver_fd, POLLIN).unwrap_or(0) != POLLIN {
        let _ = runtime.close(driver_fd);
        return Err(238);
    }
    let mut driver_bytes = [0u8; 1024];
    let driver_read = runtime
        .read(driver_fd, &mut driver_bytes)
        .map_err(|_| 238)?;
    if driver_read == 0 {
        let _ = runtime.close(driver_fd);
        return Err(238);
    }
    let driver_payload = &driver_bytes[..driver_read];
    let driver_prefix_len = driver_payload
        .iter()
        .position(|byte| *byte == b'\n')
        .map(|index| index + 1)
        .unwrap_or(driver_payload.len());
    let request = try_decode_block_request(&driver_payload[driver_prefix_len..]).ok_or(239)?;
    shell_write_all(runtime, driver_fd, completion)?;
    runtime.close(driver_fd).map_err(|_| 240)?;
    Ok(request)
}
