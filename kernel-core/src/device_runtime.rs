use super::*;
use crate::device_model::{
    DeviceEndpoint, DeviceRegistry, DeviceRequest, DriverEndpoint, GpuBufferObject, NetworkBuffer,
    NetworkBufferState, NetworkInterface, NetworkSocket, SocketRxPacket,
};
use crate::eventing_model::GraphicsEventKind;
use crate::runtime_core::RuntimeChannel;

fn device_class_for_path(path: &str) -> DeviceClass {
    if path.starts_with("/net/") || path.starts_with("/dev/net") {
        DeviceClass::Network
    } else if path.starts_with("/gpu/") || path.starts_with("/dev/gpu") {
        DeviceClass::Graphics
    } else if path.starts_with("/audio/") || path.starts_with("/dev/audio") {
        DeviceClass::Audio
    } else if path.starts_with("/input/") || path.starts_with("/dev/input") {
        DeviceClass::Input
    } else if path.starts_with("/blk/")
        || path.starts_with("/dev/disk")
        || path.starts_with("/dev/storage")
    {
        DeviceClass::Storage
    } else {
        DeviceClass::Generic
    }
}

fn default_queue_capacity(class: DeviceClass) -> usize {
    match class {
        DeviceClass::Generic => 32,
        DeviceClass::Network => 256,
        DeviceClass::Storage => 128,
        DeviceClass::Graphics => 64,
        DeviceClass::Audio => 128,
        DeviceClass::Input => 64,
    }
}

fn default_block_size(class: DeviceClass) -> u32 {
    match class {
        DeviceClass::Storage => 4096,
        DeviceClass::Generic
        | DeviceClass::Network
        | DeviceClass::Graphics
        | DeviceClass::Audio
        | DeviceClass::Input => 0,
    }
}

fn default_capacity_bytes(class: DeviceClass) -> u64 {
    match class {
        DeviceClass::Storage => 1024 * 1024 * 1024,
        DeviceClass::Generic
        | DeviceClass::Network
        | DeviceClass::Graphics
        | DeviceClass::Audio
        | DeviceClass::Input => 0,
    }
}

fn synthetic_mac_for_path(path: &str) -> [u8; 6] {
    let mut mac = [0x02, 0, 0, 0, 0, 0];
    for (index, byte) in path.as_bytes().iter().enumerate() {
        mac[(index % 5) + 1] ^= *byte;
    }
    mac
}

fn synthetic_ipv4_for_path(path: &str) -> [u8; 4] {
    let mut tail = 1u8;
    for byte in path.as_bytes() {
        tail = tail.wrapping_add(*byte);
    }
    [10, 0, 0, tail.max(2)]
}

fn checksum16(bytes: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut chunks = bytes.chunks_exact(2);
    for chunk in &mut chunks {
        sum = sum.wrapping_add(u16::from_be_bytes([chunk[0], chunk[1]]) as u32);
    }
    let rem = chunks.remainder();
    if let Some(byte) = rem.first() {
        sum = sum.wrapping_add((*byte as u32) << 8);
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

fn build_udp_ipv4_frame(
    src_mac: [u8; 6],
    dst_mac: [u8; 6],
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
    src_port: u16,
    dst_port: u16,
    payload: &[u8],
) -> Vec<u8> {
    let udp_len = 8 + payload.len();
    let ip_len = 20 + udp_len;
    let mut frame = Vec::with_capacity(14 + ip_len);
    frame.extend_from_slice(&dst_mac);
    frame.extend_from_slice(&src_mac);
    frame.extend_from_slice(&0x0800u16.to_be_bytes());

    let mut ip_header = [0u8; 20];
    ip_header[0] = 0x45;
    ip_header[1] = 0;
    ip_header[2..4].copy_from_slice(&(ip_len as u16).to_be_bytes());
    ip_header[4..6].copy_from_slice(&0u16.to_be_bytes());
    ip_header[6..8].copy_from_slice(&0x4000u16.to_be_bytes());
    ip_header[8] = 64;
    ip_header[9] = 17;
    ip_header[12..16].copy_from_slice(&src_ip);
    ip_header[16..20].copy_from_slice(&dst_ip);
    let ip_checksum = checksum16(&ip_header);
    ip_header[10..12].copy_from_slice(&ip_checksum.to_be_bytes());
    frame.extend_from_slice(&ip_header);

    let mut udp = Vec::with_capacity(udp_len);
    udp.extend_from_slice(&src_port.to_be_bytes());
    udp.extend_from_slice(&dst_port.to_be_bytes());
    udp.extend_from_slice(&(udp_len as u16).to_be_bytes());
    udp.extend_from_slice(&0u16.to_be_bytes());
    udp.extend_from_slice(payload);

    let mut pseudo = Vec::with_capacity(12 + udp.len());
    pseudo.extend_from_slice(&src_ip);
    pseudo.extend_from_slice(&dst_ip);
    pseudo.push(0);
    pseudo.push(17);
    pseudo.extend_from_slice(&(udp_len as u16).to_be_bytes());
    pseudo.extend_from_slice(&udp);
    let udp_checksum = checksum16(&pseudo);
    udp[6..8].copy_from_slice(&udp_checksum.to_be_bytes());
    frame.extend_from_slice(&udp);
    frame
}

type ParsedUdpIpv4Frame = ([u8; 6], [u8; 6], [u8; 4], [u8; 4], u16, u16, Vec<u8>);

fn parse_udp_ipv4_frame(frame: &[u8]) -> Option<ParsedUdpIpv4Frame> {
    if frame.len() < 14 + 20 + 8 {
        return None;
    }
    let ethertype = u16::from_be_bytes([frame[12], frame[13]]);
    if ethertype != 0x0800 {
        return None;
    }
    let ihl = (frame[14] & 0x0f) as usize * 4;
    if ihl < 20 || frame.len() < 14 + ihl + 8 {
        return None;
    }
    if frame[23] != 17 {
        return None;
    }
    let src_mac = <[u8; 6]>::try_from(&frame[6..12]).ok()?;
    let dst_mac = <[u8; 6]>::try_from(&frame[0..6]).ok()?;
    let src_ip = <[u8; 4]>::try_from(&frame[26..30]).ok()?;
    let dst_ip = <[u8; 4]>::try_from(&frame[30..34]).ok()?;
    let udp_start = 14 + ihl;
    let src_port = u16::from_be_bytes([frame[udp_start], frame[udp_start + 1]]);
    let dst_port = u16::from_be_bytes([frame[udp_start + 2], frame[udp_start + 3]]);
    let udp_len = u16::from_be_bytes([frame[udp_start + 4], frame[udp_start + 5]]) as usize;
    if udp_len < 8 || frame.len() < udp_start + udp_len {
        return None;
    }
    let payload = frame[udp_start + 8..udp_start + udp_len].to_vec();
    Some((
        src_mac, dst_mac, src_ip, dst_ip, src_port, dst_port, payload,
    ))
}

fn driver_mut<'a>(
    registry: &'a mut DeviceRegistry,
    path: &str,
) -> Result<&'a mut DriverEndpoint, DeviceModelError> {
    registry
        .drivers
        .iter_mut()
        .find(|driver| driver.path == path)
        .ok_or(DeviceModelError::InvalidDriver)
}

fn device_mut<'a>(
    registry: &'a mut DeviceRegistry,
    path: &str,
) -> Result<&'a mut DeviceEndpoint, DeviceModelError> {
    registry
        .devices
        .iter_mut()
        .find(|device| device.path == path)
        .ok_or(DeviceModelError::InvalidDevice)
}

fn path_inode(runtime: &KernelRuntime, path: &str) -> Result<u64, RuntimeError> {
    Ok(runtime.stat_path(path)?.inode)
}

fn graphics_event_device_inode(runtime: &KernelRuntime, device_path: &str) -> Option<u64> {
    runtime
        .stat_path(device_path)
        .ok()
        .map(|status| status.inode)
}

fn is_graphics_driver(runtime: &KernelRuntime, driver_path: &str) -> bool {
    runtime
        .device_registry
        .drivers
        .iter()
        .find(|driver| driver.path == driver_path)
        .is_some_and(|driver| {
            driver.bound_devices.iter().any(|device_path| {
                runtime
                    .device_registry
                    .devices
                    .iter()
                    .find(|device| device.path == *device_path)
                    .is_some_and(|device| device.class == DeviceClass::Graphics)
            })
        })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DriverRequestOutcome {
    Complete,
    Fail,
    Cancel,
}

fn parse_driver_completion_target<'a>(
    bytes: &'a [u8],
    fallback_request_id: Option<u64>,
) -> Result<(u64, &'a [u8], DriverRequestOutcome), DeviceModelError> {
    if let Some(rest) = bytes.strip_prefix(b"cancel-request:") {
        let newline = rest
            .iter()
            .position(|byte| *byte == b'\n')
            .ok_or(DeviceModelError::InvalidRequestState)?;
        let header = core::str::from_utf8(&rest[..newline])
            .map_err(|_| DeviceModelError::InvalidRequestState)?;
        let request_id = header
            .parse::<u64>()
            .map_err(|_| DeviceModelError::InvalidRequestState)?;
        return Ok((
            request_id,
            &rest[newline + 1..],
            DriverRequestOutcome::Cancel,
        ));
    }
    if let Some(rest) = bytes.strip_prefix(b"failed-request:") {
        let newline = rest
            .iter()
            .position(|byte| *byte == b'\n')
            .ok_or(DeviceModelError::InvalidRequestState)?;
        let header = core::str::from_utf8(&rest[..newline])
            .map_err(|_| DeviceModelError::InvalidRequestState)?;
        let request_id = header
            .parse::<u64>()
            .map_err(|_| DeviceModelError::InvalidRequestState)?;
        return Ok((request_id, &rest[newline + 1..], DriverRequestOutcome::Fail));
    }
    if let Some(rest) = bytes.strip_prefix(b"request:") {
        let newline = rest
            .iter()
            .position(|byte| *byte == b'\n')
            .ok_or(DeviceModelError::InvalidRequestState)?;
        let header = core::str::from_utf8(&rest[..newline])
            .map_err(|_| DeviceModelError::InvalidRequestState)?;
        let request_id = header
            .parse::<u64>()
            .map_err(|_| DeviceModelError::InvalidRequestState)?;
        return Ok((
            request_id,
            &rest[newline + 1..],
            DriverRequestOutcome::Complete,
        ));
    }
    let request_id = fallback_request_id.ok_or(DeviceModelError::InvalidRequestState)?;
    Ok((request_id, bytes, DriverRequestOutcome::Complete))
}

fn complete_device_driver_request(
    runtime: &mut KernelRuntime,
    driver_path: &str,
    bytes: &[u8],
) -> Result<bool, RuntimeError> {
    let explicit_target = bytes.starts_with(b"cancel-request:")
        || bytes.starts_with(b"failed-request:")
        || bytes.starts_with(b"request:");
    let fallback_request_id = runtime
        .device_registry
        .drivers
        .iter()
        .find(|driver| driver.path == driver_path)
        .and_then(|driver| driver.in_flight_requests.first().copied());
    if fallback_request_id.is_none() && !explicit_target {
        return Ok(false);
    }
    let (request_id, payload, outcome) =
        parse_driver_completion_target(bytes, fallback_request_id)?;
    let driver = runtime
        .device_registry
        .drivers
        .iter()
        .find(|driver| driver.path == driver_path)
        .ok_or(DeviceModelError::InvalidDriver)?;
    let active = match outcome {
        DriverRequestOutcome::Cancel => {
            driver.in_flight_requests.contains(&request_id)
                || driver.queued_requests.contains(&request_id)
        }
        DriverRequestOutcome::Complete | DriverRequestOutcome::Fail => {
            driver.in_flight_requests.contains(&request_id)
        }
    };
    if !active {
        return Err(DeviceModelError::InvalidRequestState.into());
    }
    let request = runtime
        .device_registry
        .requests
        .iter_mut()
        .find(|request| request.id == request_id)
        .ok_or(DeviceModelError::RequestNotFound)?;
    let valid_state = match outcome {
        DriverRequestOutcome::Cancel => matches!(
            request.state,
            DeviceRequestState::Queued | DeviceRequestState::InFlight
        ),
        DriverRequestOutcome::Complete | DriverRequestOutcome::Fail => {
            request.state == DeviceRequestState::InFlight
        }
    };
    if !valid_state {
        return Err(DeviceModelError::InvalidRequestState.into());
    }
    request.response.clear();
    request.response.extend_from_slice(payload);
    request.state = match outcome {
        DriverRequestOutcome::Complete => DeviceRequestState::Completed,
        DriverRequestOutcome::Fail => DeviceRequestState::Failed,
        DriverRequestOutcome::Cancel => DeviceRequestState::Canceled,
    };
    request.completed_tick = Some(runtime.current_tick);
    let request_kind = request.kind;
    let request_opcode = request.opcode;
    let request_buffer_id = request.graphics_buffer_id;
    let request_submitted_tick = request.submitted_tick;
    let request_started_tick = request.started_tick;
    let device_path = request.device_path.clone();
    let queue_drained = {
        let device = device_mut(&mut runtime.device_registry, &device_path)?;
        device
            .pending_requests
            .retain(|candidate| *candidate != request_id);
        if device.class == DeviceClass::Graphics && request_kind == DeviceRequestKind::Control {
            device.graphics_control_reserve_armed = false;
        }
        if device.class == DeviceClass::Graphics
            && outcome == DriverRequestOutcome::Complete
            && request_kind == DeviceRequestKind::Control
            && request_opcode == Some(0x4750_0001)
        {
            device.graphics_presented_frames = device.graphics_presented_frames.saturating_add(1);
            device.graphics_last_presented_frame.clear();
            device
                .graphics_last_presented_frame
                .extend_from_slice(payload);
        }
        if outcome != DriverRequestOutcome::Cancel {
            device.completion_queue.push(request_id);
            device.completed_requests = device.completed_requests.saturating_add(1);
        }
        let latency_ticks = runtime.current_tick.saturating_sub(request_submitted_tick);
        device.total_latency_ticks = device.total_latency_ticks.saturating_add(latency_ticks);
        device.max_latency_ticks = device.max_latency_ticks.max(latency_ticks);
        let queue_wait_ticks = request_started_tick
            .map(|tick| tick.saturating_sub(request_submitted_tick))
            .unwrap_or(latency_ticks);
        device.total_queue_wait_ticks = device
            .total_queue_wait_ticks
            .saturating_add(queue_wait_ticks);
        device.max_queue_wait_ticks = device.max_queue_wait_ticks.max(queue_wait_ticks);
        if device.class == DeviceClass::Graphics && device.pending_requests.is_empty() {
            device.graphics_control_reserve_armed = true;
        }
        device.pending_requests.is_empty()
    };
    if let Some(buffer_id) = request_buffer_id {
        if let Some(buffer) = runtime
            .device_registry
            .gpu_buffers
            .iter_mut()
            .find(|buffer| buffer.id == buffer_id)
        {
            buffer.busy = false;
        }
    }
    if let Some(device_inode) = graphics_event_device_inode(runtime, &device_path) {
        let _ = event_queue_runtime::emit_graphics_events(
            runtime,
            device_inode,
            request_id,
            match outcome {
                DriverRequestOutcome::Complete => GraphicsEventKind::Completed,
                DriverRequestOutcome::Fail => GraphicsEventKind::Failed,
                DriverRequestOutcome::Cancel => GraphicsEventKind::Canceled,
            },
        );
        if queue_drained {
            let _ = event_queue_runtime::emit_graphics_events(
                runtime,
                device_inode,
                request_id,
                GraphicsEventKind::Drained,
            );
        }
    }
    {
        let driver = driver_mut(&mut runtime.device_registry, driver_path)?;
        driver
            .in_flight_requests
            .retain(|candidate| *candidate != request_id);
        driver
            .queued_requests
            .retain(|candidate| *candidate != request_id);
        if outcome != DriverRequestOutcome::Cancel {
            driver.completed_requests = driver.completed_requests.saturating_add(1);
        }
    }
    let _ = refresh_and_notify_bindings_for_paths(runtime, &[&device_path, driver_path]);
    Ok(true)
}

