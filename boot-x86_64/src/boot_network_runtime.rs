extern crate alloc;

use alloc::collections::VecDeque;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

use ngos_user_abi::{
    Errno, NativeDeviceRecord, NativeDeviceRequestRecord, NativeDriverRecord,
    NativeNetworkInterfaceRecord, NativeNetworkSocketRecord, NativeUdpRecvMeta, POLLIN, POLLOUT,
};

pub const NETWORK_DEVICE_PATH: &str = "/dev/net0";
pub const NETWORK_DRIVER_PATH: &str = "/drv/net0";
pub const NETWORK_DEVICE1_PATH: &str = "/dev/net1";
pub const NETWORK_DRIVER1_PATH: &str = "/drv/net1";

const NETWORK_DEVICE_CLASS: u32 = 6;
const NETWORK_DEVICE_STATE_REGISTERED: u32 = 0;
const NETWORK_DRIVER_STATE_ACTIVE: u32 = 1;
const NETWORK_QUEUE_CAPACITY: u64 = 128;
const NETWORK_BOOT_ISSUER: u64 = 1;
const NETWORK_REQUEST_KIND_WRITE: u32 = 1;
const NETWORK_SEND_OPCODE: u64 = 0x4e45_5401;
const NETWORK_REQUEST_STATE_INFLIGHT: u32 = 1;
const NETWORK_REQUEST_STATE_COMPLETED: u32 = 2;
const SOCKET_RX_LIMIT: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkEndpointKind {
    Device,
    Driver,
}

#[derive(Debug, Clone)]
struct BootNetworkRequestRecord {
    request_id: u64,
    issuer: u64,
    kind: u32,
    state: u32,
    opcode: u64,
    buffer_id: u64,
    payload_len: u64,
    response_len: u64,
    submitted_tick: u64,
    started_tick: u64,
    completed_tick: u64,
    frame_tag: [u8; 64],
    source_api_name: [u8; 24],
    translation_label: [u8; 32],
}

#[derive(Debug, Clone)]
struct BootNetworkSocket {
    path: String,
    device_path: String,
    local_ipv4: [u8; 4],
    remote_ipv4: [u8; 4],
    local_port: u16,
    remote_port: u16,
    connected: bool,
    rx_queue: VecDeque<([u8; 4], u16, Vec<u8>)>,
    rx_limit: usize,
    tx_packets: u64,
    rx_packets: u64,
    dropped_packets: u64,
}

#[derive(Debug, Clone)]
struct BootNetworkQueueEntry {
    interface_index: usize,
    request_id: u64,
    payload: Vec<u8>,
}

#[derive(Debug, Clone)]
struct BootNetworkInterfaceState {
    device_path: String,
    admin_up: bool,
    link_up: bool,
    promiscuous: bool,
    mtu: u64,
    tx_capacity: u64,
    rx_capacity: u64,
    tx_inflight_limit: u64,
    mac: [u8; 6],
    ipv4_addr: [u8; 4],
    ipv4_netmask: [u8; 4],
    ipv4_gateway: [u8; 4],
    tx_packets: u64,
    rx_packets: u64,
    tx_completions: u64,
    tx_dropped: u64,
    rx_dropped: u64,
    in_flight_requests: u64,
    last_payload_len: u64,
}

impl BootNetworkInterfaceState {
    fn new(index: usize) -> Self {
        let device_path = format!("/dev/net{index}");
        let mac_tail = 0x55u8.saturating_add(index as u8);
        Self {
            device_path,
            admin_up: false,
            link_up: true,
            promiscuous: false,
            mtu: 1500,
            tx_capacity: 4,
            rx_capacity: 4,
            tx_inflight_limit: 2,
            mac: [0x02, 0x11, 0x22, 0x33, 0x44, mac_tail],
            ipv4_addr: [0, 0, 0, 0],
            ipv4_netmask: [0, 0, 0, 0],
            ipv4_gateway: [0, 0, 0, 0],
            tx_packets: 0,
            rx_packets: 0,
            tx_completions: 0,
            tx_dropped: 0,
            rx_dropped: 0,
            in_flight_requests: 0,
            last_payload_len: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UdpSocketLocalBinding {
    pub local_ipv4: [u8; 4],
    pub local_port: u16,
}

#[derive(Debug)]
struct BootNetworkRuntimeState {
    interfaces: Vec<BootNetworkInterfaceState>,
    driver_queue: VecDeque<BootNetworkQueueEntry>,
    completion_queue: VecDeque<BootNetworkQueueEntry>,
    request_records: Vec<BootNetworkRequestRecord>,
    sockets: Vec<BootNetworkSocket>,
    submitted_requests: u64,
    completed_requests: u64,
    last_request_id: u64,
    next_tick: u64,
    last_completed_request_id: u64,
    last_completed_frame_tag: [u8; 64],
    last_completed_source_api_name: [u8; 24],
    last_completed_translation_label: [u8; 32],
    last_terminal_request_id: u64,
    last_terminal_state: u32,
    last_terminal_frame_tag: [u8; 64],
    last_terminal_source_api_name: [u8; 24],
    last_terminal_translation_label: [u8; 32],
    last_payload_len: u64,
    last_completion_payload: Vec<u8>,
}

impl Default for BootNetworkRuntimeState {
    fn default() -> Self {
        Self {
            interfaces: vec![
                BootNetworkInterfaceState::new(0),
                BootNetworkInterfaceState::new(1),
            ],
            driver_queue: VecDeque::new(),
            completion_queue: VecDeque::new(),
            request_records: Vec::new(),
            sockets: Vec::new(),
            submitted_requests: 0,
            completed_requests: 0,
            last_request_id: 0,
            next_tick: 1,
            last_completed_request_id: 0,
            last_completed_frame_tag: [0; 64],
            last_completed_source_api_name: [0; 24],
            last_completed_translation_label: [0; 32],
            last_terminal_request_id: 0,
            last_terminal_state: 0,
            last_terminal_frame_tag: [0; 64],
            last_terminal_source_api_name: [0; 24],
            last_terminal_translation_label: [0; 32],
            last_payload_len: 0,
            last_completion_payload: Vec::new(),
        }
    }
}

struct BootNetworkRuntimeCell {
    locked: AtomicBool,
    state: UnsafeCell<Option<BootNetworkRuntimeState>>,
}

unsafe impl Sync for BootNetworkRuntimeCell {}

impl BootNetworkRuntimeCell {
    const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
            state: UnsafeCell::new(None),
        }
    }

    fn initialize(&self) {
        self.with_mut(|state| {
            if state.is_none() {
                *state = Some(BootNetworkRuntimeState::default());
            }
        });
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut Option<BootNetworkRuntimeState>) -> R) -> R {
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        let result = unsafe { f(&mut *self.state.get()) };
        self.locked.store(false, Ordering::Release);
        result
    }
}

