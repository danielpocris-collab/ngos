//! Canonical subsystem role:
//! - subsystem: device and driver object model
//! - owner layer: Layer 1
//! - semantic owner: `kernel-core`
//! - truth path role: canonical device, driver, and request object model for
//!   the kernel
//!
//! Canonical contract families defined here:
//! - device object contracts
//! - driver object contracts
//! - device request state contracts
//! - network/socket object contracts
//!
//! This module may define canonical device/runtime object truth. Higher layers
//! may observe it, but they must not redefine it.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeviceRequest {
    pub(crate) id: u64,
    pub(crate) device_path: String,
    pub(crate) driver_path: String,
    pub(crate) issuer: ProcessId,
    pub(crate) kind: DeviceRequestKind,
    pub(crate) state: DeviceRequestState,
    pub(crate) opcode: Option<u32>,
    pub(crate) graphics_buffer_id: Option<u64>,
    pub(crate) graphics_buffer_len: Option<usize>,
    pub(crate) payload: Vec<u8>,
    pub(crate) response: Vec<u8>,
    pub(crate) submitted_tick: u64,
    pub(crate) started_tick: Option<u64>,
    pub(crate) completed_tick: Option<u64>,
    pub(crate) frame_tag: String,
    pub(crate) source_api_name: String,
    pub(crate) translation_label: String,
}

