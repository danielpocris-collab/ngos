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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NetworkSocket {
    pub(crate) path: String,
    pub(crate) owner: ProcessId,
    pub(crate) interface: String,
    pub(crate) local_ipv4: [u8; 4],
    pub(crate) remote_ipv4: [u8; 4],
    pub(crate) local_port: u16,
    pub(crate) remote_port: u16,
    pub(crate) rx_queue: Vec<SocketRxPacket>,
    pub(crate) rx_queue_limit: usize,
    pub(crate) connected: bool,
    pub(crate) tx_packets: u64,
    pub(crate) rx_packets: u64,
    pub(crate) dropped_packets: u64,
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