static NETWORK_RUNTIME: BootNetworkRuntimeCell = BootNetworkRuntimeCell::new();

pub fn reset() {
    NETWORK_RUNTIME.with_mut(|state| *state = Some(BootNetworkRuntimeState::default()));
}

pub fn endpoint_for_path(path: &str) -> Option<NetworkEndpointKind> {
    interface_index_from_device_path(path)
        .map(|_| NetworkEndpointKind::Device)
        .or_else(|| interface_index_from_driver_path(path).map(|_| NetworkEndpointKind::Driver))
}

fn interface_index_from_device_path(path: &str) -> Option<usize> {
    path.strip_prefix("/dev/net")?.parse::<usize>().ok()
}

fn interface_index_from_driver_path(path: &str) -> Option<usize> {
    path.strip_prefix("/drv/net")?.parse::<usize>().ok()
}

fn interface_by_device_path_mut<'a>(
    state: &'a mut BootNetworkRuntimeState,
    path: &str,
) -> Option<(usize, &'a mut BootNetworkInterfaceState)> {
    let index = interface_index_from_device_path(path)?;
    if index >= state.interfaces.len() {
        return None;
    }
    Some((index, &mut state.interfaces[index]))
}

pub fn socket_device_path(path: &str) -> Option<String> {
    NETWORK_RUNTIME.initialize();
    NETWORK_RUNTIME.with_mut(|state| {
        let state = state.as_ref()?;
        state
            .sockets
            .iter()
            .find(|socket| socket.path == path)
            .map(|socket| socket.device_path.clone())
    })
}

pub fn endpoint_id(path: &str) -> u64 {
    socket_id(path)
}

fn copy_fixed<const N: usize>(dst: &mut [u8; N], text: &str) {
    *dst = [0; N];
    let bytes = text.as_bytes();
    let len = bytes.len().min(N);
    dst[..len].copy_from_slice(&bytes[..len]);
}

fn update_terminal_metadata(
    state: &mut BootNetworkRuntimeState,
    request_id: u64,
    terminal_state: u32,
    frame_tag: &[u8; 64],
    source_api_name: &[u8; 24],
    translation_label: &[u8; 32],
) {
    state.last_terminal_request_id = request_id;
    state.last_terminal_state = terminal_state;
    state.last_terminal_frame_tag = *frame_tag;
    state.last_terminal_source_api_name = *source_api_name;
    state.last_terminal_translation_label = *translation_label;
}

pub fn socket_id(path: &str) -> u64 {
    let mut hash = 1469598103934665603u64;
    for byte in path.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(1099511628211u64);
    }
    hash
}