fn reset_graphics_driver(
    runtime: &mut KernelRuntime,
    driver_path: &str,
) -> Result<u32, RuntimeError> {
    let bound_devices = runtime
        .device_registry
        .drivers
        .iter()
        .find(|driver| driver.path == driver_path)
        .ok_or(DeviceModelError::InvalidDriver)?
        .bound_devices
        .iter()
        .filter(|device_path| {
            runtime
                .device_registry
                .devices
                .iter()
                .find(|device| device.path == **device_path)
                .is_some_and(|device| device.class == DeviceClass::Graphics)
        })
        .cloned()
        .collect::<Vec<_>>();
    if bound_devices.is_empty() {
        return Err(DeviceModelError::InvalidDriver.into());
    }
    if let Ok(driver) = driver_mut(&mut runtime.device_registry, driver_path) {
        driver.state = DriverState::Faulted;
    }
    for device_path in &bound_devices {
        if let Ok(device) = device_mut(&mut runtime.device_registry, device_path) {
            device.state = DeviceState::Faulted;
            if let Some(device_inode) = graphics_event_device_inode(runtime, device_path) {
                let _ = event_queue_runtime::emit_graphics_events(
                    runtime,
                    device_inode,
                    0,
                    GraphicsEventKind::Faulted,
                );
            }
        }
    }
    let mut canceled = 0usize;
    for device_path in &bound_devices {
        for issuer in runtime
            .device_registry
            .requests
            .iter()
            .filter(|request| {
                request.device_path == *device_path
                    && matches!(
                        request.state,
                        DeviceRequestState::Queued | DeviceRequestState::InFlight
                    )
            })
            .map(|request| request.issuer)
            .collect::<Vec<_>>()
        {
            canceled = canceled
                .saturating_add(runtime.cancel_graphics_requests_for_issuer(device_path, issuer)?);
        }
    }
    if let Ok(driver) = driver_mut(&mut runtime.device_registry, driver_path) {
        driver.state = DriverState::Active;
    }
    for device_path in &bound_devices {
        if let Ok(device) = device_mut(&mut runtime.device_registry, device_path) {
            device.state = DeviceState::Bound;
            if device.class == DeviceClass::Graphics && device.pending_requests.is_empty() {
                device.graphics_control_reserve_armed = true;
            }
            if let Some(device_inode) = graphics_event_device_inode(runtime, device_path) {
                let _ = event_queue_runtime::emit_graphics_events(
                    runtime,
                    device_inode,
                    0,
                    GraphicsEventKind::Recovered,
                );
            }
        }
    }
    let mut notify_paths = Vec::with_capacity(bound_devices.len() + 1);
    notify_paths.push(driver_path);
    for device_path in &bound_devices {
        notify_paths.push(device_path.as_str());
    }
    let _ = refresh_and_notify_bindings_for_paths(runtime, &notify_paths);
    Ok(canceled as u32)
}

fn retire_graphics_driver(
    runtime: &mut KernelRuntime,
    driver_path: &str,
) -> Result<u32, RuntimeError> {
    let bound_devices = runtime
        .device_registry
        .drivers
        .iter()
        .find(|driver| driver.path == driver_path)
        .ok_or(DeviceModelError::InvalidDriver)?
        .bound_devices
        .iter()
        .filter(|device_path| {
            runtime
                .device_registry
                .devices
                .iter()
                .find(|device| device.path == **device_path)
                .is_some_and(|device| device.class == DeviceClass::Graphics)
        })
        .cloned()
        .collect::<Vec<_>>();
    if bound_devices.is_empty() {
        return Err(DeviceModelError::InvalidDriver.into());
    }
    if runtime
        .device_registry
        .drivers
        .iter()
        .find(|driver| driver.path == driver_path)
        .is_some_and(|driver| driver.state == DriverState::Retired)
    {
        return Err(DeviceModelError::InvalidDriver.into());
    }
    let mut canceled = 0usize;
    for device_path in &bound_devices {
        for issuer in runtime
            .device_registry
            .requests
            .iter()
            .filter(|request| {
                request.device_path == *device_path
                    && matches!(
                        request.state,
                        DeviceRequestState::Queued | DeviceRequestState::InFlight
                    )
            })
            .map(|request| request.issuer)
            .collect::<Vec<_>>()
        {
            canceled = canceled
                .saturating_add(runtime.cancel_graphics_requests_for_issuer(device_path, issuer)?);
        }
    }
    if let Ok(driver) = driver_mut(&mut runtime.device_registry, driver_path) {
        driver.state = DriverState::Retired;
        driver.queued_requests.clear();
        driver.in_flight_requests.clear();
    }
    for device_path in &bound_devices {
        if let Ok(device) = device_mut(&mut runtime.device_registry, device_path) {
            device.state = DeviceState::Retired;
            device.pending_requests.clear();
            device.completion_queue.clear();
            device.graphics_control_reserve_armed = false;
            if let Some(device_inode) = graphics_event_device_inode(runtime, device_path) {
                let _ = event_queue_runtime::emit_graphics_events(
                    runtime,
                    device_inode,
                    0,
                    GraphicsEventKind::Retired,
                );
            }
        }
    }
    let mut notify_paths = Vec::with_capacity(bound_devices.len() + 1);
    notify_paths.push(driver_path);
    for device_path in &bound_devices {
        notify_paths.push(device_path.as_str());
    }
    let _ = refresh_and_notify_bindings_for_paths(runtime, &notify_paths);
    Ok(canceled as u32)
}

fn graphics_resource_name_for_device_path(path: &str) -> Option<&str> {
    path.strip_prefix("/dev/")
}

fn runtime_channel_mut<'a>(
    runtime: &'a mut KernelRuntime,
    path: &str,
) -> Option<&'a mut RuntimeChannel> {
    runtime
        .runtime_channels
        .iter_mut()
        .find(|channel| channel.path == path)
}

fn ensure_runtime_channel<'a>(
    runtime: &'a mut KernelRuntime,
    path: &str,
) -> &'a mut RuntimeChannel {
    if let Some(index) = runtime
        .runtime_channels
        .iter()
        .position(|channel| channel.path == path)
    {
        return &mut runtime.runtime_channels[index];
    }
    runtime.runtime_channels.push(RuntimeChannel {
        path: path.to_string(),
        messages: Vec::new(),
    });
    runtime
        .runtime_channels
        .last_mut()
        .expect("runtime channel was just created")
}

fn enforce_graphics_device_lease(
    runtime: &KernelRuntime,
    owner: ProcessId,
    device_path: &str,
) -> Result<(), RuntimeError> {
    if graphics_resource_name_for_device_path(device_path).is_none() {
        return Err(RuntimeError::NativeModel(
            NativeModelError::ProcessContractMissing {
                kind: ContractKind::Display,
            },
        ));
    };
    for (_, resource) in runtime.resources.objects.iter() {
        if !matches!(resource.kind, ResourceKind::Device | ResourceKind::Surface) {
            continue;
        }
        let Some(holder) = resource.holder else {
            continue;
        };
        let contract = runtime.contracts.get(holder)?;
        if contract.issuer != owner || contract.kind != ContractKind::Display {
            continue;
        }
        if contract.state != ContractState::Active {
            return Err(RuntimeError::NativeModel(
                NativeModelError::ContractNotActive {
                    state: contract.state,
                },
            ));
        }
        if resource.state != ResourceState::Active {
            return Err(RuntimeError::NativeModel(
                NativeModelError::ResourceNotActive {
                    state: resource.state,
                },
            ));
        }
        return Ok(());
    }
    Err(RuntimeError::NativeModel(
        NativeModelError::ProcessContractMissing {
            kind: ContractKind::Display,
        },
    ))
}

fn network_effective_link_up(iface: &NetworkInterface) -> bool {
    iface.admin_up && iface.link_up
}

fn network_buffer_payload(iface: &NetworkInterface, buffer_id: u64) -> Result<&[u8], RuntimeError> {
    iface
        .buffers
        .iter()
        .find(|buffer| buffer.id == buffer_id)
        .map(|buffer| buffer.payload.as_slice())
        .ok_or(DeviceModelError::RequestNotFound.into())
}