impl DeviceRequest {
    pub(crate) fn info(&self) -> DeviceRequestInfo {
        DeviceRequestInfo {
            id: self.id,
            device_path: self.device_path.clone(),
            driver_path: self.driver_path.clone(),
            issuer: self.issuer,
            kind: self.kind,
            state: self.state,
            opcode: self.opcode,
            graphics_buffer_id: self.graphics_buffer_id,
            graphics_buffer_len: self.graphics_buffer_len,
            payload_len: self.graphics_buffer_len.unwrap_or(self.payload.len()),
            response_len: self.response.len(),
            submitted_tick: self.submitted_tick,
            started_tick: self.started_tick,
            completed_tick: self.completed_tick,
            frame_tag: self.frame_tag.clone(),
            source_api_name: self.source_api_name.clone(),
            translation_label: self.translation_label.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DriverEndpoint {
    pub(crate) path: String,
    pub(crate) owner: ProcessId,
    pub(crate) capability: CapabilityId,
    pub(crate) state: DriverState,
    pub(crate) bound_devices: Vec<String>,
    pub(crate) queued_requests: Vec<u64>,
    pub(crate) in_flight_requests: Vec<u64>,
    pub(crate) completed_requests: u64,
    pub(crate) last_completed_request_id: u64,
    pub(crate) last_completed_frame_tag: String,
    pub(crate) last_completed_source_api_name: String,
    pub(crate) last_completed_translation_label: String,
    pub(crate) last_terminal_request_id: u64,
    pub(crate) last_terminal_state: DeviceRequestState,
    pub(crate) last_terminal_frame_tag: String,
    pub(crate) last_terminal_source_api_name: String,
    pub(crate) last_terminal_translation_label: String,
}

impl DriverEndpoint {
    fn info(&self) -> DriverInfo {
        DriverInfo {
            path: self.path.clone(),
            owner: self.owner,
            state: self.state,
            capability: self.capability,
            bound_devices: self.bound_devices.clone(),
            queued_requests: self.queued_requests.len(),
            in_flight_requests: self.in_flight_requests.len(),
            completed_requests: self.completed_requests,
            last_completed_request_id: self.last_completed_request_id,
            last_completed_frame_tag: self.last_completed_frame_tag.clone(),
            last_completed_source_api_name: self.last_completed_source_api_name.clone(),
            last_completed_translation_label: self.last_completed_translation_label.clone(),
            last_terminal_request_id: self.last_terminal_request_id,
            last_terminal_state: self.last_terminal_state,
            last_terminal_frame_tag: self.last_terminal_frame_tag.clone(),
            last_terminal_source_api_name: self.last_terminal_source_api_name.clone(),
            last_terminal_translation_label: self.last_terminal_translation_label.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeviceEndpoint {
    pub(crate) path: String,
    pub(crate) owner: ProcessId,
    pub(crate) capability: CapabilityId,
    pub(crate) class: DeviceClass,
    pub(crate) state: DeviceState,
    pub(crate) driver: Option<String>,
    pub(crate) queue_capacity: usize,
    pub(crate) pending_requests: Vec<u64>,
    pub(crate) completion_queue: Vec<u64>,
    pub(crate) graphics_control_reserve_armed: bool,
    pub(crate) graphics_presented_frames: u64,
    pub(crate) graphics_last_presented_frame: Vec<u8>,
    pub(crate) submitted_requests: u64,
    pub(crate) completed_requests: u64,
    pub(crate) last_completed_request_id: u64,
    pub(crate) last_completed_frame_tag: String,
    pub(crate) last_completed_source_api_name: String,
    pub(crate) last_completed_translation_label: String,
    pub(crate) last_terminal_request_id: u64,
    pub(crate) last_terminal_state: DeviceRequestState,
    pub(crate) last_terminal_frame_tag: String,
    pub(crate) last_terminal_source_api_name: String,
    pub(crate) last_terminal_translation_label: String,
    pub(crate) total_latency_ticks: u64,
    pub(crate) max_latency_ticks: u64,
    pub(crate) total_queue_wait_ticks: u64,
    pub(crate) max_queue_wait_ticks: u64,
    pub(crate) link_up: bool,
    pub(crate) block_size: u32,
    pub(crate) capacity_bytes: u64,
}

impl DeviceEndpoint {
    fn info(&self) -> DeviceInfo {
        DeviceInfo {
            path: self.path.clone(),
            owner: self.owner,
            class: self.class,
            state: self.state,
            capability: self.capability,
            driver: self.driver.clone(),
            queue_depth: self.pending_requests.len(),
            queue_capacity: self.queue_capacity,
            submitted_requests: self.submitted_requests,
            completed_requests: self.completed_requests,
            last_completed_request_id: self.last_completed_request_id,
            last_completed_frame_tag: self.last_completed_frame_tag.clone(),
            last_completed_source_api_name: self.last_completed_source_api_name.clone(),
            last_completed_translation_label: self.last_completed_translation_label.clone(),
            last_terminal_request_id: self.last_terminal_request_id,
            last_terminal_state: self.last_terminal_state,
            last_terminal_frame_tag: self.last_terminal_frame_tag.clone(),
            last_terminal_source_api_name: self.last_terminal_source_api_name.clone(),
            last_terminal_translation_label: self.last_terminal_translation_label.clone(),
            total_latency_ticks: self.total_latency_ticks,
            max_latency_ticks: self.max_latency_ticks,
            total_queue_wait_ticks: self.total_queue_wait_ticks,
            max_queue_wait_ticks: self.max_queue_wait_ticks,
            link_up: self.link_up,
            block_size: self.block_size,
            capacity_bytes: self.capacity_bytes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeviceRegistry {
    pub(crate) drivers: Vec<DriverEndpoint>,
    pub(crate) devices: Vec<DeviceEndpoint>,
    pub(crate) requests: Vec<DeviceRequest>,
    pub(crate) gpu_buffers: Vec<GpuBufferObject>,
    pub(crate) next_request_id: u64,
    pub(crate) next_gpu_buffer_id: u64,
}

impl DeviceRegistry {
    pub(crate) fn new() -> Self {
        Self {
            drivers: Vec::new(),
            devices: Vec::new(),
            requests: Vec::new(),
            gpu_buffers: Vec::new(),
            next_request_id: 1,
            next_gpu_buffer_id: 1,
        }
    }

    pub(crate) fn driver_info(&self, path: &str) -> Result<DriverInfo, DeviceModelError> {
        self.drivers
            .iter()
            .find(|driver| driver.path == path)
            .map(DriverEndpoint::info)
            .ok_or(DeviceModelError::InvalidDriver)
    }

    pub(crate) fn device_info(&self, path: &str) -> Result<DeviceInfo, DeviceModelError> {
        self.devices
            .iter()
            .find(|device| device.path == path)
            .map(DeviceEndpoint::info)
            .ok_or(DeviceModelError::InvalidDevice)
    }

    pub(crate) fn request_info(
        &self,
        request_id: u64,
    ) -> Result<DeviceRequestInfo, DeviceModelError> {
        self.requests
            .iter()
            .find(|request| request.id == request_id)
            .map(DeviceRequest::info)
            .ok_or(DeviceModelError::RequestNotFound)
    }

    pub(crate) fn gpu_buffer_info(
        &self,
        buffer_id: u64,
    ) -> Result<GpuBufferInfo, DeviceModelError> {
        self.gpu_buffers
            .iter()
            .find(|buffer| buffer.id == buffer_id)
            .map(GpuBufferObject::info)
            .ok_or(DeviceModelError::RequestNotFound)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GpuBufferObject {
    pub(crate) id: u64,
    pub(crate) owner: ProcessId,
    pub(crate) length: usize,
    pub(crate) used_len: usize,
    pub(crate) busy: bool,
    pub(crate) bytes: Vec<u8>,
}

impl GpuBufferObject {
    fn info(&self) -> GpuBufferInfo {
        GpuBufferInfo {
            id: self.id,
            owner: self.owner,
            length: self.length,
            used_len: self.used_len,
            busy: self.busy,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TcpControlBlock {
    pub(crate) state: TcpState,
    pub(crate) local_seq: u32,
    pub(crate) remote_seq: u32,
    pub(crate) local_ack: u32,
    pub(crate) remote_ack: u32,
    pub(crate) local_window: u32,
    pub(crate) remote_window: u32,
    pub(crate) local_port: u16,
    pub(crate) remote_port: u16,
    pub(crate) listen_backlog: usize,
    pub(crate) accept_queue: Vec<u64>,
    pub(crate) retransmit_timeout_ticks: u64,
    pub(crate) last_transmit_tick: Option<u64>,
    pub(crate) unacked_segments: Vec<TcpSegment>,
    pub(crate) ooo_queue: Vec<TcpSegment>,
    pub(crate) rtt_estimate_ticks: u64,
    pub(crate) rtt_variance_ticks: u64,
    pub(crate) congestion_window: u32,
    pub(crate) slow_start_threshold: u32,
    pub(crate) duplicate_acks: u32,
}

impl TcpControlBlock {
    pub(crate) fn new_listen(local_port: u16, backlog: usize) -> Self {
        Self {
            state: TcpState::Listen,
            local_seq: 0,
            remote_seq: 0,
            local_ack: 0,
            remote_ack: 0,
            local_window: 65535,
            remote_window: 65535,
            local_port,
            remote_port: 0,
            listen_backlog: backlog,
            accept_queue: Vec::new(),
            retransmit_timeout_ticks: 100,
            last_transmit_tick: None,
            unacked_segments: Vec::new(),
            ooo_queue: Vec::new(),
            rtt_estimate_ticks: 0,
            rtt_variance_ticks: 0,
            congestion_window: 1,
            slow_start_threshold: 65535,
            duplicate_acks: 0,
        }
    }

    pub(crate) fn new_init(local_port: u16, remote_port: u16) -> Self {
        Self {
            state: TcpState::Closed,
            local_seq: 0,
            remote_seq: 0,
            local_ack: 0,
            remote_ack: 0,
            local_window: 65535,
            remote_window: 65535,
            local_port,
            remote_port,
            listen_backlog: 0,
            accept_queue: Vec::new(),
            retransmit_timeout_ticks: 100,
            last_transmit_tick: None,
            unacked_segments: Vec::new(),
            ooo_queue: Vec::new(),
            rtt_estimate_ticks: 0,
            rtt_variance_ticks: 0,
            congestion_window: 1,
            slow_start_threshold: 65535,
            duplicate_acks: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TcpSegment {
    pub(crate) seq: u32,
    pub(crate) ack: u32,
    pub(crate) window: u32,
    pub(crate) flags: TcpFlags,
    pub(crate) payload: Vec<u8>,
    pub(crate) local_port: u16,
    pub(crate) remote_port: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TcpFlags {
    pub(crate) fin: bool,
    pub(crate) syn: bool,
    pub(crate) rst: bool,
    pub(crate) psh: bool,
    pub(crate) ack: bool,
    pub(crate) urg: bool,
}

impl TcpFlags {
    pub(crate) fn to_u8(&self) -> u8 {
        let mut flags = 0u8;
        if self.urg {
            flags |= 0x20;
        }
        if self.ack {
            flags |= 0x10;
        }
        if self.psh {
            flags |= 0x08;
        }
        if self.rst {
            flags |= 0x04;
        }
        if self.syn {
            flags |= 0x02;
        }
        if self.fin {
            flags |= 0x01;
        }
        flags
    }

    pub(crate) fn from_u8(flags: u8) -> Self {
        Self {
            fin: flags & 0x01 != 0,
            syn: flags & 0x02 != 0,
            rst: flags & 0x04 != 0,
            psh: flags & 0x08 != 0,
            ack: flags & 0x10 != 0,
            urg: flags & 0x20 != 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NetworkSocket {
    pub(crate) path: String,
    pub(crate) owner: ProcessId,
    pub(crate) interface: String,
    pub(crate) ip_version: IpVersion,
    pub(crate) local_ipv4: [u8; 4],
    pub(crate) local_ipv6: Ipv6Address,
    pub(crate) remote_ipv4: [u8; 4],
    pub(crate) remote_ipv6: Ipv6Address,
    pub(crate) local_port: u16,
    pub(crate) remote_port: u16,
    pub(crate) rx_queue: Vec<SocketRxPacket>,
    pub(crate) rx_queue_limit: usize,
    pub(crate) connected: bool,
    pub(crate) tx_packets: u64,
    pub(crate) rx_packets: u64,
    pub(crate) dropped_packets: u64,
    pub(crate) socket_type: SocketType,
    pub(crate) tcp_state: Option<TcpControlBlock>,
}

impl NetworkSocket {
    pub(crate) fn info(&self) -> NetworkSocketInfo {
        NetworkSocketInfo {
            path: self.path.clone(),
            owner: self.owner,
            interface: self.interface.clone(),
            local_ipv4: self.local_ipv4,
            remote_ipv4: self.remote_ipv4,
            local_port: self.local_port,
            remote_port: self.remote_port,
            rx_depth: self.rx_queue.len(),
            rx_queue_limit: self.rx_queue_limit,
            connected: self.connected,
            tx_packets: self.tx_packets,
            rx_packets: self.rx_packets,
            dropped_packets: self.dropped_packets,
            socket_type: self.socket_type,
            tcp_state: self.tcp_state.as_ref().map(|tcb| tcb.state),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SocketRxPacket {
    pub(crate) buffer_id: u64,
    pub(crate) src_ipv4: [u8; 4],
    pub(crate) dst_ipv4: [u8; 4],
    pub(crate) src_port: u16,
    pub(crate) dst_port: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Ipv6Address {
    pub(crate) octets: [u8; 16],
}

impl Ipv6Address {
    pub(crate) const UNSPECIFIED: Self = Self { octets: [0; 16] };
    pub(crate) const LOOPBACK: Self = Self {
        octets: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    };

    pub(crate) fn from_segments(a: u16, b: u16, c: u16, d: u16, e: u16, f: u16, g: u16, h: u16) -> Self {
        let mut octets = [0u8; 16];
        octets[0..2].copy_from_slice(&a.to_be_bytes());
        octets[2..4].copy_from_slice(&b.to_be_bytes());
        octets[4..6].copy_from_slice(&c.to_be_bytes());
        octets[6..8].copy_from_slice(&d.to_be_bytes());
        octets[8..10].copy_from_slice(&e.to_be_bytes());
        octets[10..12].copy_from_slice(&f.to_be_bytes());
        octets[12..14].copy_from_slice(&g.to_be_bytes());
        octets[14..16].copy_from_slice(&h.to_be_bytes());
        Self { octets }
    }

    pub(crate) fn from_ipv4_mapped(ipv4: [u8; 4]) -> Self {
        let mut octets = [0u8; 16];
        octets[10] = 0xff;
        octets[11] = 0xff;
        octets[12] = ipv4[0];
        octets[13] = ipv4[1];
        octets[14] = ipv4[2];
        octets[15] = ipv4[3];
        Self { octets }
    }

    pub(crate) fn is_ipv4_mapped(&self) -> bool {
        self.octets[10] == 0xff && self.octets[11] == 0xff
    }

    pub(crate) fn to_ipv4_mapped(&self) -> Option<[u8; 4]> {
        if self.is_ipv4_mapped() {
            Some([self.octets[12], self.octets[13], self.octets[14], self.octets[15]])
        } else {
            None
        }
    }

    pub(crate) fn is_link_local(&self) -> bool {
        self.octets[0] == 0xfe && (self.octets[1] & 0xc0) == 0x80
    }

    pub(crate) fn is_multicast(&self) -> bool {
        self.octets[0] == 0xff
    }

    pub(crate) fn is_loopback(&self) -> bool {
        *self == Self::LOOPBACK
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IpVersion {
    Ipv4,
    Ipv6,
}

impl IpVersion {
    pub(crate) fn to_u8(self) -> u8 {
        match self {
            Self::Ipv4 => 4,
            Self::Ipv6 => 6,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IpAddress {
    V4([u8; 4]),
    V6(Ipv6Address),
}

impl IpAddress {
    pub(crate) fn version(&self) -> IpVersion {
        match self {
            Self::V4(_) => IpVersion::Ipv4,
            Self::V6(_) => IpVersion::Ipv6,
        }
    }

    pub(crate) fn is_unspecified(&self) -> bool {
        match self {
            Self::V4(addr) => *addr == [0, 0, 0, 0],
            Self::V6(addr) => *addr == Ipv6Address::UNSPECIFIED,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IcmpType {
    EchoReply = 0,
    DestinationUnreachable = 3,
    SourceQuench = 4,
    Redirect = 5,
    EchoRequest = 8,
    TimeExceeded = 11,
    ParameterProblem = 12,
    TimestampRequest = 13,
    TimestampReply = 14,
}

impl IcmpType {
    pub(crate) fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::EchoReply),
            3 => Some(Self::DestinationUnreachable),
            4 => Some(Self::SourceQuench),
            5 => Some(Self::Redirect),
            8 => Some(Self::EchoRequest),
            11 => Some(Self::TimeExceeded),
            12 => Some(Self::ParameterProblem),
            13 => Some(Self::TimestampRequest),
            14 => Some(Self::TimestampReply),
            _ => None,
        }
    }

    pub(crate) fn to_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IcmpUnreachableCode {
    NetworkUnreachable = 0,
    HostUnreachable = 1,
    ProtocolUnreachable = 2,
    PortUnreachable = 3,
    FragmentationNeeded = 4,
    SourceRouteFailed = 5,
}

impl IcmpUnreachableCode {
    pub(crate) fn to_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IcmpMessage {
    pub(crate) icmp_type: IcmpType,
    pub(crate) code: u8,
    pub(crate) checksum: u16,
    pub(crate) identifier: u16,
    pub(crate) sequence: u16,
    pub(crate) payload: Vec<u8>,
}

impl IcmpMessage {
    pub(crate) fn echo_request(identifier: u16, sequence: u16, payload: Vec<u8>) -> Self {
        Self {
            icmp_type: IcmpType::EchoRequest,
            code: 0,
            checksum: 0,
            identifier,
            sequence,
            payload,
        }
    }

    pub(crate) fn echo_reply(identifier: u16, sequence: u16, payload: Vec<u8>) -> Self {
        Self {
            icmp_type: IcmpType::EchoReply,
            code: 0,
            checksum: 0,
            identifier,
            sequence,
            payload,
        }
    }

    pub(crate) fn dest_unreachable(
        code: IcmpUnreachableCode,
        original_header: Vec<u8>,
    ) -> Self {
        Self {
            icmp_type: IcmpType::DestinationUnreachable,
            code: code.to_u8(),
            checksum: 0,
            identifier: 0,
            sequence: 0,
            payload: original_header,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SocketType {
    Udp,
    Tcp,
    Icmp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NetworkBufferState {
    Free,
    TxQueued,
    TxInFlight,
    SocketQueued,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NetworkBuffer {
    pub(crate) id: u64,
    pub(crate) source_socket: String,
    pub(crate) payload: Vec<u8>,
    pub(crate) state: NetworkBufferState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NetworkInterface {
    pub(crate) device_path: String,
    pub(crate) driver_path: String,
    pub(crate) admin_up: bool,
    pub(crate) link_up: bool,
    pub(crate) promiscuous: bool,
    pub(crate) mtu: usize,
    pub(crate) mac: [u8; 6],
    pub(crate) ipv4_addr: [u8; 4],
    pub(crate) ipv4_netmask: [u8; 4],
    pub(crate) ipv4_gateway: [u8; 4],
    pub(crate) ipv6_addr: Ipv6Address,
    pub(crate) ipv6_gateway: Ipv6Address,
    pub(crate) ipv6_prefix_len: u8,
    pub(crate) link_local: Ipv6Address,
    pub(crate) tx_capacity: usize,
    pub(crate) rx_capacity: usize,
    pub(crate) tx_inflight_limit: usize,
    pub(crate) tx_ring: Vec<u64>,
    pub(crate) tx_in_flight: Vec<u64>,
    pub(crate) rx_ring: Vec<u64>,
    pub(crate) free_buffers: Vec<u64>,
    pub(crate) buffers: Vec<NetworkBuffer>,
    pub(crate) next_buffer_id: u64,
    pub(crate) tx_packets: u64,
    pub(crate) rx_packets: u64,
    pub(crate) tx_completions: u64,
    pub(crate) tx_dropped: u64,
    pub(crate) rx_dropped: u64,
    pub(crate) attached_sockets: Vec<String>,
}

impl NetworkInterface {
    pub(crate) fn info(&self) -> NetworkInterfaceInfo {
        NetworkInterfaceInfo {
            device_path: self.device_path.clone(),
            driver_path: self.driver_path.clone(),
            admin_up: self.admin_up,
            link_up: self.link_up,
            promiscuous: self.promiscuous,
            mtu: self.mtu,
            mac: self.mac,
            ipv4_addr: self.ipv4_addr,
            ipv4_netmask: self.ipv4_netmask,
            ipv4_gateway: self.ipv4_gateway,
            rx_ring_depth: self.rx_ring.len(),
            tx_ring_depth: self.tx_ring.len(),
            tx_inflight_depth: self.tx_in_flight.len(),
            free_buffer_count: self.free_buffers.len(),
            tx_capacity: self.tx_capacity,
            rx_capacity: self.rx_capacity,
            tx_inflight_limit: self.tx_inflight_limit,
            tx_packets: self.tx_packets,
            rx_packets: self.rx_packets,
            tx_completions: self.tx_completions,
            tx_dropped: self.tx_dropped,
            rx_dropped: self.rx_dropped,
            attached_sockets: self.attached_sockets.clone(),
        }
    }
}