pub fn device_record(path: &str) -> Option<NativeDeviceRecord> {
    NETWORK_RUNTIME.initialize();
    NETWORK_RUNTIME.with_mut(|state| {
        let state = state.as_mut()?;
        let interface_index = interface_index_from_device_path(path)?;
        let interface = state.interfaces.get(interface_index)?;
        let queue_depth = state
            .driver_queue
            .iter()
            .filter(|entry| entry.interface_index == interface_index)
            .count() as u64;
        Some(NativeDeviceRecord {
            class: NETWORK_DEVICE_CLASS,
            state: NETWORK_DEVICE_STATE_REGISTERED,
            reserved0: 0,
            queue_depth,
            queue_capacity: NETWORK_QUEUE_CAPACITY,
            submitted_requests: state.submitted_requests,
            completed_requests: state.completed_requests,
            total_latency_ticks: 0,
            max_latency_ticks: 0,
            total_queue_wait_ticks: 0,
            max_queue_wait_ticks: 0,
            link_up: interface.link_up as u32,
            reserved1: 0,
            block_size: 0,
            reserved2: 0,
            capacity_bytes: interface.last_payload_len,
            last_completed_request_id: state.last_completed_request_id,
            last_completed_frame_tag: state.last_completed_frame_tag,
            last_completed_source_api_name: state.last_completed_source_api_name,
            last_completed_translation_label: state.last_completed_translation_label,
            last_terminal_request_id: state.last_terminal_request_id,
            last_terminal_state: state.last_terminal_state,
            reserved3: 0,
            last_terminal_frame_tag: state.last_terminal_frame_tag,
            last_terminal_source_api_name: state.last_terminal_source_api_name,
            last_terminal_translation_label: state.last_terminal_translation_label,
        })
    })
}

pub fn driver_record(path: &str) -> Option<NativeDriverRecord> {
    NETWORK_RUNTIME.initialize();
    NETWORK_RUNTIME.with_mut(|state| {
        let state = state.as_mut()?;
        let interface_index = interface_index_from_driver_path(path)?;
        let interface = state.interfaces.get(interface_index)?;
        let queued_requests = state
            .driver_queue
            .iter()
            .filter(|entry| entry.interface_index == interface_index)
            .count() as u64;
        Some(NativeDriverRecord {
            state: NETWORK_DRIVER_STATE_ACTIVE,
            reserved: 0,
            bound_device_count: 1,
            queued_requests,
            in_flight_requests: interface.in_flight_requests,
            completed_requests: state.completed_requests,
            last_completed_request_id: state.last_completed_request_id,
            last_completed_frame_tag: state.last_completed_frame_tag,
            last_completed_source_api_name: state.last_completed_source_api_name,
            last_completed_translation_label: state.last_completed_translation_label,
            last_terminal_request_id: state.last_terminal_request_id,
            last_terminal_state: state.last_terminal_state,
            reserved1: 0,
            last_terminal_frame_tag: state.last_terminal_frame_tag,
            last_terminal_source_api_name: state.last_terminal_source_api_name,
            last_terminal_translation_label: state.last_terminal_translation_label,
        })
    })
}

pub fn device_request_record(request_id: u64) -> Option<NativeDeviceRequestRecord> {
    NETWORK_RUNTIME.initialize();
    NETWORK_RUNTIME.with_mut(|state| {
        let state = state.as_mut()?;
        let record = state
            .request_records
            .iter()
            .find(|record| record.request_id == request_id)?;
        Some(NativeDeviceRequestRecord {
            issuer: record.issuer,
            kind: record.kind,
            state: record.state,
            opcode: record.opcode,
            buffer_id: record.buffer_id,
            payload_len: record.payload_len,
            response_len: record.response_len,
            submitted_tick: record.submitted_tick,
            started_tick: record.started_tick,
            completed_tick: record.completed_tick,
            frame_tag: record.frame_tag,
            source_api_name: record.source_api_name,
            translation_label: record.translation_label,
        })
    })
}

pub fn interface_record(path: &str) -> Option<NativeNetworkInterfaceRecord> {
    NETWORK_RUNTIME.initialize();
    NETWORK_RUNTIME.with_mut(|state| {
        let state = state.as_mut()?;
        let interface_index = interface_index_from_device_path(path)?;
        let interface = state.interfaces.get(interface_index)?;
        let device_path = interface.device_path.clone();
        let rx_ring_depth = state
            .sockets
            .iter()
            .filter(|socket| socket.device_path == device_path)
            .map(|socket| socket.rx_queue.len() as u64)
            .sum();
        let tx_ring_depth = state
            .driver_queue
            .iter()
            .filter(|entry| entry.interface_index == interface_index)
            .count() as u64;
        let attached_socket_count = state
            .sockets
            .iter()
            .filter(|socket| socket.device_path == device_path)
            .count() as u64;
        Some(NativeNetworkInterfaceRecord {
            admin_up: interface.admin_up as u32,
            link_up: interface.link_up as u32,
            promiscuous: interface.promiscuous as u32,
            reserved: 0,
            mtu: interface.mtu,
            tx_capacity: interface.tx_capacity,
            rx_capacity: interface.rx_capacity,
            tx_inflight_limit: interface.tx_inflight_limit,
            tx_inflight_depth: interface.in_flight_requests,
            free_buffer_count: interface.rx_capacity.saturating_sub(interface.rx_packets),
            mac: interface.mac,
            mac_reserved: [0; 2],
            ipv4_addr: interface.ipv4_addr,
            ipv4_netmask: interface.ipv4_netmask,
            ipv4_gateway: interface.ipv4_gateway,
            ipv4_reserved: [0; 4],
            rx_ring_depth,
            tx_ring_depth,
            tx_packets: interface.tx_packets,
            rx_packets: interface.rx_packets,
            tx_completions: interface.tx_completions,
            tx_dropped: interface.tx_dropped,
            rx_dropped: interface.rx_dropped,
            attached_socket_count,
        })
    })
}