pub(crate) fn sync_endpoint_io_state(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    fd: Descriptor,
) -> Result<(), RuntimeError> {
    let descriptor = runtime
        .namespace(owner)?
        .get(fd)
        .map_err(RuntimeError::from)?
        .clone();
    let path = descriptor.name().to_string();
    match descriptor.kind() {
        ObjectKind::Socket => {
            if let Some(socket) = runtime
                .network_sockets
                .iter()
                .find(|socket| socket.path == path && socket.owner == owner)
            {
                let payload = if let Some(packet) = socket.rx_queue.first() {
                    if let Some(iface) = runtime
                        .network_ifaces
                        .iter()
                        .find(|iface| iface.device_path == socket.interface)
                    {
                        network_buffer_payload(iface, packet.buffer_id)?.to_vec()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };
                runtime
                    .io_registry
                    .replace_payload(owner, fd, &payload)
                    .map_err(map_runtime_io_error)?;
                let state = if let Some(iface) = runtime
                    .network_ifaces
                    .iter()
                    .find(|iface| iface.device_path == socket.interface)
                {
                    if socket.rx_queue.is_empty()
                        && network_effective_link_up(iface)
                        && iface.tx_ring.len() < iface.tx_capacity
                    {
                        IoState::Writable
                    } else if socket.rx_queue.is_empty() {
                        IoState::Idle
                    } else if network_effective_link_up(iface) {
                        IoState::ReadWrite
                    } else {
                        IoState::Readable
                    }
                } else if socket.rx_queue.is_empty() {
                    IoState::Idle
                } else {
                    IoState::Readable
                };
                runtime
                    .io_registry
                    .set_state(owner, fd, state)
                    .map_err(map_runtime_io_error)?;
            }
        }
        ObjectKind::Channel => {
            let payload = runtime
                .runtime_channels
                .iter()
                .find(|channel| channel.path == path)
                .and_then(|channel| channel.messages.first().cloned())
                .unwrap_or_default();
            runtime
                .io_registry
                .replace_payload(owner, fd, &payload)
                .map_err(map_runtime_io_error)?;
            runtime
                .io_registry
                .set_state(
                    owner,
                    fd,
                    if payload.is_empty() {
                        IoState::Writable
                    } else {
                        IoState::ReadWrite
                    },
                )
                .map_err(map_runtime_io_error)?;
        }
        ObjectKind::Driver => {
            if let Some(iface) = runtime
                .network_ifaces
                .iter()
                .find(|iface| iface.driver_path == path)
            {
                let payload = iface
                    .tx_ring
                    .first()
                    .map(|buffer_id| {
                        let buffer = iface
                            .buffers
                            .iter()
                            .find(|buffer| buffer.id == *buffer_id)
                            .ok_or(DeviceModelError::RequestNotFound)
                            .expect("network tx buffer must exist");
                        let (src_port, dst_port) = parse_udp_ipv4_frame(&buffer.payload)
                            .map(|(_, _, _, _, src_port, dst_port, _)| (src_port, dst_port))
                            .unwrap_or((0, 0));
                        format!(
                            "net-tx iface={} socket={} bytes={} sport={} dport={} buffer={} queued={} inflight={}\n",
                            iface.device_path,
                            buffer.source_socket,
                            buffer.payload.len(),
                            src_port,
                            dst_port,
                            buffer.id,
                            iface.tx_ring.len(),
                            iface.tx_in_flight.len()
                        )
                        .into_bytes()
                        .into_iter()
                        .chain(buffer.payload.iter().copied())
                        .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                runtime
                    .io_registry
                    .replace_payload(owner, fd, &payload)
                    .map_err(map_runtime_io_error)?;
                let state = if iface.tx_ring.is_empty() && iface.tx_in_flight.is_empty() {
                    IoState::Writable
                } else {
                    IoState::ReadWrite
                };
                runtime
                    .io_registry
                    .set_state(owner, fd, state)
                    .map_err(map_runtime_io_error)?;
                return Ok(());
            }
            if let Some(driver) = runtime
                .device_registry
                .drivers
                .iter()
                .find(|driver| driver.path == path)
            {
                let payload = if let Some(request_id) = driver.in_flight_requests.first().copied() {
                    let request = runtime
                        .device_registry
                        .requests
                        .iter()
                        .find(|request| request.id == request_id)
                        .ok_or(DeviceModelError::RequestNotFound)?;
                    format!(
                        "request:{} kind={:?} device={} bytes={} opcode={:?}",
                        request.id,
                        request.kind,
                        request.device_path,
                        request.payload.len(),
                        request.opcode
                    )
                    .into_bytes()
                } else {
                    Vec::new()
                };
                runtime
                    .io_registry
                    .replace_payload(owner, fd, &payload)
                    .map_err(map_runtime_io_error)?;
                let state =
                    if driver.in_flight_requests.is_empty() && driver.queued_requests.is_empty() {
                        IoState::Writable
                    } else {
                        IoState::ReadWrite
                    };
                runtime
                    .io_registry
                    .set_state(owner, fd, state)
                    .map_err(map_runtime_io_error)?;
            }
        }
        ObjectKind::Device => {
            if let Some(device) = runtime
                .device_registry
                .devices
                .iter()
                .find(|device| device.path == path)
            {
                let payload = if let Some(request_id) = device.completion_queue.first().copied() {
                    let request = runtime
                        .device_registry
                        .requests
                        .iter()
                        .find(|request| request.id == request_id)
                        .ok_or(DeviceModelError::RequestNotFound)?;
                    request.response.clone()
                } else {
                    Vec::new()
                };
                runtime
                    .io_registry
                    .replace_payload(owner, fd, &payload)
                    .map_err(map_runtime_io_error)?;
                let state = if device.completion_queue.is_empty() {
                    IoState::Writable
                } else {
                    IoState::ReadWrite
                };
                runtime
                    .io_registry
                    .set_state(owner, fd, state)
                    .map_err(map_runtime_io_error)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn refresh_and_notify_bindings_for_paths(
    runtime: &mut KernelRuntime,
    paths: &[&str],
) -> Result<(), RuntimeError> {
    let mut bindings = Vec::new();
    for path in paths {
        for binding in runtime.descriptor_bindings_for_path(path)? {
            if !bindings.contains(&binding) {
                bindings.push(binding);
            }
        }
    }
    for (owner, fd) in bindings {
        let _ = sync_endpoint_io_state(runtime, owner, fd);
        let _ = runtime.notify_descriptor_ready(owner, fd);
    }
    Ok(())
}

impl KernelRuntime {
    fn ensure_network_iface_for_device(&mut self, device_path: &str, driver_path: &str) {
        if self
            .network_ifaces
            .iter()
            .any(|iface| iface.device_path == device_path)
        {
            return;
        }
        self.network_ifaces.push(NetworkInterface {
            device_path: device_path.to_string(),
            driver_path: driver_path.to_string(),
            admin_up: true,
            link_up: true,
            promiscuous: false,
            mtu: 1500,
            mac: synthetic_mac_for_path(device_path),
            ipv4_addr: synthetic_ipv4_for_path(device_path),
            ipv4_netmask: [255, 255, 255, 0],
            ipv4_gateway: [10, 0, 0, 1],
            tx_capacity: 128,
            rx_capacity: 128,
            tx_inflight_limit: 64,
            tx_ring: Vec::new(),
            tx_in_flight: Vec::new(),
            rx_ring: Vec::new(),
            free_buffers: Vec::new(),
            buffers: Vec::new(),
            next_buffer_id: 1,
            tx_packets: 0,
            rx_packets: 0,
            tx_completions: 0,
            tx_dropped: 0,
            rx_dropped: 0,
            attached_sockets: Vec::new(),
        });
    }

    fn alloc_network_buffer(
        &mut self,
        iface_index: usize,
        source_socket: String,
        payload: Vec<u8>,
        state: NetworkBufferState,
    ) -> Result<u64, RuntimeError> {
        if let Some(buffer_id) = self.network_ifaces[iface_index].free_buffers.pop() {
            let buffer = self.network_ifaces[iface_index]
                .buffers
                .iter_mut()
                .find(|buffer| buffer.id == buffer_id)
                .ok_or(DeviceModelError::RequestNotFound)?;
            buffer.source_socket = source_socket;
            buffer.payload = payload;
            buffer.state = state;
            return Ok(buffer_id);
        }
        let total_in_use = self.network_ifaces[iface_index].buffers.len();
        let max_buffers = self.network_ifaces[iface_index]
            .tx_capacity
            .saturating_add(self.network_ifaces[iface_index].rx_capacity);
        if total_in_use >= max_buffers {
            return Err(DeviceModelError::QueueFull.into());
        }
        let buffer_id = self.network_ifaces[iface_index].next_buffer_id;
        self.network_ifaces[iface_index].next_buffer_id = self.network_ifaces[iface_index]
            .next_buffer_id
            .saturating_add(1);
        self.network_ifaces[iface_index]
            .buffers
            .push(NetworkBuffer {
                id: buffer_id,
                source_socket,
                payload,
                state,
            });
        Ok(buffer_id)
    }

    fn network_buffer_mut(
        &mut self,
        iface_index: usize,
        buffer_id: u64,
    ) -> Result<&mut NetworkBuffer, RuntimeError> {
        self.network_ifaces[iface_index]
            .buffers
            .iter_mut()
            .find(|buffer| buffer.id == buffer_id)
            .ok_or(DeviceModelError::RequestNotFound.into())
    }

    fn release_network_buffer(
        &mut self,
        iface_index: usize,
        buffer_id: u64,
    ) -> Result<(), RuntimeError> {
        let buffer = self.network_buffer_mut(iface_index, buffer_id)?;
        buffer.source_socket.clear();
        buffer.payload.clear();
        buffer.state = NetworkBufferState::Free;
        if !self.network_ifaces[iface_index]
            .free_buffers
            .contains(&buffer_id)
        {
            self.network_ifaces[iface_index]
                .free_buffers
                .push(buffer_id);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn queue_socket_rx_buffer(
        &mut self,
        iface_index: usize,
        socket_index: usize,
        buffer_id: u64,
        src_ipv4: [u8; 4],
        dst_ipv4: [u8; 4],
        src_port: u16,
        dst_port: u16,
    ) -> Result<(), RuntimeError> {
        if self.network_sockets[socket_index].rx_queue.len()
            >= self.network_sockets[socket_index].rx_queue_limit
        {
            self.network_sockets[socket_index].dropped_packets = self.network_sockets[socket_index]
                .dropped_packets
                .saturating_add(1);
            self.network_ifaces[iface_index].rx_dropped = self.network_ifaces[iface_index]
                .rx_dropped
                .saturating_add(1);
            self.release_network_buffer(iface_index, buffer_id)?;
            return Err(DeviceModelError::QueueFull.into());
        }
        self.network_buffer_mut(iface_index, buffer_id)?.state = NetworkBufferState::SocketQueued;
        self.network_sockets[socket_index]
            .rx_queue
            .push(SocketRxPacket {
                buffer_id,
                src_ipv4,
                dst_ipv4,
                src_port,
                dst_port,
            });
        Ok(())
    }

    pub(crate) fn ensure_endpoint_registered_for_node(
        &mut self,
        path: &str,
        kind: ObjectKind,
        capability: CapabilityId,
    ) -> Result<(), RuntimeError> {
        let capability = self.capabilities.get(capability)?;
        match kind {
            ObjectKind::Driver => {
                if self
                    .device_registry
                    .drivers
                    .iter()
                    .any(|driver| driver.path == path)
                {
                    return Ok(());
                }
                self.device_registry.drivers.push(DriverEndpoint {
                    path: path.to_string(),
                    owner: capability.owner(),
                    capability: capability.id(),
                    state: DriverState::Registered,
                    bound_devices: Vec::new(),
                    queued_requests: Vec::new(),
                    in_flight_requests: Vec::new(),
                    completed_requests: 0,
                });
            }
            ObjectKind::Device => {
                if self
                    .device_registry
                    .devices
                    .iter()
                    .any(|device| device.path == path)
                {
                    return Ok(());
                }
                let class = device_class_for_path(path);
                self.device_registry.devices.push(DeviceEndpoint {
                    path: path.to_string(),
                    owner: capability.owner(),
                    capability: capability.id(),
                    class,
                    state: DeviceState::Registered,
                    driver: None,
                    queue_capacity: default_queue_capacity(class),
                    pending_requests: Vec::new(),
                    completion_queue: Vec::new(),
                    graphics_control_reserve_armed: class == DeviceClass::Graphics,
                    graphics_presented_frames: 0,
                    graphics_last_presented_frame: Vec::new(),
                    submitted_requests: 0,
                    completed_requests: 0,
                    total_latency_ticks: 0,
                    max_latency_ticks: 0,
                    total_queue_wait_ticks: 0,
                    max_queue_wait_ticks: 0,
                    link_up: true,
                    block_size: default_block_size(class),
                    capacity_bytes: default_capacity_bytes(class),
                });
            }
            _ => {}
        }
        Ok(())
    }

    pub fn attach_socket_to_network_interface(
        &mut self,
        socket_path: &str,
        owner: ProcessId,
        device_path: &str,
    ) -> Result<(), RuntimeError> {
        let iface_index = self
            .network_ifaces
            .iter()
            .position(|iface| iface.device_path == device_path)
            .ok_or(DeviceModelError::InvalidDevice)?;
        let iface_name = self.network_ifaces[iface_index].device_path.clone();
        if !self
            .network_sockets
            .iter()
            .any(|socket| socket.path == socket_path && socket.owner == owner)
        {
            self.network_sockets.push(NetworkSocket {
                path: socket_path.to_string(),
                owner,
                interface: iface_name.clone(),
                local_ipv4: self.network_ifaces[iface_index].ipv4_addr,
                remote_ipv4: self.network_ifaces[iface_index].ipv4_gateway,
                local_port: 0,
                remote_port: 0,
                rx_queue: Vec::new(),
                rx_queue_limit: self.network_ifaces[iface_index].rx_capacity,
                connected: false,
                tx_packets: 0,
                rx_packets: 0,
                dropped_packets: 0,
            });
        }
        if !self.network_ifaces[iface_index]
            .attached_sockets
            .iter()
            .any(|path| path == socket_path)
        {
            self.network_ifaces[iface_index]
                .attached_sockets
                .push(socket_path.to_string());
        }
        for (binding_owner, binding_fd) in self.descriptor_bindings_for_path(socket_path)? {
            let _ = sync_endpoint_io_state(self, binding_owner, binding_fd);
            let _ = self.notify_descriptor_ready(binding_owner, binding_fd);
        }
        Ok(())
    }

    pub fn configure_network_interface_ipv4(
        &mut self,
        device_path: &str,
        addr: [u8; 4],
        netmask: [u8; 4],
        gateway: [u8; 4],
    ) -> Result<(), RuntimeError> {
        let iface = self
            .network_ifaces
            .iter_mut()
            .find(|iface| iface.device_path == device_path)
            .ok_or(DeviceModelError::InvalidDevice)?;
        iface.ipv4_addr = addr;
        iface.ipv4_netmask = netmask;
        iface.ipv4_gateway = gateway;
        for socket in &mut self.network_sockets {
            if socket.interface == device_path {
                socket.local_ipv4 = addr;
                if socket.remote_ipv4 == [0, 0, 0, 0] {
                    socket.remote_ipv4 = gateway;
                }
            }
        }
        event_queue_runtime::emit_network_events(
            self,
            path_inode(self, device_path)?,
            None,
            NetworkEventKind::LinkChanged,
        )?;
        Ok(())
    }

    pub fn set_network_interface_link_state(
        &mut self,
        device_path: &str,
        link_up: bool,
    ) -> Result<(), RuntimeError> {
        let iface = self
            .network_ifaces
            .iter_mut()
            .find(|iface| iface.device_path == device_path)
            .ok_or(DeviceModelError::InvalidDevice)?;
        if iface.link_up == link_up {
            return Ok(());
        }
        iface.link_up = link_up;
        let attached_sockets = iface.attached_sockets.clone();
        for socket_path in attached_sockets {
            for (binding_owner, binding_fd) in self.descriptor_bindings_for_path(&socket_path)? {
                let _ = sync_endpoint_io_state(self, binding_owner, binding_fd);
                let _ = self.notify_descriptor_ready(binding_owner, binding_fd);
            }
        }
        for (binding_owner, binding_fd) in self.descriptor_bindings_for_path(device_path)? {
            let _ = sync_endpoint_io_state(self, binding_owner, binding_fd);
            let _ = self.notify_descriptor_ready(binding_owner, binding_fd);
        }
        event_queue_runtime::emit_network_events(
            self,
            path_inode(self, device_path)?,
            None,
            NetworkEventKind::LinkChanged,
        )?;
        Ok(())
    }

    pub fn set_network_interface_mtu(
        &mut self,
        device_path: &str,
        mtu: usize,
    ) -> Result<(), RuntimeError> {
        if mtu < 68 {
            return Err(DeviceModelError::InvalidDevice.into());
        }
        let iface = self
            .network_ifaces
            .iter_mut()
            .find(|iface| iface.device_path == device_path)
            .ok_or(DeviceModelError::InvalidDevice)?;
        iface.mtu = mtu;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn configure_network_interface_admin(
        &mut self,
        device_path: &str,
        admin_up: bool,
        promiscuous: bool,
        mtu: usize,
        tx_capacity: usize,
        rx_capacity: usize,
        tx_inflight_limit: usize,
    ) -> Result<(), RuntimeError> {
        if mtu < 68 || tx_capacity == 0 || rx_capacity == 0 || tx_inflight_limit == 0 {
            return Err(DeviceModelError::InvalidDevice.into());
        }
        let iface_index = self
            .network_ifaces
            .iter()
            .position(|iface| iface.device_path == device_path)
            .ok_or(DeviceModelError::InvalidDevice)?;
        let in_use_buffers = self.network_ifaces[iface_index]
            .buffers
            .len()
            .saturating_sub(self.network_ifaces[iface_index].free_buffers.len());
        if self.network_ifaces[iface_index].tx_ring.len() > tx_capacity
            || self.network_ifaces[iface_index].rx_ring.len() > rx_capacity
            || self.network_ifaces[iface_index].tx_in_flight.len() > tx_inflight_limit
            || in_use_buffers > tx_capacity.saturating_add(rx_capacity)
        {
            return Err(DeviceModelError::QueueFull.into());
        }
        self.network_ifaces[iface_index].admin_up = admin_up;
        self.network_ifaces[iface_index].promiscuous = promiscuous;
        self.network_ifaces[iface_index].mtu = mtu;
        self.network_ifaces[iface_index].tx_capacity = tx_capacity;
        self.network_ifaces[iface_index].rx_capacity = rx_capacity;
        self.network_ifaces[iface_index].tx_inflight_limit = tx_inflight_limit;
        for socket in &mut self.network_sockets {
            if socket.interface == device_path {
                socket.rx_queue_limit = rx_capacity;
            }
        }
        Ok(())
    }

    pub fn bind_udp_socket(
        &mut self,
        socket_path: &str,
        owner: ProcessId,
        device_path: &str,
        local_port: u16,
        remote_ipv4: [u8; 4],
        remote_port: u16,
    ) -> Result<(), RuntimeError> {
        self.attach_socket_to_network_interface(socket_path, owner, device_path)?;
        let socket = self
            .network_sockets
            .iter_mut()
            .find(|socket| socket.path == socket_path && socket.owner == owner)
            .ok_or(DeviceModelError::InvalidDevice)?;
        socket.local_port = local_port;
        socket.remote_port = remote_port;
        socket.remote_ipv4 = remote_ipv4;
        socket.connected = remote_port != 0 || remote_ipv4 != [0, 0, 0, 0];
        Ok(())
    }

    pub fn connect_udp_socket(
        &mut self,
        socket_path: &str,
        owner: ProcessId,
        remote_ipv4: [u8; 4],
        remote_port: u16,
    ) -> Result<(), RuntimeError> {
        let socket = self
            .network_sockets
            .iter_mut()
            .find(|socket| socket.path == socket_path && socket.owner == owner)
            .ok_or(DeviceModelError::InvalidDevice)?;
        socket.remote_ipv4 = remote_ipv4;
        socket.remote_port = remote_port;
        socket.connected = remote_port != 0 || remote_ipv4 != [0, 0, 0, 0];
        Ok(())
    }

    pub fn send_udp_socket_to(
        &mut self,
        socket_path: &str,
        owner: ProcessId,
        remote_ipv4: [u8; 4],
        remote_port: u16,
        bytes: &[u8],
    ) -> Result<usize, RuntimeError> {
        let socket_index = self
            .network_sockets
            .iter()
            .position(|socket| socket.path == socket_path && socket.owner == owner)
            .ok_or(DeviceModelError::InvalidDevice)?;
        let remote_ipv4 = if remote_ipv4 == [0, 0, 0, 0] {
            self.network_sockets[socket_index].remote_ipv4
        } else {
            remote_ipv4
        };
        let remote_port = if remote_port == 0 {
            self.network_sockets[socket_index].remote_port
        } else {
            remote_port
        };
        let local_port = self.network_sockets[socket_index].local_port;
        if local_port == 0 || remote_port == 0 {
            return Err(DeviceModelError::NotBound.into());
        }
        let interface = self.network_sockets[socket_index].interface.clone();
        let iface_index = self
            .network_ifaces
            .iter()
            .position(|iface| iface.device_path == interface)
            .ok_or(DeviceModelError::InvalidDevice)?;
        if !network_effective_link_up(&self.network_ifaces[iface_index]) {
            return Err(DeviceModelError::NotBound.into());
        }
        if self.network_ifaces[iface_index].tx_ring.len()
            >= self.network_ifaces[iface_index].tx_capacity
            || self.network_ifaces[iface_index].tx_in_flight.len()
                >= self.network_ifaces[iface_index].tx_inflight_limit
        {
            self.network_ifaces[iface_index].tx_dropped = self.network_ifaces[iface_index]
                .tx_dropped
                .saturating_add(1);
            return Err(DeviceModelError::QueueFull.into());
        }
        let frame = build_udp_ipv4_frame(
            self.network_ifaces[iface_index].mac,
            [0xff; 6],
            self.network_sockets[socket_index].local_ipv4,
            remote_ipv4,
            local_port,
            remote_port,
            bytes,
        );
        if frame.len().saturating_sub(14) > self.network_ifaces[iface_index].mtu {
            return Err(DeviceModelError::PacketTooLarge.into());
        }
        let buffer_id = self.alloc_network_buffer(
            iface_index,
            socket_path.to_string(),
            frame.clone(),
            NetworkBufferState::TxQueued,
        )?;
        self.network_ifaces[iface_index].tx_ring.push(buffer_id);
        self.network_ifaces[iface_index].tx_packets = self.network_ifaces[iface_index]
            .tx_packets
            .saturating_add(1);
        self.network_sockets[socket_index].tx_packets = self.network_sockets[socket_index]
            .tx_packets
            .saturating_add(1);
        let driver_path = self.network_ifaces[iface_index].driver_path.clone();
        let tx_text = format!(
            "net-tx iface={} socket={} bytes={} sport={} dport={} buffer={} queued={} inflight={}\n",
            interface,
            socket_path,
            bytes.len(),
            local_port,
            remote_port,
            buffer_id,
            self.network_ifaces[iface_index].tx_ring.len(),
            self.network_ifaces[iface_index].tx_in_flight.len()
        )
        .into_bytes()
        .into_iter()
        .chain(frame.iter().copied())
        .collect::<Vec<_>>();
        for (binding_owner, binding_fd) in self.descriptor_bindings_for_path(&driver_path)? {
            self.io_registry
                .replace_payload(binding_owner, binding_fd, &tx_text)
                .map_err(map_runtime_io_error)?;
            self.io_registry
                .set_state(binding_owner, binding_fd, IoState::ReadWrite)
                .map_err(map_runtime_io_error)?;
            let _ = self.notify_descriptor_ready(binding_owner, binding_fd);
        }
        Ok(bytes.len())
    }

    pub fn recv_udp_socket_from(
        &mut self,
        socket_path: &str,
        owner: ProcessId,
        max_len: usize,
    ) -> Result<(Vec<u8>, [u8; 4], u16), RuntimeError> {
        let socket_index = self
            .network_sockets
            .iter()
            .position(|socket| socket.path == socket_path && socket.owner == owner)
            .ok_or(DeviceModelError::InvalidDevice)?;
        if self.network_sockets[socket_index].rx_queue.is_empty() {
            return Err(DeviceModelError::QueueEmpty.into());
        }
        let packet = self.network_sockets[socket_index].rx_queue.remove(0);
        let iface_index = self
            .network_ifaces
            .iter()
            .position(|iface| iface.device_path == self.network_sockets[socket_index].interface)
            .ok_or(DeviceModelError::InvalidDevice)?;
        let mut payload =
            network_buffer_payload(&self.network_ifaces[iface_index], packet.buffer_id)?.to_vec();
        if payload.len() > max_len {
            payload.truncate(max_len);
        }
        self.release_network_buffer(iface_index, packet.buffer_id)?;
        self.network_sockets[socket_index].rx_packets = self.network_sockets[socket_index]
            .rx_packets
            .saturating_add(1);
        Ok((payload, packet.src_ipv4, packet.src_port))
    }

    pub fn complete_network_tx(
        &mut self,
        driver_path: &str,
        completions: usize,
    ) -> Result<usize, RuntimeError> {
        let iface_index = self
            .network_ifaces
            .iter()
            .position(|iface| iface.driver_path == driver_path)
            .ok_or(DeviceModelError::InvalidDriver)?;
        let interface_inode = path_inode(self, &self.network_ifaces[iface_index].device_path)?;
        let to_complete = completions.min(self.network_ifaces[iface_index].tx_in_flight.len());
        for _ in 0..to_complete {
            let buffer_id = self.network_ifaces[iface_index].tx_in_flight.remove(0);
            self.release_network_buffer(iface_index, buffer_id)?;
            self.network_ifaces[iface_index].tx_completions = self.network_ifaces[iface_index]
                .tx_completions
                .saturating_add(1);
        }
        if self.network_ifaces[iface_index].tx_ring.is_empty()
            && self.network_ifaces[iface_index].tx_in_flight.is_empty()
        {
            event_queue_runtime::emit_network_events(
                self,
                interface_inode,
                None,
                NetworkEventKind::TxDrained,
            )?;
        }
        Ok(to_complete)
    }

    pub fn network_interface_info(
        &self,
        device_path: &str,
    ) -> Result<NetworkInterfaceInfo, RuntimeError> {
        self.network_ifaces
            .iter()
            .find(|iface| iface.device_path == device_path)
            .map(NetworkInterface::info)
            .ok_or(DeviceModelError::InvalidDevice.into())
    }

    pub fn network_socket_info(
        &self,
        socket_path: &str,
    ) -> Result<NetworkSocketInfo, RuntimeError> {
        self.network_sockets
            .iter()
            .find(|socket| socket.path == socket_path)
            .map(NetworkSocket::info)
            .ok_or(DeviceModelError::InvalidDevice.into())
    }

    pub(crate) fn retire_endpoint_for_path(&mut self, path: &str) {
        if let Some(socket_index) = self
            .network_sockets
            .iter()
            .position(|socket| socket.path == path)
            && let Some(iface_index) = self
                .network_ifaces
                .iter()
                .position(|iface| iface.device_path == self.network_sockets[socket_index].interface)
        {
            let pending = self.network_sockets[socket_index]
                .rx_queue
                .iter()
                .map(|packet| packet.buffer_id)
                .collect::<Vec<_>>();
            for buffer_id in pending {
                let _ = self.release_network_buffer(iface_index, buffer_id);
            }
        }
        self.network_sockets.retain(|socket| socket.path != path);
        self.network_ifaces
            .retain(|iface| iface.device_path != path && iface.driver_path != path);
        for iface in &mut self.network_ifaces {
            iface.attached_sockets.retain(|socket| socket != path);
            if iface.driver_path == path {
                iface.link_up = false;
            }
        }
        if let Some(index) = self
            .device_registry
            .devices
            .iter()
            .position(|device| device.path == path)
        {
            let device = self.device_registry.devices.remove(index);
            if let Some(driver_path) = device.driver
                && let Some(driver) = self
                    .device_registry
                    .drivers
                    .iter_mut()
                    .find(|driver| driver.path == driver_path)
            {
                driver.bound_devices.retain(|candidate| candidate != path);
                let device_requests = self
                    .device_registry
                    .requests
                    .iter()
                    .filter(|request| request.device_path == path)
                    .map(|request| request.id)
                    .collect::<Vec<_>>();
                driver
                    .queued_requests
                    .retain(|request_id| !device_requests.contains(request_id));
                driver
                    .in_flight_requests
                    .retain(|request_id| !device_requests.contains(request_id));
            }
            for request in &mut self.device_registry.requests {
                if request.device_path == path && request.state != DeviceRequestState::Completed {
                    request.state = DeviceRequestState::Canceled;
                    request.completed_tick = Some(self.current_tick);
                }
            }
        }
        if let Some(index) = self
            .device_registry
            .drivers
            .iter()
            .position(|driver| driver.path == path)
        {
            let driver = self.device_registry.drivers.remove(index);
            for device_path in driver.bound_devices {
                if let Some(device) = self
                    .device_registry
                    .devices
                    .iter_mut()
                    .find(|device| device.path == device_path)
                {
                    device.driver = None;
                    device.state = DeviceState::Registered;
                }
            }
            for request_id in driver
                .queued_requests
                .into_iter()
                .chain(driver.in_flight_requests)
            {
                if let Some(request) = self
                    .device_registry
                    .requests
                    .iter_mut()
                    .find(|request| request.id == request_id)
                {
                    request.state = DeviceRequestState::Canceled;
                    request.completed_tick = Some(self.current_tick);
                }
            }
        }
    }

    pub(crate) fn rename_endpoint_path(&mut self, from: &str, to: &str) {
        for socket in &mut self.network_sockets {
            if socket.path == from {
                socket.path = to.to_string();
            }
            if socket.interface == from {
                socket.interface = to.to_string();
            }
        }
        for iface in &mut self.network_ifaces {
            if iface.device_path == from {
                iface.device_path = to.to_string();
            }
            if iface.driver_path == from {
                iface.driver_path = to.to_string();
            }
            for socket in &mut iface.attached_sockets {
                if socket == from {
                    *socket = to.to_string();
                }
            }
        }
        if let Some(driver) = self
            .device_registry
            .drivers
            .iter_mut()
            .find(|driver| driver.path == from)
        {
            driver.path = to.to_string();
            for device in &mut self.device_registry.devices {
                if device.driver.as_deref() == Some(from) {
                    device.driver = Some(to.to_string());
                }
            }
            for request in &mut self.device_registry.requests {
                if request.driver_path == from {
                    request.driver_path = to.to_string();
                }
            }
        }
        if let Some(device) = self
            .device_registry
            .devices
            .iter_mut()
            .find(|device| device.path == from)
        {
            device.path = to.to_string();
            for driver in &mut self.device_registry.drivers {
                for bound in &mut driver.bound_devices {
                    if bound == from {
                        *bound = to.to_string();
                    }
                }
            }
            for request in &mut self.device_registry.requests {
                if request.device_path == from {
                    request.device_path = to.to_string();
                }
            }
        }
    }

    pub fn bind_device_to_driver(
        &mut self,
        device_path: &str,
        driver_path: &str,
    ) -> Result<(), RuntimeError> {
        let device_info = self.device_registry.device_info(device_path)?;
        let driver_info = self.device_registry.driver_info(driver_path)?;
        if device_info.class == DeviceClass::Graphics
            && (device_info.state == DeviceState::Retired
                || driver_info.state == DriverState::Retired)
        {
            return Err(DeviceModelError::InvalidDriver.into());
        }
        if device_info.class == DeviceClass::Graphics {
            let device = self
                .device_registry
                .devices
                .iter()
                .find(|device| device.path == device_path)
                .ok_or(DeviceModelError::InvalidDevice)?;
            if !device.pending_requests.is_empty() || !device.completion_queue.is_empty() {
                return Err(DeviceModelError::InvalidRequestState.into());
            }
        }
        if let Some(current_driver_path) = self
            .device_registry
            .devices
            .iter()
            .find(|device| device.path == device_path)
            .and_then(|device| device.driver.clone())
        {
            if current_driver_path == driver_path {
                return Ok(());
            }
            let current_driver = driver_mut(&mut self.device_registry, &current_driver_path)?;
            current_driver
                .bound_devices
                .retain(|bound| bound != device_path);
            if current_driver.bound_devices.is_empty()
                && current_driver.state != DriverState::Retired
            {
                current_driver.state = DriverState::Registered;
            }
        }
        let _ = self.device_registry.device_info(device_path)?;
        let _ = self.device_registry.driver_info(driver_path)?;
        {
            let driver = driver_mut(&mut self.device_registry, driver_path)?;
            if !driver
                .bound_devices
                .iter()
                .any(|bound| bound == device_path)
            {
                driver.bound_devices.push(device_path.to_string());
            }
            driver.state = DriverState::Active;
        }
        {
            let device = device_mut(&mut self.device_registry, device_path)?;
            device.driver = Some(driver_path.to_string());
            device.state = DeviceState::Bound;
        }
        if self
            .device_registry
            .devices
            .iter()
            .find(|device| device.path == device_path)
            .is_some_and(|device| device.class == DeviceClass::Network)
        {
            self.ensure_network_iface_for_device(device_path, driver_path);
        }
        if let Ok(fds) = self.descriptor_bindings_for_path(device_path) {
            for (owner, fd) in fds {
                let _ = sync_endpoint_io_state(self, owner, fd);
            }
        }
        if let Ok(fds) = self.descriptor_bindings_for_path(driver_path) {
            for (owner, fd) in fds {
                let _ = sync_endpoint_io_state(self, owner, fd);
            }
        }
        Ok(())
    }

    pub fn unbind_device_driver(&mut self, device_path: &str) -> Result<(), RuntimeError> {
        let device_info = self.device_registry.device_info(device_path)?;
        if device_info.class == DeviceClass::Graphics && device_info.state == DeviceState::Retired {
            return Err(DeviceModelError::InvalidDevice.into());
        }
        let driver_path = self
            .device_registry
            .devices
            .iter()
            .find(|device| device.path == device_path)
            .and_then(|device| device.driver.clone())
            .ok_or(DeviceModelError::NotBound)?;
        if device_info.class == DeviceClass::Graphics {
            let device = self
                .device_registry
                .devices
                .iter()
                .find(|device| device.path == device_path)
                .ok_or(DeviceModelError::InvalidDevice)?;
            if !device.pending_requests.is_empty() || !device.completion_queue.is_empty() {
                return Err(DeviceModelError::InvalidRequestState.into());
            }
        }
        {
            let driver = driver_mut(&mut self.device_registry, &driver_path)?;
            driver.bound_devices.retain(|bound| bound != device_path);
            if driver.bound_devices.is_empty() && driver.state != DriverState::Retired {
                driver.state = DriverState::Registered;
            }
        }
        {
            let device = device_mut(&mut self.device_registry, device_path)?;
            device.driver = None;
            if device.state != DeviceState::Retired {
                device.state = DeviceState::Registered;
            }
        }
        if let Ok(fds) = self.descriptor_bindings_for_path(device_path) {
            for (owner, fd) in fds {
                let _ = sync_endpoint_io_state(self, owner, fd);
            }
        }
        if let Ok(fds) = self.descriptor_bindings_for_path(&driver_path) {
            for (owner, fd) in fds {
                let _ = sync_endpoint_io_state(self, owner, fd);
            }
        }
        Ok(())
    }

    pub fn configure_device_geometry(
        &mut self,
        device_path: &str,
        block_size: u32,
        capacity_bytes: u64,
    ) -> Result<(), RuntimeError> {
        let device = device_mut(&mut self.device_registry, device_path)?;
        if device.class != DeviceClass::Storage || block_size == 0 || capacity_bytes == 0 {
            return Err(DeviceModelError::InvalidDevice.into());
        }
        device.block_size = block_size;
        device.capacity_bytes = capacity_bytes;
        Ok(())
    }

    pub fn configure_device_queue(
        &mut self,
        device_path: &str,
        queue_capacity: usize,
    ) -> Result<(), RuntimeError> {
        if queue_capacity == 0 {
            return Err(DeviceModelError::InvalidDevice.into());
        }
        let device = device_mut(&mut self.device_registry, device_path)?;
        if device.pending_requests.len() > queue_capacity {
            return Err(DeviceModelError::QueueFull.into());
        }
        device.queue_capacity = queue_capacity;
        Ok(())
    }

    pub fn cancel_graphics_requests_for_issuer(
        &mut self,
        device_path: &str,
        issuer: ProcessId,
    ) -> Result<usize, RuntimeError> {
        let Some(device_inode) = graphics_event_device_inode(self, device_path) else {
            return Ok(0);
        };
        let request_ids = self
            .device_registry
            .requests
            .iter()
            .filter(|request| {
                request.device_path == device_path
                    && request.issuer == issuer
                    && matches!(
                        request.state,
                        DeviceRequestState::Queued | DeviceRequestState::InFlight
                    )
            })
            .map(|request| request.id)
            .collect::<Vec<_>>();
        let canceled_control = self.device_registry.requests.iter().any(|request| {
            request.device_path == device_path
                && request.issuer == issuer
                && request.kind == DeviceRequestKind::Control
                && matches!(
                    request.state,
                    DeviceRequestState::Queued | DeviceRequestState::InFlight
                )
        });
        if request_ids.is_empty() {
            return Ok(0);
        }
        for request_id in &request_ids {
            if let Some(request) = self
                .device_registry
                .requests
                .iter_mut()
                .find(|request| request.id == *request_id)
            {
                request.state = DeviceRequestState::Canceled;
                request.completed_tick = Some(self.current_tick);
            }
        }
        if let Ok(device) = device_mut(&mut self.device_registry, device_path) {
            device
                .pending_requests
                .retain(|request_id| !request_ids.contains(request_id));
            if device.class == DeviceClass::Graphics && canceled_control {
                device.graphics_control_reserve_armed = false;
            }
            if device.class == DeviceClass::Graphics && device.pending_requests.is_empty() {
                device.graphics_control_reserve_armed = true;
            }
        }
        if let Some(driver_path) = self
            .device_registry
            .devices
            .iter()
            .find(|device| device.path == device_path)
            .and_then(|device| device.driver.clone())
            && let Ok(driver) = driver_mut(&mut self.device_registry, &driver_path)
        {
            driver
                .queued_requests
                .retain(|request_id| !request_ids.contains(request_id));
            driver
                .in_flight_requests
                .retain(|request_id| !request_ids.contains(request_id));
        }
        for request_id in &request_ids {
            let _ = event_queue_runtime::emit_graphics_events(
                self,
                device_inode,
                *request_id,
                GraphicsEventKind::Canceled,
            );
        }
        let queue_drained = self
            .device_registry
            .devices
            .iter()
            .find(|device| device.path == device_path)
            .is_some_and(|device| device.pending_requests.is_empty());
        if queue_drained {
            let _ = event_queue_runtime::emit_graphics_events(
                self,
                device_inode,
                0,
                GraphicsEventKind::Drained,
            );
        }
        for (binding_owner, binding_fd) in self.descriptor_bindings_for_path(device_path)? {
            let _ = sync_endpoint_io_state(self, binding_owner, binding_fd);
            let _ = self.notify_descriptor_ready(binding_owner, binding_fd);
        }
        Ok(request_ids.len())
    }

    pub fn device_info_by_path(&self, path: &str) -> Result<DeviceInfo, RuntimeError> {
        self.device_registry.device_info(path).map_err(Into::into)
    }

    pub fn driver_info_by_path(&self, path: &str) -> Result<DriverInfo, RuntimeError> {
        self.device_registry.driver_info(path).map_err(Into::into)
    }

    pub fn device_request_info(&self, request_id: u64) -> Result<DeviceRequestInfo, RuntimeError> {
        self.device_registry
            .request_info(request_id)
            .map_err(Into::into)
    }

    pub fn create_graphics_buffer(
        &mut self,
        owner: ProcessId,
        length: usize,
    ) -> Result<u64, RuntimeError> {
        if length == 0 {
            return Err(DeviceModelError::InvalidRequestState.into());
        }
        let buffer_id = self.device_registry.next_gpu_buffer_id;
        self.device_registry.next_gpu_buffer_id =
            self.device_registry.next_gpu_buffer_id.saturating_add(1);
        self.device_registry.gpu_buffers.push(GpuBufferObject {
            id: buffer_id,
            owner,
            length,
            used_len: 0,
            busy: false,
            bytes: vec![0; length],
        });
        Ok(buffer_id)
    }

    pub fn write_graphics_buffer(
        &mut self,
        owner: ProcessId,
        buffer_id: u64,
        offset: usize,
        bytes: &[u8],
    ) -> Result<usize, RuntimeError> {
        let Some(buffer) = self
            .device_registry
            .gpu_buffers
            .iter_mut()
            .find(|buffer| buffer.id == buffer_id)
        else {
            return Err(DeviceModelError::RequestNotFound.into());
        };
        if buffer.owner != owner {
            return Err(DeviceModelError::InvalidRequestState.into());
        }
        if buffer.busy {
            return Err(DeviceModelError::InvalidRequestState.into());
        }
        let end = offset
            .checked_add(bytes.len())
            .ok_or(DeviceModelError::InvalidRequestState)?;
        if end > buffer.length {
            return Err(DeviceModelError::QueueFull.into());
        }
        buffer.bytes[offset..end].copy_from_slice(bytes);
        buffer.used_len = buffer.used_len.max(end);
        Ok(bytes.len())
    }

    pub fn graphics_buffer_info(&self, buffer_id: u64) -> Result<GpuBufferInfo, RuntimeError> {
        self.device_registry
            .gpu_buffer_info(buffer_id)
            .map_err(Into::into)
    }

    pub fn graphics_scanout_info(&self, device_path: &str) -> Result<GpuScanoutInfo, RuntimeError> {
        let device = self
            .device_registry
            .devices
            .iter()
            .find(|device| device.path == device_path)
            .ok_or(DeviceModelError::InvalidDevice)?;
        if device.class != DeviceClass::Graphics {
            return Err(DeviceModelError::InvalidDevice.into());
        }
        Ok(GpuScanoutInfo {
            device_path: device.path.clone(),
            presented_frames: device.graphics_presented_frames,
            last_frame_len: device.graphics_last_presented_frame.len(),
        })
    }

    pub fn graphics_binding_evidence(
        &mut self,
        device_path: &str,
    ) -> Result<Option<platform_hal::GpuBindingEvidence>, RuntimeError> {
        let device = self
            .device_registry
            .devices
            .iter()
            .find(|device| device.path == device_path)
            .ok_or(DeviceModelError::InvalidDevice)?;
        if device.class != DeviceClass::Graphics {
            return Err(DeviceModelError::InvalidDevice.into());
        }
        let Some(provider) = self.hardware.as_mut() else {
            return Ok(None);
        };
        provider
            .primary_gpu_binding_evidence()
            .map_err(|_| DeviceModelError::InvalidDevice.into())
    }

    pub fn graphics_vbios_window(
        &mut self,
        device_path: &str,
    ) -> Result<Option<platform_hal::GpuVbiosWindowEvidence>, RuntimeError> {
        let device = self
            .device_registry
            .devices
            .iter()
            .find(|device| device.path == device_path)
            .ok_or(DeviceModelError::InvalidDevice)?;
        if device.class != DeviceClass::Graphics {
            return Err(DeviceModelError::InvalidDevice.into());
        }
        let Some(provider) = self.hardware.as_mut() else {
            return Ok(None);
        };
        provider
            .primary_gpu_vbios_window()
            .map_err(|_| DeviceModelError::InvalidDevice.into())
    }

    pub fn read_graphics_vbios(
        &mut self,
        device_path: &str,
        max_len: usize,
    ) -> Result<Vec<u8>, RuntimeError> {
        let device = self
            .device_registry
            .devices
            .iter()
            .find(|device| device.path == device_path)
            .ok_or(DeviceModelError::InvalidDevice)?;
        if device.class != DeviceClass::Graphics {
            return Err(DeviceModelError::InvalidDevice.into());
        }
        let Some(provider) = self.hardware.as_mut() else {
            return Err(DeviceModelError::InvalidDevice.into());
        };
        provider
            .primary_gpu_vbios_bytes(max_len)
            .map_err(|_| DeviceModelError::InvalidDevice.into())
    }

    pub fn graphics_vbios_image_evidence(
        &mut self,
        device_path: &str,
    ) -> Result<Option<platform_hal::GpuVbiosImageEvidence>, RuntimeError> {
        let device = self
            .device_registry
            .devices
            .iter()
            .find(|device| device.path == device_path)
            .ok_or(DeviceModelError::InvalidDevice)?;
        if device.class != DeviceClass::Graphics {
            return Err(DeviceModelError::InvalidDevice.into());
        }
        let Some(provider) = self.hardware.as_mut() else {
            return Ok(None);
        };
        provider
            .primary_gpu_vbios_image_evidence()
            .map_err(|_| DeviceModelError::InvalidDevice.into())
    }

    pub fn graphics_gsp_evidence(
        &mut self,
        device_path: &str,
    ) -> Result<Option<platform_hal::GpuGspEvidence>, RuntimeError> {
        let device = self
            .device_registry
            .devices
            .iter()
            .find(|device| device.path == device_path)
            .ok_or(DeviceModelError::InvalidDevice)?;
        if device.class != DeviceClass::Graphics {
            return Err(DeviceModelError::InvalidDevice.into());
        }
        let Some(provider) = self.hardware.as_mut() else {
            return Ok(None);
        };
        provider
            .primary_gpu_gsp_evidence()
            .map_err(|_| DeviceModelError::InvalidDevice.into())
    }

    pub fn graphics_interrupt_evidence(
        &mut self,
        device_path: &str,
    ) -> Result<Option<platform_hal::GpuInterruptEvidence>, RuntimeError> {
        let device = self
            .device_registry
            .devices
            .iter()
            .find(|device| device.path == device_path)
            .ok_or(DeviceModelError::InvalidDevice)?;
        if device.class != DeviceClass::Graphics {
            return Err(DeviceModelError::InvalidDevice.into());
        }
        let Some(provider) = self.hardware.as_mut() else {
            return Ok(None);
        };
        provider
            .primary_gpu_interrupt_evidence()
            .map_err(|_| DeviceModelError::InvalidDevice.into())
    }

    pub fn graphics_display_evidence(
        &mut self,
        device_path: &str,
    ) -> Result<Option<platform_hal::GpuDisplayEvidence>, RuntimeError> {
        let device = self
            .device_registry
            .devices
            .iter()
            .find(|device| device.path == device_path)
            .ok_or(DeviceModelError::InvalidDevice)?;
        if device.class != DeviceClass::Graphics {
            return Err(DeviceModelError::InvalidDevice.into());
        }
        let Some(provider) = self.hardware.as_mut() else {
            return Ok(None);
        };
        provider
            .primary_gpu_display_evidence()
            .map_err(|_| DeviceModelError::InvalidDevice.into())
    }

    pub fn graphics_power_evidence(
        &mut self,
        device_path: &str,
    ) -> Result<Option<platform_hal::GpuPowerEvidence>, RuntimeError> {
        let device = self
            .device_registry
            .devices
            .iter()
            .find(|device| device.path == device_path)
            .ok_or(DeviceModelError::InvalidDevice)?;
        if device.class != DeviceClass::Graphics {
            return Err(DeviceModelError::InvalidDevice.into());
        }
        let Some(provider) = self.hardware.as_mut() else {
            return Ok(None);
        };
        provider
            .primary_gpu_power_evidence()
            .map_err(|_| DeviceModelError::InvalidDevice.into())
    }

    pub fn graphics_set_power_state(
        &mut self,
        device_path: &str,
        pstate: u32,
    ) -> Result<(), RuntimeError> {
        let device = self
            .device_registry
            .devices
            .iter()
            .find(|device| device.path == device_path)
            .ok_or(DeviceModelError::InvalidDevice)?;
        if device.class != DeviceClass::Graphics {
            return Err(DeviceModelError::InvalidDevice.into());
        }
        let Some(provider) = self.hardware.as_mut() else {
            return Err(DeviceModelError::InvalidDevice.into());
        };
        provider
            .set_primary_gpu_power_state(pstate)
            .map_err(|_| DeviceModelError::InvalidDevice.into())
    }

    pub fn graphics_media_evidence(
        &mut self,
        device_path: &str,
    ) -> Result<Option<platform_hal::GpuMediaEvidence>, RuntimeError> {
        let device = self
            .device_registry
            .devices
            .iter()
            .find(|device| device.path == device_path)
            .ok_or(DeviceModelError::InvalidDevice)?;
        if device.class != DeviceClass::Graphics {
            return Err(DeviceModelError::InvalidDevice.into());
        }
        let Some(provider) = self.hardware.as_mut() else {
            return Ok(None);
        };
        provider
            .primary_gpu_media_evidence()
            .map_err(|_| DeviceModelError::InvalidDevice.into())
    }

    pub fn graphics_start_media_session(
        &mut self,
        device_path: &str,
        width: u32,
        height: u32,
        bitrate_kbps: u32,
        codec: u32,
    ) -> Result<(), RuntimeError> {
        let device = self
            .device_registry
            .devices
            .iter()
            .find(|device| device.path == device_path)
            .ok_or(DeviceModelError::InvalidDevice)?;
        if device.class != DeviceClass::Graphics {
            return Err(DeviceModelError::InvalidDevice.into());
        }
        let Some(provider) = self.hardware.as_mut() else {
            return Err(DeviceModelError::InvalidDevice.into());
        };
        provider
            .start_primary_gpu_media_session(width, height, bitrate_kbps, codec)
            .map_err(|_| DeviceModelError::InvalidDevice.into())
    }

    pub fn graphics_neural_evidence(
        &mut self,
        device_path: &str,
    ) -> Result<Option<platform_hal::GpuNeuralEvidence>, RuntimeError> {
        let device = self
            .device_registry
            .devices
            .iter()
            .find(|device| device.path == device_path)
            .ok_or(DeviceModelError::InvalidDevice)?;
        if device.class != DeviceClass::Graphics {
            return Err(DeviceModelError::InvalidDevice.into());
        }
        let Some(provider) = self.hardware.as_mut() else {
            return Ok(None);
        };
        provider
            .primary_gpu_neural_evidence()
            .map_err(|_| DeviceModelError::InvalidDevice.into())
    }

    pub fn graphics_inject_neural_semantic(
        &mut self,
        device_path: &str,
        semantic_label: &str,
    ) -> Result<(), RuntimeError> {
        let device = self
            .device_registry
            .devices
            .iter()
            .find(|device| device.path == device_path)
            .ok_or(DeviceModelError::InvalidDevice)?;
        if device.class != DeviceClass::Graphics {
            return Err(DeviceModelError::InvalidDevice.into());
        }
        let Some(provider) = self.hardware.as_mut() else {
            return Err(DeviceModelError::InvalidDevice.into());
        };
        provider
            .inject_primary_gpu_neural_semantic(semantic_label)
            .map_err(|_| DeviceModelError::InvalidDevice.into())
    }

    pub fn graphics_commit_neural_frame(&mut self, device_path: &str) -> Result<(), RuntimeError> {
        let device = self
            .device_registry
            .devices
            .iter()
            .find(|device| device.path == device_path)
            .ok_or(DeviceModelError::InvalidDevice)?;
        if device.class != DeviceClass::Graphics {
            return Err(DeviceModelError::InvalidDevice.into());
        }
        let Some(provider) = self.hardware.as_mut() else {
            return Err(DeviceModelError::InvalidDevice.into());
        };
        provider
            .commit_primary_gpu_neural_frame()
            .map_err(|_| DeviceModelError::InvalidDevice.into())
    }

    pub fn graphics_tensor_evidence(
        &mut self,
        device_path: &str,
    ) -> Result<Option<platform_hal::GpuTensorEvidence>, RuntimeError> {
        let device = self
            .device_registry
            .devices
            .iter()
            .find(|device| device.path == device_path)
            .ok_or(DeviceModelError::InvalidDevice)?;
        if device.class != DeviceClass::Graphics {
            return Err(DeviceModelError::InvalidDevice.into());
        }
        let Some(provider) = self.hardware.as_mut() else {
            return Ok(None);
        };
        provider
            .primary_gpu_tensor_evidence()
            .map_err(|_| DeviceModelError::InvalidDevice.into())
    }

    pub fn graphics_dispatch_tensor_kernel(
        &mut self,
        device_path: &str,
        kernel_id: u32,
    ) -> Result<(), RuntimeError> {
        let device = self
            .device_registry
            .devices
            .iter()
            .find(|device| device.path == device_path)
            .ok_or(DeviceModelError::InvalidDevice)?;
        if device.class != DeviceClass::Graphics {
            return Err(DeviceModelError::InvalidDevice.into());
        }
        let Some(provider) = self.hardware.as_mut() else {
            return Err(DeviceModelError::InvalidDevice.into());
        };
        provider
            .dispatch_primary_gpu_tensor_kernel(kernel_id)
            .map_err(|_| DeviceModelError::InvalidDevice.into())
    }

    pub fn read_graphics_scanout_frame(
        &self,
        device_path: &str,
        buffer_len: usize,
    ) -> Result<Vec<u8>, RuntimeError> {
        let device = self
            .device_registry
            .devices
            .iter()
            .find(|device| device.path == device_path)
            .ok_or(DeviceModelError::InvalidDevice)?;
        if device.class != DeviceClass::Graphics {
            return Err(DeviceModelError::InvalidDevice.into());
        }
        let copy_len = core::cmp::min(buffer_len, device.graphics_last_presented_frame.len());
        Ok(device.graphics_last_presented_frame[..copy_len].to_vec())
    }

    pub fn submit_graphics_buffer(
        &mut self,
        owner: ProcessId,
        device_path: &str,
        buffer_id: u64,
    ) -> Result<usize, RuntimeError> {
        let buffer_index = self
            .device_registry
            .gpu_buffers
            .iter()
            .position(|buffer| buffer.id == buffer_id)
            .ok_or(DeviceModelError::RequestNotFound)?;
        let (payload_len, payload_bytes) = {
            let buffer = &mut self.device_registry.gpu_buffers[buffer_index];
            if buffer.owner != owner {
                return Err(DeviceModelError::InvalidRequestState.into());
            }
            if buffer.busy {
                return Err(DeviceModelError::InvalidRequestState.into());
            }
            let len = buffer.used_len;
            let bytes = buffer.bytes[..len].to_vec();
            buffer.busy = true;
            (len, bytes)
        };

        // Try Hardware Path first (NVIDIA Blackwell)
        if let Some(hw) = self.hardware.as_mut() {
            if hw.submit_gpu_command(0x100, &payload_bytes).is_ok() {
                {
                    let device = device_mut(&mut self.device_registry, device_path)?;
                    device.submitted_requests = device.submitted_requests.saturating_add(1);
                    device.completed_requests = device.completed_requests.saturating_add(1);
                    device.graphics_presented_frames =
                        device.graphics_presented_frames.saturating_add(1);
                    device.graphics_last_presented_frame.clear();
                    device
                        .graphics_last_presented_frame
                        .extend_from_slice(&payload_bytes);
                    device.graphics_control_reserve_armed = true;
                }

                self.device_registry.gpu_buffers[buffer_index].busy = false;

                if let Some(device_inode) = graphics_event_device_inode(self, device_path) {
                    let _ = event_queue_runtime::emit_graphics_events(
                        self,
                        device_inode,
                        0,
                        GraphicsEventKind::Submitted,
                    );
                    let _ = event_queue_runtime::emit_graphics_events(
                        self,
                        device_inode,
                        0,
                        GraphicsEventKind::Completed,
                    );
                    let _ = event_queue_runtime::emit_graphics_events(
                        self,
                        device_inode,
                        0,
                        GraphicsEventKind::Drained,
                    );
                }

                return Ok(payload_len);
            }
        }

        // Fallback to Software Simulation
        self.enqueue_graphics_request(
            owner,
            device_path,
            DeviceRequestKind::Write,
            None,
            Some(buffer_id),
            Some(payload_len),
            Vec::new(),
        )
    }

    pub fn present_graphics_frame(
        &mut self,
        owner: ProcessId,
        device_path: &str,
        frame: &[u8],
    ) -> Result<u32, RuntimeError> {
        if frame.is_empty() {
            return Err(DeviceModelError::InvalidRequestState.into());
        }
        let payload = frame.to_vec();
        if let Some(hw) = self.hardware.as_mut() {
            if hw.submit_gpu_command(0x4750_0001, &payload).is_ok() {
                let device = device_mut(&mut self.device_registry, device_path)?;
                if device.class != DeviceClass::Graphics || device.state != DeviceState::Bound {
                    return Err(DeviceModelError::InvalidDevice.into());
                }
                device.submitted_requests = device.submitted_requests.saturating_add(1);
                device.completed_requests = device.completed_requests.saturating_add(1);
                device.graphics_presented_frames =
                    device.graphics_presented_frames.saturating_add(1);
                device.graphics_last_presented_frame = payload;
                if let Some(device_inode) = graphics_event_device_inode(self, device_path) {
                    let _ = event_queue_runtime::emit_graphics_events(
                        self,
                        device_inode,
                        0,
                        GraphicsEventKind::Submitted,
                    );
                    let _ = event_queue_runtime::emit_graphics_events(
                        self,
                        device_inode,
                        0,
                        GraphicsEventKind::Completed,
                    );
                    let _ = event_queue_runtime::emit_graphics_events(
                        self,
                        device_inode,
                        0,
                        GraphicsEventKind::Drained,
                    );
                }
                return Ok(0x4750_0000);
            }
        }
        let request_id = self.enqueue_graphics_request(
            owner,
            device_path,
            DeviceRequestKind::Control,
            Some(0x4750_0001),
            None,
            None,
            Vec::new(),
        )?;
        Ok(0x4750_0001 ^ request_id as u32)
    }

    fn enqueue_graphics_request(
        &mut self,
        owner: ProcessId,
        device_path: &str,
        kind: DeviceRequestKind,
        opcode: Option<u32>,
        graphics_buffer_id: Option<u64>,
        graphics_buffer_len: Option<usize>,
        payload: Vec<u8>,
    ) -> Result<usize, RuntimeError> {
        let driver_path: String = {
            let device = device_mut(&mut self.device_registry, device_path)?;
            let Some(driver_path) = device.driver.clone() else {
                return Ok(0);
            };
            if device.pending_requests.len() >= device.queue_capacity {
                return Err(DeviceModelError::QueueFull.into());
            }
            device.submitted_requests = device.submitted_requests.saturating_add(1);
            driver_path
        };
        let request_id = self.device_registry.next_request_id;
        self.device_registry.next_request_id =
            self.device_registry.next_request_id.saturating_add(1);
        self.device_registry.requests.push(DeviceRequest {
            id: request_id,
            device_path: device_path.to_string(),
            driver_path: driver_path.clone(),
            issuer: owner,
            kind,
            state: DeviceRequestState::Queued,
            opcode,
            graphics_buffer_id,
            graphics_buffer_len,
            payload,
            response: Vec::new(),
            submitted_tick: self.current_tick,
            started_tick: None,
            completed_tick: None,
        });
        if let Some(device_inode) = graphics_event_device_inode(self, device_path) {
            let _ = event_queue_runtime::emit_graphics_events(
                self,
                device_inode,
                request_id,
                GraphicsEventKind::Submitted,
            );
        }
        {
            let device = device_mut(&mut self.device_registry, device_path)?;
            device.pending_requests.push(request_id);
        }
        let control_insert_index = if kind == DeviceRequestKind::Control {
            self.device_registry
                .drivers
                .iter()
                .find(|driver| driver.path == driver_path)
                .map(|driver| {
                    driver
                        .queued_requests
                        .iter()
                        .position(|candidate| {
                            self.device_registry
                                .requests
                                .iter()
                                .find(|request| request.id == *candidate)
                                .is_some_and(|request| request.kind == DeviceRequestKind::Write)
                        })
                        .unwrap_or(driver.queued_requests.len())
                })
        } else {
            None
        };
        {
            let driver = driver_mut(&mut self.device_registry, &driver_path)?;
            driver.state = DriverState::Active;
            if let Some(insert_index) = control_insert_index {
                driver.queued_requests.insert(insert_index, request_id);
            } else {
                driver.queued_requests.push(request_id);
            }
        }
        for owners in [
            self.descriptor_bindings_for_path(device_path)?,
            self.descriptor_bindings_for_path(&driver_path)?,
        ] {
            for (binding_owner, binding_fd) in owners {
                let _ = sync_endpoint_io_state(self, binding_owner, binding_fd);
                let _ = self.notify_descriptor_ready(binding_owner, binding_fd);
            }
        }
        Ok(request_id as usize)
    }

    fn complete_stream_device_write(
        &mut self,
        device_path: &str,
        bytes: &[u8],
    ) -> Result<usize, RuntimeError> {
        let driver_path = {
            let device = device_mut(&mut self.device_registry, device_path)?;
            if device.state != DeviceState::Bound {
                return Err(DeviceModelError::InvalidDevice.into());
            }
            if !matches!(device.class, DeviceClass::Audio | DeviceClass::Input) {
                return Err(DeviceModelError::InvalidDevice.into());
            }
            device.submitted_requests = device.submitted_requests.saturating_add(1);
            device.completed_requests = device.completed_requests.saturating_add(1);
            device.total_latency_ticks = device.total_latency_ticks.saturating_add(1);
            device.max_latency_ticks = device.max_latency_ticks.max(1);
            device.driver.clone()
        };
        if let Some(driver_path) = driver_path.as_deref() {
            let driver = driver_mut(&mut self.device_registry, driver_path)?;
            driver.state = DriverState::Active;
            driver.completed_requests = driver.completed_requests.saturating_add(1);
        }
        let notify_paths = if let Some(driver_path) = driver_path.as_deref() {
            vec![device_path, driver_path]
        } else {
            vec![device_path]
        };
        let _ = refresh_and_notify_bindings_for_paths(self, &notify_paths);
        Ok(bytes.len())
    }

    fn descriptor_bindings_for_path(
        &self,
        path: &str,
    ) -> Result<Vec<(ProcessId, Descriptor)>, RuntimeError> {
        let mut bindings = Vec::new();
        for (owner, namespace) in &self.namespaces {
            for fd in namespace.by_owner(*owner) {
                let descriptor = namespace.get(fd).map_err(RuntimeError::from)?;
                if descriptor.name() == path {
                    bindings.push((*owner, fd));
                }
            }
        }
        Ok(bindings)
    }

    pub(crate) fn endpoint_read_io(
        &mut self,
        owner: ProcessId,
        fd: Descriptor,
    ) -> Result<Option<Vec<u8>>, RuntimeError> {
        let descriptor = self
            .namespace(owner)?
            .get(fd)
            .map_err(RuntimeError::from)?
            .clone();
        match descriptor.kind() {
            ObjectKind::Socket => {
                if let Some(index) = self
                    .network_sockets
                    .iter()
                    .position(|socket| socket.path == descriptor.name() && socket.owner == owner)
                {
                    if self.network_sockets[index].rx_queue.is_empty() {
                        return Ok(None);
                    }
                    let packet = self.network_sockets[index].rx_queue.remove(0);
                    let iface_index = self
                        .network_ifaces
                        .iter()
                        .position(|iface| {
                            iface.device_path == self.network_sockets[index].interface
                        })
                        .ok_or(DeviceModelError::InvalidDevice)?;
                    let payload = network_buffer_payload(
                        &self.network_ifaces[iface_index],
                        packet.buffer_id,
                    )?
                    .to_vec();
                    self.release_network_buffer(iface_index, packet.buffer_id)?;
                    self.network_sockets[index].rx_packets =
                        self.network_sockets[index].rx_packets.saturating_add(1);
                    sync_endpoint_io_state(self, owner, fd)?;
                    return Ok(Some(payload));
                }
                Ok(None)
            }
            ObjectKind::Channel => {
                let payload = {
                    let Some(channel) = runtime_channel_mut(self, descriptor.name()) else {
                        return Ok(None);
                    };
                    if channel.messages.is_empty() {
                        return Ok(None);
                    }
                    channel.messages.remove(0)
                };
                let bindings = self.descriptor_bindings_for_path(descriptor.name())?;
                for (binding_owner, binding_fd) in bindings {
                    sync_endpoint_io_state(self, binding_owner, binding_fd)?;
                }
                Ok(Some(payload))
            }
            ObjectKind::Driver => {
                if let Some(iface_index) = self
                    .network_ifaces
                    .iter()
                    .position(|iface| iface.driver_path == descriptor.name())
                    && !self.network_ifaces[iface_index].tx_ring.is_empty()
                {
                    let interface_inode =
                        path_inode(self, &self.network_ifaces[iface_index].device_path)?;
                    if self.network_ifaces[iface_index].tx_in_flight.len()
                        >= self.network_ifaces[iface_index].tx_inflight_limit
                    {
                        return Ok(None);
                    }
                    let buffer_id = self.network_ifaces[iface_index].tx_ring.remove(0);
                    self.network_ifaces[iface_index]
                        .tx_in_flight
                        .push(buffer_id);
                    self.network_buffer_mut(iface_index, buffer_id)?.state =
                        NetworkBufferState::TxInFlight;
                    let payload_buf =
                        network_buffer_payload(&self.network_ifaces[iface_index], buffer_id)?
                            .to_vec();
                    let source_socket = self.network_ifaces[iface_index]
                        .buffers
                        .iter()
                        .find(|buffer| buffer.id == buffer_id)
                        .map(|buffer| buffer.source_socket.clone())
                        .ok_or(DeviceModelError::RequestNotFound)?;
                    let tx_drained = self.network_ifaces[iface_index].tx_ring.is_empty();
                    let (src_port, dst_port) = parse_udp_ipv4_frame(&payload_buf)
                        .map(|(_, _, _, _, src_port, dst_port, _)| (src_port, dst_port))
                        .unwrap_or((0, 0));
                    let payload = format!(
                        "net-tx iface={} socket={} bytes={} sport={} dport={} buffer={} queued={} inflight={}\n",
                        self.network_ifaces[iface_index].device_path,
                        source_socket,
                        payload_buf.len(),
                        src_port,
                        dst_port,
                        buffer_id,
                        self.network_ifaces[iface_index].tx_ring.len(),
                        self.network_ifaces[iface_index].tx_in_flight.len()
                    )
                    .into_bytes()
                    .into_iter()
                    .chain(payload_buf)
                    .collect::<Vec<_>>();
                    sync_endpoint_io_state(self, owner, fd)?;
                    if tx_drained {
                        event_queue_runtime::emit_network_events(
                            self,
                            interface_inode,
                            None,
                            NetworkEventKind::TxDrained,
                        )?;
                    }
                    return Ok(Some(payload));
                }
                if !self
                    .device_registry
                    .drivers
                    .iter()
                    .any(|driver| driver.path == descriptor.name())
                {
                    return Ok(None);
                }
                if self
                    .device_registry
                    .drivers
                    .iter()
                    .find(|driver| driver.path == descriptor.name())
                    .is_some_and(|driver| driver.state == DriverState::Retired)
                {
                    return Err(DeviceModelError::InvalidDriver.into());
                }
                let request_id = {
                    let driver_path = descriptor.name().to_string();
                    let inflight_request_id = self
                        .device_registry
                        .drivers
                        .iter()
                        .find(|driver| driver.path == driver_path)
                        .and_then(|driver| driver.in_flight_requests.first().copied());
                    if let Some(request_id) = inflight_request_id {
                        request_id
                    } else if let Some(request_id) = self
                        .device_registry
                        .drivers
                        .iter()
                        .find(|driver| driver.path == driver_path)
                        .and_then(|driver| driver.queued_requests.first().copied())
                    {
                        let driver = driver_mut(&mut self.device_registry, descriptor.name())?;
                        driver.queued_requests.remove(0);
                        driver.in_flight_requests.push(request_id);
                        request_id
                    } else {
                        return Ok(None);
                    }
                };
                let request = self
                    .device_registry
                    .requests
                    .iter_mut()
                    .find(|request| request.id == request_id)
                    .ok_or(DeviceModelError::RequestNotFound)?;
                request.state = DeviceRequestState::InFlight;
                request.started_tick = Some(self.current_tick);
                let mut header = format!(
                    "request:{} kind={:?} device={} opcode={:?}",
                    request.id, request.kind, request.device_path, request.opcode
                );
                if let Some(buffer_id) = request.graphics_buffer_id {
                    header.push_str(&format!(" buffer={}", buffer_id));
                }
                header.push('\n');
                let body = if let Some(buffer_id) = request.graphics_buffer_id {
                    let buffer = self
                        .device_registry
                        .gpu_buffers
                        .iter()
                        .find(|buffer| buffer.id == buffer_id)
                        .ok_or(DeviceModelError::RequestNotFound)?;
                    let payload_len = request.graphics_buffer_len.unwrap_or(buffer.used_len);
                    buffer.bytes[..payload_len].to_vec()
                } else {
                    request.payload.clone()
                };
                let payload = header
                    .into_bytes()
                    .into_iter()
                    .chain(body)
                    .collect::<Vec<_>>();
                sync_endpoint_io_state(self, owner, fd)?;
                Ok(Some(payload))
            }
            ObjectKind::Device => {
                if !self
                    .device_registry
                    .devices
                    .iter()
                    .any(|device| device.path == descriptor.name())
                {
                    return Ok(None);
                }
                let request_id = {
                    let device = device_mut(&mut self.device_registry, descriptor.name())?;
                    if device.completion_queue.is_empty() {
                        return Ok(None);
                    }
                    device.completion_queue.remove(0)
                };
                let request = self
                    .device_registry
                    .requests
                    .iter()
                    .find(|request| request.id == request_id)
                    .ok_or(DeviceModelError::RequestNotFound)?;
                let payload = request.response.clone();
                sync_endpoint_io_state(self, owner, fd)?;
                Ok(Some(payload))
            }
            _ => Ok(None),
        }
    }

    pub(crate) fn endpoint_write_io(
        &mut self,
        owner: ProcessId,
        fd: Descriptor,
        bytes: &[u8],
    ) -> Result<Option<usize>, RuntimeError> {
        let descriptor = self
            .namespace(owner)?
            .get(fd)
            .map_err(RuntimeError::from)?
            .clone();
        match descriptor.kind() {
            ObjectKind::Socket => {
                if !self
                    .network_sockets
                    .iter()
                    .any(|socket| socket.path == descriptor.name() && socket.owner == owner)
                {
                    return Ok(None);
                }
                self.send_udp_socket_to(descriptor.name(), owner, [0, 0, 0, 0], 0, bytes)
                    .map(Some)
            }
            ObjectKind::Channel => {
                let channel = ensure_runtime_channel(self, descriptor.name());
                channel.messages.push(bytes.to_vec());
                let bindings = self.descriptor_bindings_for_path(descriptor.name())?;
                for (binding_owner, binding_fd) in bindings {
                    sync_endpoint_io_state(self, binding_owner, binding_fd)?;
                    self.notify_descriptor_ready(binding_owner, binding_fd)?;
                }
                Ok(Some(bytes.len()))
            }
            ObjectKind::Device => {
                if !self
                    .device_registry
                    .devices
                    .iter()
                    .any(|device| device.path == descriptor.name())
                {
                    return Ok(None);
                }
                if self
                    .device_registry
                    .devices
                    .iter()
                    .find(|device| device.path == descriptor.name())
                    .is_some_and(|device| device.class == DeviceClass::Graphics)
                {
                    if self
                        .device_registry
                        .devices
                        .iter()
                        .find(|device| device.path == descriptor.name())
                        .is_some_and(|device| device.state != DeviceState::Bound)
                    {
                        return Err(DeviceModelError::InvalidDevice.into());
                    }
                    enforce_graphics_device_lease(self, owner, descriptor.name())?;
                    if let Some(device) = self
                        .device_registry
                        .devices
                        .iter()
                        .find(|device| device.path == descriptor.name())
                        && device.queue_capacity > 1
                        && device.graphics_control_reserve_armed
                        && device.pending_requests.len().saturating_add(1) >= device.queue_capacity
                    {
                        return Err(DeviceModelError::QueueFull.into());
                    }
                }
                let class = self
                    .device_registry
                    .devices
                    .iter()
                    .find(|device| device.path == descriptor.name())
                    .map(|device| device.class)
                    .ok_or(DeviceModelError::InvalidDevice)?;
                match class {
                    DeviceClass::Audio | DeviceClass::Input => {
                        let written =
                            self.complete_stream_device_write(descriptor.name(), bytes)?;
                        Ok(Some(written))
                    }
                    _ => {
                        self.enqueue_graphics_request(
                            owner,
                            descriptor.name(),
                            DeviceRequestKind::Write,
                            None,
                            None,
                            None,
                            bytes.to_vec(),
                        )?;
                        Ok(Some(bytes.len()))
                    }
                }
            }
            ObjectKind::Driver => {
                if let Some(iface_index) = self
                    .network_ifaces
                    .iter()
                    .position(|iface| iface.driver_path == descriptor.name())
                {
                    let interface_inode =
                        path_inode(self, &self.network_ifaces[iface_index].device_path)?;
                    let frame = bytes.to_vec();
                    if frame.len().saturating_sub(14) > self.network_ifaces[iface_index].mtu {
                        return Err(DeviceModelError::PacketTooLarge.into());
                    }
                    if self.network_ifaces[iface_index].rx_ring.len()
                        >= self.network_ifaces[iface_index].rx_capacity
                    {
                        self.network_ifaces[iface_index].rx_dropped = self.network_ifaces
                            [iface_index]
                            .rx_dropped
                            .saturating_add(1);
                        return Err(DeviceModelError::QueueFull.into());
                    }
                    let buffer_id = self.alloc_network_buffer(
                        iface_index,
                        descriptor.name().to_string(),
                        frame.clone(),
                        NetworkBufferState::SocketQueued,
                    )?;
                    self.network_ifaces[iface_index].rx_ring.push(buffer_id);
                    self.network_ifaces[iface_index].rx_packets = self.network_ifaces[iface_index]
                        .rx_packets
                        .saturating_add(1);
                    if let Some((_, _, src_ip, dst_ip, src_port, dst_port, payload)) =
                        parse_udp_ipv4_frame(&frame)
                    {
                        let sockets = self.network_ifaces[iface_index].attached_sockets.clone();
                        let mut socket_inodes = Vec::new();
                        let mut delivered = false;
                        let mut delivery_error = None;
                        for socket_path in sockets {
                            if let Some(socket_index) =
                                self.network_sockets.iter().position(|socket| {
                                    socket.path == socket_path
                                        && socket.interface
                                            == self.network_ifaces[iface_index].device_path
                                        && socket.local_port == dst_port
                                        && (self.network_ifaces[iface_index].promiscuous
                                            || socket.local_ipv4 == dst_ip)
                                        && (socket.remote_port == src_port
                                            || socket.remote_port == 0)
                                        && (socket.remote_ipv4 == src_ip
                                            || socket.remote_ipv4 == [0, 0, 0, 0])
                                })
                            {
                                let clone_id = if delivered {
                                    self.alloc_network_buffer(
                                        iface_index,
                                        socket_path.clone(),
                                        payload.clone(),
                                        NetworkBufferState::SocketQueued,
                                    )?
                                } else {
                                    let rx_buffer =
                                        self.network_buffer_mut(iface_index, buffer_id)?;
                                    rx_buffer.payload = payload.clone();
                                    rx_buffer.source_socket = socket_path.clone();
                                    buffer_id
                                };
                                match self.queue_socket_rx_buffer(
                                    iface_index,
                                    socket_index,
                                    clone_id,
                                    src_ip,
                                    dst_ip,
                                    src_port,
                                    dst_port,
                                ) {
                                    Ok(()) => {
                                        delivered = true;
                                        socket_inodes.push(path_inode(self, &socket_path)?);
                                        for (binding_owner, binding_fd) in
                                            self.descriptor_bindings_for_path(&socket_path)?
                                        {
                                            let _ = sync_endpoint_io_state(
                                                self,
                                                binding_owner,
                                                binding_fd,
                                            );
                                            let _ = self
                                                .notify_descriptor_ready(binding_owner, binding_fd);
                                        }
                                    }
                                    Err(error) => {
                                        delivery_error = Some(error);
                                    }
                                }
                            }
                        }
                        self.network_ifaces[iface_index].rx_ring.clear();
                        if !delivered {
                            self.network_ifaces[iface_index].rx_dropped = self.network_ifaces
                                [iface_index]
                                .rx_dropped
                                .saturating_add(1);
                            self.release_network_buffer(iface_index, buffer_id)?;
                            if let Some(error) = delivery_error {
                                return Err(error);
                            }
                        }
                        event_queue_runtime::emit_network_events(
                            self,
                            interface_inode,
                            None,
                            NetworkEventKind::RxReady,
                        )?;
                        for socket_inode in socket_inodes {
                            event_queue_runtime::emit_network_events(
                                self,
                                interface_inode,
                                Some(socket_inode),
                                NetworkEventKind::RxReady,
                            )?;
                        }
                    }
                    let _ = complete_device_driver_request(self, descriptor.name(), bytes)?;
                    self.io_registry
                        .replace_payload(owner, fd, b"")
                        .map_err(map_runtime_io_error)?;
                    self.io_registry
                        .set_state(owner, fd, IoState::Writable)
                        .map_err(map_runtime_io_error)?;
                    return Ok(Some(bytes.len()));
                }
                if !self
                    .device_registry
                    .drivers
                    .iter()
                    .any(|driver| driver.path == descriptor.name())
                {
                    return Ok(None);
                }
                let _ = complete_device_driver_request(self, descriptor.name(), bytes)?;
                Ok(Some(bytes.len()))
            }
            _ => Ok(None),
        }
    }

    pub(crate) fn endpoint_control_io(
        &mut self,
        owner: ProcessId,
        fd: Descriptor,
        opcode: u32,
    ) -> Result<Option<u32>, RuntimeError> {
        let descriptor = self
            .namespace(owner)?
            .get(fd)
            .map_err(RuntimeError::from)?
            .clone();
        if descriptor.kind() == ObjectKind::Driver {
            if !is_graphics_driver(self, descriptor.name()) {
                return Ok(None);
            }
            let driver_state = self
                .device_registry
                .drivers
                .iter()
                .find(|driver| driver.path == descriptor.name())
                .map(|driver| driver.state)
                .ok_or(DeviceModelError::InvalidDriver)?;
            if driver_state == DriverState::Retired {
                return Err(DeviceModelError::InvalidDriver.into());
            }
            if opcode == 0x4750_1001 {
                return Ok(Some(reset_graphics_driver(self, descriptor.name())?));
            }
            if opcode == 0x4750_1002 {
                return Ok(Some(retire_graphics_driver(self, descriptor.name())?));
            }
            if opcode != 0x4750_1001 && opcode != 0x4750_1002 {
                return Err(DeviceModelError::InvalidRequestState.into());
            }
        }
        if descriptor.kind() != ObjectKind::Device {
            return Ok(None);
        }
        if !self
            .device_registry
            .devices
            .iter()
            .any(|device| device.path == descriptor.name())
        {
            return Ok(None);
        }
        if self
            .device_registry
            .devices
            .iter()
            .find(|device| device.path == descriptor.name())
            .is_some_and(|device| device.class == DeviceClass::Graphics)
        {
            if self
                .device_registry
                .devices
                .iter()
                .find(|device| device.path == descriptor.name())
                .is_some_and(|device| device.state != DeviceState::Bound)
            {
                return Err(DeviceModelError::InvalidDevice.into());
            }
            enforce_graphics_device_lease(self, owner, descriptor.name())?;
        }
        let request_id = self.enqueue_graphics_request(
            owner,
            descriptor.name(),
            DeviceRequestKind::Control,
            Some(opcode),
            None,
            None,
            Vec::new(),
        )?;
        Ok(Some(opcode ^ request_id as u32))
    }

    pub(crate) fn endpoint_poll_io(
        &self,
        owner: ProcessId,
        fd: Descriptor,
    ) -> Result<Option<IoPollEvents>, RuntimeError> {
        let descriptor = self.namespace(owner)?.get(fd).map_err(RuntimeError::from)?;
        match descriptor.kind() {
            ObjectKind::Socket => {
                let Some(socket) = self
                    .network_sockets
                    .iter()
                    .find(|socket| socket.path == descriptor.name() && socket.owner == owner)
                else {
                    return Ok(None);
                };
                let Some(iface) = self
                    .network_ifaces
                    .iter()
                    .find(|iface| iface.device_path == socket.interface)
                else {
                    return Ok(None);
                };
                let mut events = IoPollEvents::PRIORITY;
                if !socket.rx_queue.is_empty() {
                    events = events | IoPollEvents::READABLE;
                }
                if network_effective_link_up(iface)
                    && iface.tx_ring.len() < iface.tx_capacity
                    && iface.tx_in_flight.len() < iface.tx_inflight_limit
                {
                    events = events | IoPollEvents::WRITABLE;
                }
                Ok(Some(events))
            }
            ObjectKind::Driver => {
                let Some(driver) = self
                    .device_registry
                    .drivers
                    .iter()
                    .find(|driver| driver.path == descriptor.name())
                else {
                    return Ok(None);
                };
                let mut events = IoPollEvents::WRITABLE | IoPollEvents::PRIORITY;
                let network_readable = self
                    .network_ifaces
                    .iter()
                    .any(|iface| iface.driver_path == driver.path && !iface.tx_ring.is_empty());
                if !driver.queued_requests.is_empty() || !driver.in_flight_requests.is_empty() {
                    events = events | IoPollEvents::READABLE;
                }
                if network_readable {
                    events = events | IoPollEvents::READABLE;
                }
                Ok(Some(events))
            }
            ObjectKind::Device => {
                let Some(device) = self
                    .device_registry
                    .devices
                    .iter()
                    .find(|device| device.path == descriptor.name())
                else {
                    return Ok(None);
                };
                let mut events = IoPollEvents::WRITABLE;
                if !device.completion_queue.is_empty() {
                    events = events | IoPollEvents::READABLE;
                }
                if device.driver.is_some() {
                    events = events | IoPollEvents::PRIORITY;
                }
                Ok(Some(events))
            }
            _ => Ok(None),
        }
    }
}