pub fn socket_record(path: &str) -> Option<NativeNetworkSocketRecord> {
    NETWORK_RUNTIME.initialize();
    NETWORK_RUNTIME.with_mut(|state| {
        let state = state.as_mut()?;
        let socket = state.sockets.iter().find(|socket| socket.path == path)?;
        Some(NativeNetworkSocketRecord {
            local_ipv4: socket.local_ipv4,
            remote_ipv4: socket.remote_ipv4,
            local_port: socket.local_port,
            remote_port: socket.remote_port,
            connected: socket.connected as u32,
            reserved: 0,
            rx_depth: socket.rx_queue.len() as u64,
            rx_queue_limit: socket.rx_limit as u64,
            tx_packets: socket.tx_packets,
            rx_packets: socket.rx_packets,
            dropped_packets: socket.dropped_packets,
        })
    })
}

pub fn attached_socket_count(path: &str) -> u64 {
    NETWORK_RUNTIME.initialize();
    NETWORK_RUNTIME.with_mut(|state| {
        let Some(state) = state.as_ref() else {
            return 0;
        };
        state
            .sockets
            .iter()
            .filter(|socket| socket.device_path == path)
            .count() as u64
    })
}

pub fn udp_socket_local_binding(path: &str) -> Result<UdpSocketLocalBinding, Errno> {
    NETWORK_RUNTIME.initialize();
    NETWORK_RUNTIME.with_mut(|state| {
        let state = state.as_ref().ok_or(Errno::NoEnt)?;
        let socket = state
            .sockets
            .iter()
            .find(|socket| socket.path == path)
            .ok_or(Errno::NoEnt)?;
        let interface = state
            .interfaces
            .iter()
            .find(|interface| interface.device_path == socket.device_path)
            .ok_or(Errno::NoEnt)?;
        Ok(UdpSocketLocalBinding {
            local_ipv4: if socket.local_ipv4 == [0, 0, 0, 0] {
                interface.ipv4_addr
            } else {
                socket.local_ipv4
            },
            local_port: socket.local_port,
        })
    })
}

pub fn record_udp_socket_tx(path: &str) -> Result<(), Errno> {
    NETWORK_RUNTIME.initialize();
    NETWORK_RUNTIME.with_mut(|state| {
        let state = state.as_mut().ok_or(Errno::NoEnt)?;
        let socket = state
            .sockets
            .iter_mut()
            .find(|socket| socket.path == path)
            .ok_or(Errno::NoEnt)?;
        socket.tx_packets = socket.tx_packets.saturating_add(1);
        Ok(())
    })
}

pub fn remove_socket(path: &str) -> bool {
    NETWORK_RUNTIME.initialize();
    NETWORK_RUNTIME.with_mut(|state| {
        let Some(state) = state.as_mut() else {
            return false;
        };
        let before = state.sockets.len();
        state.sockets.retain(|socket| socket.path != path);
        before != state.sockets.len()
    })
}

pub fn configure_interface_ipv4(
    path: &str,
    addr: [u8; 4],
    netmask: [u8; 4],
    gateway: [u8; 4],
) -> Result<(), Errno> {
    NETWORK_RUNTIME.initialize();
    NETWORK_RUNTIME.with_mut(|state| {
        let state = state.as_mut().ok_or(Errno::NoEnt)?;
        let (_, interface) = interface_by_device_path_mut(state, path).ok_or(Errno::NoEnt)?;
        interface.ipv4_addr = addr;
        interface.ipv4_netmask = netmask;
        interface.ipv4_gateway = gateway;
        Ok(())
    })
}

#[allow(clippy::too_many_arguments)]
pub fn configure_interface_admin(
    path: &str,
    mtu: u64,
    tx_capacity: u64,
    rx_capacity: u64,
    tx_inflight_limit: u64,
    admin_up: bool,
    promiscuous: bool,
) -> Result<(), Errno> {
    NETWORK_RUNTIME.initialize();
    NETWORK_RUNTIME.with_mut(|state| {
        let state = state.as_mut().ok_or(Errno::NoEnt)?;
        let (_, interface) = interface_by_device_path_mut(state, path).ok_or(Errno::NoEnt)?;
        interface.mtu = mtu.max(576);
        interface.tx_capacity = tx_capacity.max(1);
        interface.rx_capacity = rx_capacity.max(1);
        interface.tx_inflight_limit = tx_inflight_limit.max(1);
        interface.admin_up = admin_up;
        interface.promiscuous = promiscuous;
        Ok(())
    })
}

pub fn set_link_state(path: &str, link_up: bool) -> Result<(), Errno> {
    NETWORK_RUNTIME.initialize();
    NETWORK_RUNTIME.with_mut(|state| {
        let state = state.as_mut().ok_or(Errno::NoEnt)?;
        let (_, interface) = interface_by_device_path_mut(state, path).ok_or(Errno::NoEnt)?;
        interface.link_up = link_up;
        Ok(())
    })
}

pub fn bind_udp_socket(
    socket_path: &str,
    device_path: &str,
    local_port: u16,
    remote_ipv4: [u8; 4],
    remote_port: u16,
) -> Result<(), Errno> {
    NETWORK_RUNTIME.initialize();
    NETWORK_RUNTIME.with_mut(|state| {
        let state = state.as_mut().ok_or(Errno::NoEnt)?;
        let (_, interface) =
            interface_by_device_path_mut(state, device_path).ok_or(Errno::NoEnt)?;
        let interface_addr = interface.ipv4_addr;
        if let Some(socket) = state
            .sockets
            .iter_mut()
            .find(|socket| socket.path == socket_path)
        {
            socket.device_path = device_path.to_string();
            socket.local_ipv4 = interface_addr;
            socket.remote_ipv4 = remote_ipv4;
            socket.local_port = local_port;
            socket.remote_port = remote_port;
            socket.connected = false;
            return Ok(());
        }
        state.sockets.push(BootNetworkSocket {
            path: socket_path.to_string(),
            device_path: device_path.to_string(),
            local_ipv4: interface_addr,
            remote_ipv4,
            local_port,
            remote_port,
            connected: false,
            rx_queue: VecDeque::new(),
            rx_limit: SOCKET_RX_LIMIT,
            tx_packets: 0,
            rx_packets: 0,
            dropped_packets: 0,
        });
        Ok(())
    })
}

pub fn connect_udp_socket(
    socket_path: &str,
    remote_ipv4: [u8; 4],
    remote_port: u16,
) -> Result<(), Errno> {
    NETWORK_RUNTIME.initialize();
    NETWORK_RUNTIME.with_mut(|state| {
        let state = state.as_mut().ok_or(Errno::NoEnt)?;
        let socket = state
            .sockets
            .iter_mut()
            .find(|socket| socket.path == socket_path)
            .ok_or(Errno::NoEnt)?;
        socket.remote_ipv4 = remote_ipv4;
        socket.remote_port = remote_port;
        socket.connected = true;
        Ok(())
    })
}

pub fn send_udp_to(
    socket_path: &str,
    remote_ipv4: [u8; 4],
    remote_port: u16,
    payload: &[u8],
) -> Result<(usize, u64), Errno> {
    NETWORK_RUNTIME.initialize();
    NETWORK_RUNTIME.with_mut(|state| {
        let state = state.as_mut().ok_or(Errno::NoEnt)?;
        let socket_index = state
            .sockets
            .iter()
            .position(|socket| socket.path == socket_path)
            .ok_or(Errno::NoEnt)?;
        let device_path = state.sockets[socket_index].device_path.clone();
        let interface_index = interface_index_from_device_path(&device_path).ok_or(Errno::NoEnt)?;
        let interface_snapshot = state
            .interfaces
            .get(interface_index)
            .ok_or(Errno::NoEnt)?
            .clone();
        if !interface_snapshot.admin_up || !interface_snapshot.link_up {
            state.interfaces[interface_index].tx_dropped = state.interfaces[interface_index]
                .tx_dropped
                .saturating_add(1);
            return Err(Errno::Access);
        }
        if interface_snapshot.in_flight_requests >= interface_snapshot.tx_inflight_limit {
            return Err(Errno::Again);
        }
        let socket_local_ipv4 = state.sockets[socket_index].local_ipv4;
        let socket_local_port = state.sockets[socket_index].local_port;
        let mut request_payload = Vec::new();
        request_payload.extend_from_slice(
            format!(
                "request:{}\nudp-send src={}:{} dst={}:{} bytes={}\n",
                state.last_request_id + 1,
                render_ipv4(socket_local_ipv4),
                socket_local_port,
                render_ipv4(remote_ipv4),
                remote_port,
                payload.len()
            )
            .as_bytes(),
        );
        request_payload.extend_from_slice(payload);
        state.last_request_id = state.last_request_id.saturating_add(1);
        let request_id = state.last_request_id;
        let submitted_tick = state.next_tick;
        state.next_tick = state.next_tick.saturating_add(1);
        let started_tick = state.next_tick;
        state.next_tick = state.next_tick.saturating_add(1);
        let mut frame_tag = [0; 64];
        copy_fixed(&mut frame_tag, &format!("net-tx-{request_id:04}"));
        let mut source_api_name = [0; 24];
        copy_fixed(&mut source_api_name, "native-udp");
        let mut translation_label = [0; 32];
        copy_fixed(&mut translation_label, "native-net");
        state.request_records.push(BootNetworkRequestRecord {
            request_id,
            issuer: NETWORK_BOOT_ISSUER,
            kind: NETWORK_REQUEST_KIND_WRITE,
            state: NETWORK_REQUEST_STATE_INFLIGHT,
            opcode: NETWORK_SEND_OPCODE,
            buffer_id: 0,
            payload_len: request_payload.len() as u64,
            response_len: 0,
            submitted_tick,
            started_tick,
            completed_tick: 0,
            frame_tag,
            source_api_name,
            translation_label,
        });
        state.driver_queue.push_back(BootNetworkQueueEntry {
            interface_index,
            request_id,
            payload: request_payload,
        });
        state.submitted_requests = state.submitted_requests.saturating_add(1);
        state.interfaces[interface_index].in_flight_requests = state.interfaces[interface_index]
            .in_flight_requests
            .saturating_add(1);
        state.interfaces[interface_index].last_payload_len = payload.len() as u64;
        state.interfaces[interface_index].tx_packets = state.interfaces[interface_index]
            .tx_packets
            .saturating_add(1);
        state.last_payload_len = payload.len() as u64;
        state.sockets[socket_index].tx_packets =
            state.sockets[socket_index].tx_packets.saturating_add(1);
        Ok((payload.len(), request_id))
    })
}

pub fn recv_udp_from(
    socket_path: &str,
    buffer: &mut [u8],
) -> Result<(usize, NativeUdpRecvMeta), Errno> {
    NETWORK_RUNTIME.initialize();
    NETWORK_RUNTIME.with_mut(|state| {
        let state = state.as_mut().ok_or(Errno::NoEnt)?;
        let socket = state
            .sockets
            .iter_mut()
            .find(|socket| socket.path == socket_path)
            .ok_or(Errno::NoEnt)?;
        let Some((remote_ipv4, remote_port, payload)) = socket.rx_queue.pop_front() else {
            return Err(Errno::Again);
        };
        let count = buffer.len().min(payload.len());
        buffer[..count].copy_from_slice(&payload[..count]);
        let meta = NativeUdpRecvMeta {
            remote_ipv4,
            remote_port,
            reserved: 0,
        };
        Ok((count, meta))
    })
}

pub fn complete_tx(driver_path: &str, completions: usize) -> Result<usize, Errno> {
    NETWORK_RUNTIME.initialize();
    NETWORK_RUNTIME.with_mut(|state| {
        let state = state.as_mut().ok_or(Errno::NoEnt)?;
        let interface_index = interface_index_from_driver_path(driver_path).ok_or(Errno::NoEnt)?;
        let mut completed = 0usize;
        while completed < completions {
            let Some(position) = state
                .driver_queue
                .iter()
                .position(|entry| entry.interface_index == interface_index)
            else {
                break;
            };
            let Some(entry) = state.driver_queue.remove(position) else {
                break;
            };
            let request_id = entry.request_id;
            let payload = entry.payload;
            let completed_tick = state.next_tick;
            state.next_tick = state.next_tick.saturating_add(1);
            if let Some(record) = state
                .request_records
                .iter_mut()
                .find(|record| record.request_id == request_id)
            {
                record.state = NETWORK_REQUEST_STATE_COMPLETED;
                record.completed_tick = completed_tick;
                record.response_len = payload.len() as u64;
                let frame_tag = record.frame_tag;
                let source_api_name = record.source_api_name;
                let translation_label = record.translation_label;
                state.last_completed_request_id = request_id;
                state.last_completed_frame_tag = frame_tag;
                state.last_completed_source_api_name = source_api_name;
                state.last_completed_translation_label = translation_label;
                update_terminal_metadata(
                    state,
                    request_id,
                    NETWORK_REQUEST_STATE_COMPLETED,
                    &frame_tag,
                    &source_api_name,
                    &translation_label,
                );
            }
            state.completed_requests = state.completed_requests.saturating_add(1);
            state.interfaces[interface_index].tx_completions = state.interfaces[interface_index]
                .tx_completions
                .saturating_add(1);
            state.interfaces[interface_index].in_flight_requests = state.interfaces
                [interface_index]
                .in_flight_requests
                .saturating_sub(1);
            state.completion_queue.push_back(BootNetworkQueueEntry {
                interface_index,
                request_id,
                payload: payload.clone(),
            });
            state.last_completion_payload = payload;
            completed += 1;
        }
        Ok(completed)
    })
}

pub fn poll(endpoint: NetworkEndpointKind, interest: u32) -> usize {
    NETWORK_RUNTIME.initialize();
    NETWORK_RUNTIME.with_mut(|state| {
        let Some(state) = state.as_mut() else {
            return 0;
        };
        let ready = match endpoint {
            NetworkEndpointKind::Device => {
                let mut ready = POLLOUT as usize;
                if !state.completion_queue.is_empty() {
                    ready |= POLLIN as usize;
                }
                ready
            }
            NetworkEndpointKind::Driver => {
                let mut ready = POLLOUT as usize;
                if !state.driver_queue.is_empty() {
                    ready |= POLLIN as usize;
                }
                ready
            }
        };
        ready & interest as usize
    })
}

pub fn read(
    endpoint: NetworkEndpointKind,
    buffer: *mut u8,
    len: usize,
    nonblock: bool,
) -> Result<usize, Errno> {
    if len == 0 {
        return Ok(0);
    }
    if buffer.is_null() {
        return Err(Errno::Fault);
    }
    NETWORK_RUNTIME.initialize();
    NETWORK_RUNTIME.with_mut(|state| {
        let state = state.as_mut().ok_or(Errno::NoEnt)?;
        let bytes = match endpoint {
            NetworkEndpointKind::Device => {
                let Some(entry) = state.completion_queue.pop_front() else {
                    return if nonblock { Err(Errno::Again) } else { Ok(0) };
                };
                entry.payload
            }
            NetworkEndpointKind::Driver => {
                let Some(entry) = state.driver_queue.front() else {
                    return if nonblock { Err(Errno::Again) } else { Ok(0) };
                };
                let request_id = entry.request_id;
                let mut bytes = format!("request:{request_id}\n").into_bytes();
                bytes.extend_from_slice(&entry.payload);
                if let Some(record) = state
                    .request_records
                    .iter_mut()
                    .find(|record| record.request_id == request_id)
                {
                    record.started_tick = state.next_tick;
                }
                bytes
            }
        };
        let count = len.min(bytes.len());
        unsafe {
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, count);
        }
        Ok(count)
    })
}

fn parse_request_header(bytes: &[u8]) -> Option<(u64, &[u8])> {
    let newline = bytes.iter().position(|byte| *byte == b'\n')?;
    let header = core::str::from_utf8(&bytes[..newline]).ok()?;
    let request_id = header
        .strip_prefix("complete-request:")?
        .parse::<u64>()
        .ok()?;
    Some((request_id, &bytes[newline + 1..]))
}

fn parse_udp_ipv4_frame(frame: &[u8]) -> Option<([u8; 4], u16, [u8; 4], u16, &[u8])> {
    if frame.len() < 42 {
        return None;
    }
    if frame[12] != 0x08 || frame[13] != 0x00 {
        return None;
    }
    let ip_start = 14usize;
    let version_ihl = frame[ip_start];
    if version_ihl >> 4 != 4 {
        return None;
    }
    let ihl = ((version_ihl & 0x0f) as usize) * 4;
    if frame.len() < ip_start + ihl + 8 {
        return None;
    }
    if frame[ip_start + 9] != 17 {
        return None;
    }
    let src_ip = [
        frame[ip_start + 12],
        frame[ip_start + 13],
        frame[ip_start + 14],
        frame[ip_start + 15],
    ];
    let dst_ip = [
        frame[ip_start + 16],
        frame[ip_start + 17],
        frame[ip_start + 18],
        frame[ip_start + 19],
    ];
    let udp_start = ip_start + ihl;
    let src_port = u16::from_be_bytes([frame[udp_start], frame[udp_start + 1]]);
    let dst_port = u16::from_be_bytes([frame[udp_start + 2], frame[udp_start + 3]]);
    let udp_len = u16::from_be_bytes([frame[udp_start + 4], frame[udp_start + 5]]) as usize;
    if udp_len < 8 || frame.len() < udp_start + udp_len {
        return None;
    }
    let payload = &frame[udp_start + 8..udp_start + udp_len];
    Some((src_ip, src_port, dst_ip, dst_port, payload))
}

fn queue_udp_rx_frame(
    state: &mut BootNetworkRuntimeState,
    src_ip: [u8; 4],
    src_port: u16,
    dst_ip: [u8; 4],
    dst_port: u16,
    payload: &[u8],
) -> Result<(), Errno> {
    let Some(socket_index) = state.sockets.iter().position(|socket| {
        if socket.local_port != dst_port {
            return false;
        }
        let Some(interface_index) = interface_index_from_device_path(&socket.device_path) else {
            return false;
        };
        let Some(interface) = state.interfaces.get(interface_index) else {
            return false;
        };
        socket.local_ipv4 == dst_ip
            || socket.local_ipv4 == [0, 0, 0, 0]
            || dst_ip == interface.ipv4_addr
    }) else {
        return Err(Errno::NoEnt);
    };
    let interface_index =
        interface_index_from_device_path(&state.sockets[socket_index].device_path)
            .ok_or(Errno::NoEnt)?;
    let interface = &mut state.interfaces[interface_index];
    let socket = &mut state.sockets[socket_index];
    if socket.rx_queue.len() >= socket.rx_limit {
        socket.dropped_packets = socket.dropped_packets.saturating_add(1);
        interface.rx_dropped = interface.rx_dropped.saturating_add(1);
        return Err(Errno::Again);
    }
    socket
        .rx_queue
        .push_back((src_ip, src_port, payload.to_vec()));
    socket.rx_packets = socket.rx_packets.saturating_add(1);
    interface.rx_packets = interface.rx_packets.saturating_add(1);
    Ok(())
}

pub fn ingest_udp_ipv4_frame(bytes: &[u8]) -> bool {
    NETWORK_RUNTIME.initialize();
    NETWORK_RUNTIME.with_mut(|state| {
        let Some(state) = state.as_mut() else {
            return false;
        };
        let Some((src_ip, src_port, dst_ip, dst_port, payload)) = parse_udp_ipv4_frame(bytes)
        else {
            return false;
        };
        queue_udp_rx_frame(state, src_ip, src_port, dst_ip, dst_port, payload).is_ok()
    })
}

pub fn write(endpoint: NetworkEndpointKind, bytes: &[u8]) -> Result<usize, Errno> {
    NETWORK_RUNTIME.initialize();
    NETWORK_RUNTIME.with_mut(|state| {
        let state = state.as_mut().ok_or(Errno::NoEnt)?;
        match endpoint {
            NetworkEndpointKind::Device => Err(Errno::Badf),
            NetworkEndpointKind::Driver => {
                if let Some((request_id, payload)) = parse_request_header(bytes) {
                    let completed_tick = state.next_tick;
                    state.next_tick = state.next_tick.saturating_add(1);
                    let Some(position) = state
                        .driver_queue
                        .iter()
                        .position(|entry| entry.request_id == request_id)
                    else {
                        return Err(Errno::NoEnt);
                    };
                    let Some(entry) = state.driver_queue.remove(position) else {
                        return Err(Errno::NoEnt);
                    };
                    if let Some(record) = state
                        .request_records
                        .iter_mut()
                        .find(|record| record.request_id == request_id)
                    {
                        record.state = NETWORK_REQUEST_STATE_COMPLETED;
                        record.completed_tick = completed_tick;
                        record.response_len = payload.len() as u64;
                        let frame_tag = record.frame_tag;
                        let source_api_name = record.source_api_name;
                        let translation_label = record.translation_label;
                        state.last_completed_request_id = request_id;
                        state.last_completed_frame_tag = frame_tag;
                        state.last_completed_source_api_name = source_api_name;
                        state.last_completed_translation_label = translation_label;
                        update_terminal_metadata(
                            state,
                            request_id,
                            NETWORK_REQUEST_STATE_COMPLETED,
                            &frame_tag,
                            &source_api_name,
                            &translation_label,
                        );
                    }
                    state.completed_requests = state.completed_requests.saturating_add(1);
                    let interface = &mut state.interfaces[entry.interface_index];
                    interface.tx_completions = interface.tx_completions.saturating_add(1);
                    interface.in_flight_requests = interface.in_flight_requests.saturating_sub(1);
                    state.completion_queue.push_back(BootNetworkQueueEntry {
                        interface_index: entry.interface_index,
                        request_id,
                        payload: payload.to_vec(),
                    });
                    state.last_completion_payload = payload.to_vec();
                    return Ok(bytes.len());
                }
                let Some((src_ip, src_port, dst_ip, dst_port, payload)) =
                    parse_udp_ipv4_frame(bytes)
                else {
                    return Err(Errno::Inval);
                };
                queue_udp_rx_frame(state, src_ip, src_port, dst_ip, dst_port, payload)?;
                Ok(bytes.len())
            }
        }
    })
}

fn render_ipv4(addr: [u8; 4]) -> String {
    alloc::format!("{}.{}.{}.{}", addr[0], addr[1], addr[2], addr[3])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    fn lock_test_state() -> MutexGuard<'static, ()> {
        static TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
        TEST_MUTEX
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap()
    }

    #[test]
    fn interface_records_keep_socket_counts_scoped_per_device() {
        let _guard = lock_test_state();
        reset();
        configure_interface_ipv4(
            "/dev/net0",
            [10, 1, 0, 2],
            [255, 255, 255, 0],
            [10, 1, 0, 1],
        )
        .unwrap();
        configure_interface_ipv4(
            "/dev/net1",
            [10, 2, 0, 2],
            [255, 255, 255, 0],
            [10, 2, 0, 1],
        )
        .unwrap();
        bind_udp_socket("/run/net0.sock", "/dev/net0", 4000, [0, 0, 0, 0], 0).unwrap();
        bind_udp_socket("/run/net1.sock", "/dev/net1", 4100, [0, 0, 0, 0], 0).unwrap();

        let net0 = interface_record("/dev/net0").unwrap();
        let net1 = interface_record("/dev/net1").unwrap();

        assert_eq!(net0.attached_socket_count, 1);
        assert_eq!(net1.attached_socket_count, 1);
        assert_eq!(attached_socket_count("/dev/net0"), 1);
        assert_eq!(attached_socket_count("/dev/net1"), 1);
    }

    #[test]
    fn tx_completion_stays_scoped_to_selected_driver_path() {
        let _guard = lock_test_state();
        reset();
        configure_interface_ipv4(
            "/dev/net0",
            [10, 1, 0, 2],
            [255, 255, 255, 0],
            [10, 1, 0, 1],
        )
        .unwrap();
        configure_interface_ipv4(
            "/dev/net1",
            [10, 2, 0, 2],
            [255, 255, 255, 0],
            [10, 2, 0, 1],
        )
        .unwrap();
        configure_interface_admin("/dev/net0", 1500, 4, 4, 2, true, false).unwrap();
        configure_interface_admin("/dev/net1", 1500, 4, 4, 2, true, false).unwrap();
        bind_udp_socket("/run/net0.sock", "/dev/net0", 4000, [0, 0, 0, 0], 0).unwrap();
        bind_udp_socket("/run/net1.sock", "/dev/net1", 4100, [0, 0, 0, 0], 0).unwrap();

        send_udp_to("/run/net0.sock", [10, 1, 0, 9], 5000, b"net0").unwrap();
        send_udp_to("/run/net1.sock", [10, 2, 0, 9], 5100, b"net1").unwrap();

        assert_eq!(complete_tx("/drv/net1", 1).unwrap(), 1);

        let net0 = interface_record("/dev/net0").unwrap();
        let net1 = interface_record("/dev/net1").unwrap();
        assert_eq!(net0.tx_completions, 0);
        assert_eq!(net1.tx_completions, 1);
        assert_eq!(net0.tx_ring_depth, 1);
        assert_eq!(net1.tx_ring_depth, 0);
    }
}
