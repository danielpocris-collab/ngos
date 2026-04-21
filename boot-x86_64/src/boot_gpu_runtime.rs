extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

use ngos_user_abi::{
    Errno, NativeDeviceRecord, NativeDeviceRequestRecord, NativeDriverRecord,
    NativeGpuDisplayRecord, NativeGpuScanoutRecord, POLLIN, POLLOUT,
};

pub const GPU_DEVICE_PATH: &str = "/dev/gpu0";
pub const GPU_DRIVER_PATH: &str = "/drv/gpu0";

const GPU_DEVICE_CLASS: u32 = 3;
const GPU_DEVICE_STATE_REGISTERED: u32 = 0;
const GPU_DRIVER_STATE_ACTIVE: u32 = 1;
const GPU_QUEUE_CAPACITY: u64 = 128;
const GPU_BOOT_ISSUER: u64 = 1;
const GPU_REQUEST_KIND_CONTROL: u32 = 2;
const GPU_PRESENT_OPCODE: u64 = 0x4750_0001;
const GPU_REQUEST_STATE_INFLIGHT: u32 = 1;
const GPU_REQUEST_STATE_COMPLETED: u32 = 2;
const GPU_REQUEST_STATE_FAILED: u32 = 3;
const GPU_REQUEST_STATE_CANCELED: u32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuEndpointKind {
    Device,
    Driver,
}

#[derive(Debug, Clone)]
struct BootGpuRequestRecord {
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

#[derive(Debug)]
struct BootGpuRuntimeState {
    driver_queue: VecDeque<(u64, Vec<u8>)>,
    completion_queue: VecDeque<Vec<u8>>,
    request_records: Vec<BootGpuRequestRecord>,
    submitted_requests: u64,
    completed_requests: u64,
    in_flight_requests: u64,
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
    presented_frames: u64,
    last_frame_len: u64,
    last_scanout_frame: Vec<u8>,
    last_scanout_frame_tag: [u8; 64],
    last_scanout_source_api_name: [u8; 24],
    last_scanout_translation_label: [u8; 32],
    display_present: bool,
    active_pipes: u32,
    planned_frames: u64,
    last_present_offset: u64,
    last_present_len: u64,
    hardware_programming_confirmed: bool,
}

impl Default for BootGpuRuntimeState {
    fn default() -> Self {
        Self {
            driver_queue: VecDeque::new(),
            completion_queue: VecDeque::new(),
            request_records: Vec::new(),
            submitted_requests: 0,
            completed_requests: 0,
            in_flight_requests: 0,
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
            presented_frames: 0,
            last_frame_len: 0,
            last_scanout_frame: Vec::new(),
            last_scanout_frame_tag: [0; 64],
            last_scanout_source_api_name: [0; 24],
            last_scanout_translation_label: [0; 32],
            display_present: false,
            active_pipes: 0,
            planned_frames: 0,
            last_present_offset: 0,
            last_present_len: 0,
            hardware_programming_confirmed: false,
        }
    }
}

struct BootGpuRuntimeCell {
    locked: AtomicBool,
    state: UnsafeCell<Option<BootGpuRuntimeState>>,
}

unsafe impl Sync for BootGpuRuntimeCell {}

impl BootGpuRuntimeCell {
    const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
            state: UnsafeCell::new(None),
        }
    }

    fn initialize(&self) {
        self.with_mut(|state| {
            if state.is_none() {
                *state = Some(BootGpuRuntimeState::default());
            }
        });
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut Option<BootGpuRuntimeState>) -> R) -> R {
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

static GPU_RUNTIME: BootGpuRuntimeCell = BootGpuRuntimeCell::new();

pub fn reset() {
    GPU_RUNTIME.with_mut(|state| {
        *state = Some(BootGpuRuntimeState::default());
    });
}

pub fn endpoint_for_path(path: &str) -> Option<GpuEndpointKind> {
    match path {
        GPU_DEVICE_PATH => Some(GpuEndpointKind::Device),
        GPU_DRIVER_PATH => Some(GpuEndpointKind::Driver),
        _ => None,
    }
}

pub fn device_record(path: &str) -> Option<NativeDeviceRecord> {
    if path != GPU_DEVICE_PATH {
        return None;
    }
    GPU_RUNTIME.initialize();
    GPU_RUNTIME.with_mut(|state| {
        let state = state.as_mut()?;
        Some(NativeDeviceRecord {
            class: GPU_DEVICE_CLASS,
            state: GPU_DEVICE_STATE_REGISTERED,
            reserved0: 0,
            queue_depth: state.driver_queue.len() as u64,
            queue_capacity: GPU_QUEUE_CAPACITY,
            submitted_requests: state.submitted_requests,
            completed_requests: state.completed_requests,
            total_latency_ticks: 0,
            max_latency_ticks: 0,
            total_queue_wait_ticks: 0,
            max_queue_wait_ticks: 0,
            link_up: 1,
            reserved1: 0,
            block_size: 0,
            reserved2: 0,
            capacity_bytes: state.last_frame_len,
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
    if path != GPU_DRIVER_PATH {
        return None;
    }
    GPU_RUNTIME.initialize();
    GPU_RUNTIME.with_mut(|state| {
        let state = state.as_mut()?;
        Some(NativeDriverRecord {
            state: GPU_DRIVER_STATE_ACTIVE,
            reserved: 0,
            bound_device_count: 1,
            queued_requests: state.driver_queue.len() as u64,
            in_flight_requests: state.in_flight_requests,
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
    GPU_RUNTIME.initialize();
    GPU_RUNTIME.with_mut(|state| {
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

pub fn gpu_display_record(path: &str) -> Option<NativeGpuDisplayRecord> {
    if path != GPU_DEVICE_PATH {
        return None;
    }
    GPU_RUNTIME.initialize();
    GPU_RUNTIME.with_mut(|state| {
        let state = state.as_mut()?;
        Some(NativeGpuDisplayRecord {
            present: u32::from(state.display_present),
            active_pipes: state.active_pipes,
            planned_frames: state.planned_frames,
            last_present_offset: state.last_present_offset,
            last_present_len: state.last_present_len,
            hardware_programming_confirmed: u32::from(state.hardware_programming_confirmed),
        })
    })
}

pub fn gpu_scanout_record(path: &str) -> Option<NativeGpuScanoutRecord> {
    if path != GPU_DEVICE_PATH {
        return None;
    }
    GPU_RUNTIME.initialize();
    GPU_RUNTIME.with_mut(|state| {
        let state = state.as_mut()?;
        Some(NativeGpuScanoutRecord {
            presented_frames: state.presented_frames,
            last_frame_len: state.last_frame_len,
            last_frame_tag: state.last_scanout_frame_tag,
            last_source_api_name: state.last_scanout_source_api_name,
            last_translation_label: state.last_scanout_translation_label,
        })
    })
}

pub fn read_scanout_frame(path: &str, buffer: *mut u8, len: usize) -> Result<usize, Errno> {
    if path != GPU_DEVICE_PATH {
        return Err(Errno::NoEnt);
    }
    if buffer.is_null() {
        return Err(Errno::Fault);
    }
    GPU_RUNTIME.initialize();
    GPU_RUNTIME.with_mut(|state| {
        let state = state.as_mut().ok_or(Errno::NoEnt)?;
        let count = state.last_scanout_frame.len().min(len);
        unsafe {
            core::ptr::copy_nonoverlapping(state.last_scanout_frame.as_ptr(), buffer, count);
        }
        Ok(count)
    })
}

pub fn present_frame(path: &str, bytes: &[u8]) -> Result<u64, Errno> {
    if path != GPU_DEVICE_PATH {
        return Err(Errno::NoEnt);
    }
    GPU_RUNTIME.initialize();
    GPU_RUNTIME.with_mut(|state| {
        let state = state.as_mut().ok_or(Errno::NoEnt)?;
        let request_id = enqueue_present_request(state, bytes);
        Ok(request_id)
    })
}

pub fn poll(endpoint: GpuEndpointKind, interest: u32) -> usize {
    GPU_RUNTIME.initialize();
    GPU_RUNTIME.with_mut(|state| {
        let Some(state) = state.as_mut() else {
            return 0;
        };
        let ready = match endpoint {
            GpuEndpointKind::Device => {
                let mut ready = POLLOUT as usize;
                if !state.completion_queue.is_empty() {
                    ready |= POLLIN as usize;
                }
                ready
            }
            GpuEndpointKind::Driver => {
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
    endpoint: GpuEndpointKind,
    buffer: *mut u8,
    len: usize,
    nonblock: bool,
) -> Result<usize, Errno> {
    if buffer.is_null() {
        return Err(Errno::Fault);
    }
    GPU_RUNTIME.initialize();
    GPU_RUNTIME.with_mut(|state| {
        let state = state.as_mut().ok_or(Errno::NoEnt)?;
        let queue = match endpoint {
            GpuEndpointKind::Device => &mut state.completion_queue,
            GpuEndpointKind::Driver => {
                let Some((request_id, bytes)) = state.driver_queue.pop_front() else {
                    return if nonblock { Err(Errno::Again) } else { Ok(0) };
                };
                let started_tick = next_tick(state);
                if let Some(record) = state
                    .request_records
                    .iter_mut()
                    .find(|record| record.request_id == request_id)
                {
                    record.state = GPU_REQUEST_STATE_INFLIGHT;
                    record.started_tick = started_tick;
                }
                let count = bytes.len().min(len);
                unsafe {
                    core::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, count);
                }
                return Ok(count);
            }
        };
        let Some(bytes) = queue.pop_front() else {
            return if nonblock { Err(Errno::Again) } else { Ok(0) };
        };
        let count = bytes.len().min(len);
        unsafe {
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, count);
        }
        Ok(count)
    })
}

pub fn write(endpoint: GpuEndpointKind, bytes: &[u8]) -> Result<usize, Errno> {
    GPU_RUNTIME.initialize();
    GPU_RUNTIME.with_mut(|state| {
        let state = state.as_mut().ok_or(Errno::NoEnt)?;
        match endpoint {
            GpuEndpointKind::Device => {
                enqueue_present_request(state, bytes);
                Ok(bytes.len())
            }
            GpuEndpointKind::Driver => complete_driver_request(state, bytes),
        }
    })
}

fn enqueue_present_request(state: &mut BootGpuRuntimeState, bytes: &[u8]) -> u64 {
    state.last_request_id = state.last_request_id.saturating_add(1);
    state.submitted_requests = state.submitted_requests.saturating_add(1);
    state.in_flight_requests = state.in_flight_requests.saturating_add(1);
    state.display_present = true;
    state.active_pipes = 1;
    state.planned_frames = state.planned_frames.saturating_add(1);
    state.last_present_offset = 0;
    state.last_present_len = bytes.len() as u64;
    state.hardware_programming_confirmed = false;
    state.last_terminal_request_id = state.last_request_id;
    state.last_terminal_state = GPU_REQUEST_STATE_INFLIGHT;
    set_metadata(
        bytes,
        &mut state.last_terminal_frame_tag,
        &mut state.last_terminal_source_api_name,
        &mut state.last_terminal_translation_label,
    );
    let submitted_tick = next_tick(state);
    state.request_records.push(BootGpuRequestRecord {
        request_id: state.last_request_id,
        issuer: GPU_BOOT_ISSUER,
        kind: GPU_REQUEST_KIND_CONTROL,
        state: 0,
        opcode: GPU_PRESENT_OPCODE,
        buffer_id: 0,
        payload_len: bytes.len() as u64,
        response_len: 0,
        submitted_tick,
        started_tick: 0,
        completed_tick: 0,
        frame_tag: state.last_terminal_frame_tag,
        source_api_name: state.last_terminal_source_api_name,
        translation_label: state.last_terminal_translation_label,
    });
    state
        .driver_queue
        .push_back((state.last_request_id, bytes.to_vec()));
    state.last_request_id
}

fn complete_driver_request(state: &mut BootGpuRuntimeState, bytes: &[u8]) -> Result<usize, Errno> {
    let outcome = parse_driver_outcome(bytes);
    if state.in_flight_requests != 0 {
        state.in_flight_requests -= 1;
    }
    state.last_terminal_request_id = outcome.request_id.unwrap_or(state.last_request_id);
    match outcome.kind {
        DriverOutcomeKind::Complete => {
            state.completed_requests = state.completed_requests.saturating_add(1);
            state.last_completed_request_id = state.last_terminal_request_id;
            state.last_terminal_state = GPU_REQUEST_STATE_COMPLETED;
            let completion_tick = next_tick(state);
            set_metadata(
                bytes,
                &mut state.last_completed_frame_tag,
                &mut state.last_completed_source_api_name,
                &mut state.last_completed_translation_label,
            );
            state.last_terminal_frame_tag = state.last_completed_frame_tag;
            state.last_terminal_source_api_name = state.last_completed_source_api_name;
            state.last_terminal_translation_label = state.last_completed_translation_label;
            state.presented_frames = state.presented_frames.saturating_add(1);
            state.last_frame_len = outcome.payload.len() as u64;
            state.last_scanout_frame.clear();
            state.last_scanout_frame.extend_from_slice(outcome.payload);
            state.last_scanout_frame_tag = state.last_completed_frame_tag;
            state.last_scanout_source_api_name = state.last_completed_source_api_name;
            state.last_scanout_translation_label = state.last_completed_translation_label;
            state.completion_queue.push_back(outcome.payload.to_vec());
            if let Some(record) = state
                .request_records
                .iter_mut()
                .find(|record| record.request_id == state.last_terminal_request_id)
            {
                record.state = GPU_REQUEST_STATE_COMPLETED;
                record.response_len = outcome.payload.len() as u64;
                record.completed_tick = completion_tick;
                if text_len(&record.frame_tag) == 0 {
                    record.frame_tag = state.last_completed_frame_tag;
                    record.source_api_name = state.last_completed_source_api_name;
                    record.translation_label = state.last_completed_translation_label;
                }
            }
            Ok(bytes.len())
        }
        DriverOutcomeKind::Fail => {
            state.last_terminal_state = GPU_REQUEST_STATE_FAILED;
            let completed_tick = next_tick(state);
            if let Some(record) = state
                .request_records
                .iter_mut()
                .find(|record| record.request_id == state.last_terminal_request_id)
            {
                record.state = GPU_REQUEST_STATE_FAILED;
                record.response_len = outcome.payload.len() as u64;
                record.completed_tick = completed_tick;
            }
            state.completion_queue.push_back(outcome.payload.to_vec());
            Ok(bytes.len())
        }
        DriverOutcomeKind::Cancel => {
            state.last_terminal_state = GPU_REQUEST_STATE_CANCELED;
            let completed_tick = next_tick(state);
            if let Some(record) = state
                .request_records
                .iter_mut()
                .find(|record| record.request_id == state.last_terminal_request_id)
            {
                record.state = GPU_REQUEST_STATE_CANCELED;
                record.response_len = 0;
                record.completed_tick = completed_tick;
            }
            Ok(bytes.len())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DriverOutcomeKind {
    Complete,
    Fail,
    Cancel,
}

struct DriverOutcome<'a> {
    kind: DriverOutcomeKind,
    request_id: Option<u64>,
    payload: &'a [u8],
}

fn parse_driver_outcome(bytes: &[u8]) -> DriverOutcome<'_> {
    let Some(split_index) = bytes.iter().position(|byte| *byte == b'\n') else {
        return DriverOutcome {
            kind: DriverOutcomeKind::Complete,
            request_id: None,
            payload: bytes,
        };
    };
    let header = &bytes[..split_index];
    let payload = &bytes[split_index + 1..];
    if let Some(request_id) = parse_outcome_header(header, b"complete-request:") {
        return DriverOutcome {
            kind: DriverOutcomeKind::Complete,
            request_id: Some(request_id),
            payload,
        };
    }
    if let Some(request_id) = parse_outcome_header(header, b"failed-request:") {
        return DriverOutcome {
            kind: DriverOutcomeKind::Fail,
            request_id: Some(request_id),
            payload,
        };
    }
    if let Some(request_id) = parse_outcome_header(header, b"cancel-request:") {
        return DriverOutcome {
            kind: DriverOutcomeKind::Cancel,
            request_id: Some(request_id),
            payload,
        };
    }
    DriverOutcome {
        kind: DriverOutcomeKind::Complete,
        request_id: None,
        payload: bytes,
    }
}

fn parse_outcome_header(header: &[u8], prefix: &[u8]) -> Option<u64> {
    let value = header.strip_prefix(prefix)?;
    let text = core::str::from_utf8(value).ok()?;
    text.parse::<u64>().ok()
}

fn set_metadata(
    bytes: &[u8],
    frame_tag: &mut [u8; 64],
    source_api_name: &mut [u8; 24],
    translation_label: &mut [u8; 32],
) {
    write_fixed(frame_tag, find_value(bytes, b"frame=").unwrap_or_default());
    write_fixed(
        source_api_name,
        find_value(bytes, b"source-api=").unwrap_or_default(),
    );
    write_fixed(
        translation_label,
        find_value(bytes, b"translation=").unwrap_or_default(),
    );
}

fn find_value<'a>(bytes: &'a [u8], prefix: &[u8]) -> Option<&'a [u8]> {
    bytes
        .split(|b| *b == b'\n')
        .find_map(|line| line.strip_prefix(prefix))
}

fn write_fixed<const N: usize>(dst: &mut [u8; N], value: &[u8]) {
    *dst = [0; N];
    let count = value.len().min(N.saturating_sub(1));
    dst[..count].copy_from_slice(&value[..count]);
}

fn next_tick(state: &mut BootGpuRuntimeState) -> u64 {
    let tick = state.next_tick;
    state.next_tick = state.next_tick.saturating_add(1);
    tick
}

fn text_len<const N: usize>(bytes: &[u8; N]) -> usize {
    bytes.iter().position(|byte| *byte == 0).unwrap_or(N)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicBool, Ordering};

    static TEST_LOCK: AtomicBool = AtomicBool::new(false);

    struct TestGuard;

    impl Drop for TestGuard {
        fn drop(&mut self) {
            TEST_LOCK.store(false, Ordering::Release);
        }
    }

    fn lock_test_state() -> TestGuard {
        while TEST_LOCK
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        TestGuard
    }

    fn text(bytes: &[u8]) -> &str {
        let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
        core::str::from_utf8(&bytes[..end]).unwrap()
    }

    #[test]
    fn boot_gpu_runtime_tracks_submit_complete_and_scanout_metadata() {
        let _guard = lock_test_state();
        reset();
        let payload =
            b"frame=boot-gpu-001\nsource-api=directx12\ntranslation=compat-to-vulkan\npresent=0,0,1280,720";
        assert_eq!(write(GpuEndpointKind::Device, payload), Ok(payload.len()));
        let mut request = [0u8; 256];
        let request_len = read(
            GpuEndpointKind::Driver,
            request.as_mut_ptr(),
            request.len(),
            false,
        )
        .unwrap();
        assert_eq!(&request[..request_len], payload);
        assert_eq!(write(GpuEndpointKind::Driver, payload), Ok(payload.len()));
        let display = gpu_display_record(GPU_DEVICE_PATH).unwrap();
        assert_eq!(display.present, 1);
        assert_eq!(display.planned_frames, 1);
        let scanout = gpu_scanout_record(GPU_DEVICE_PATH).unwrap();
        assert_eq!(scanout.presented_frames, 1);
        assert_eq!(text(&scanout.last_frame_tag), "boot-gpu-001");
        assert_eq!(text(&scanout.last_source_api_name), "directx12");
        assert_eq!(text(&scanout.last_translation_label), "compat-to-vulkan");
        let request = device_request_record(1).unwrap();
        assert_eq!(request.state, GPU_REQUEST_STATE_COMPLETED);
        assert_eq!(request.opcode, GPU_PRESENT_OPCODE);
        assert_eq!(text(&request.frame_tag), "boot-gpu-001");
    }

    #[test]
    fn boot_gpu_runtime_retains_failed_terminal_metadata_and_device_error_payload() {
        let _guard = lock_test_state();
        reset();
        let payload =
            b"frame=boot-gpu-fail-001\nsource-api=directx12\ntranslation=compat-to-vulkan\npresent=0,0,1280,720";
        assert_eq!(write(GpuEndpointKind::Device, payload), Ok(payload.len()));
        let mut request = [0u8; 256];
        let request_len = read(
            GpuEndpointKind::Driver,
            request.as_mut_ptr(),
            request.len(),
            false,
        )
        .unwrap();
        assert_eq!(&request[..request_len], payload);

        let driver_reply = b"failed-request:1\nerror:boot-present";
        assert_eq!(
            write(GpuEndpointKind::Driver, driver_reply),
            Ok(driver_reply.len())
        );

        let device = device_record(GPU_DEVICE_PATH).unwrap();
        let driver = driver_record(GPU_DRIVER_PATH).unwrap();
        let request = device_request_record(1).unwrap();
        assert_eq!(device.last_terminal_request_id, 1);
        assert_eq!(device.last_terminal_state, GPU_REQUEST_STATE_FAILED);
        assert_eq!(driver.last_terminal_state, GPU_REQUEST_STATE_FAILED);
        assert_eq!(request.state, GPU_REQUEST_STATE_FAILED);
        assert_eq!(request.response_len, b"error:boot-present".len() as u64);
        assert_eq!(text(&device.last_terminal_frame_tag), "boot-gpu-fail-001");
        assert_eq!(text(&driver.last_terminal_source_api_name), "directx12");
        assert_eq!(
            text(&driver.last_terminal_translation_label),
            "compat-to-vulkan"
        );

        let mut completion = [0u8; 128];
        let completion_len = read(
            GpuEndpointKind::Device,
            completion.as_mut_ptr(),
            completion.len(),
            false,
        )
        .unwrap();
        assert_eq!(&completion[..completion_len], b"error:boot-present");
        let scanout = gpu_scanout_record(GPU_DEVICE_PATH).unwrap();
        assert_eq!(scanout.presented_frames, 0);
    }

    #[test]
    fn boot_gpu_runtime_retains_canceled_terminal_metadata_without_scanout_or_completion() {
        let _guard = lock_test_state();
        reset();
        let payload =
            b"frame=boot-gpu-cancel-001\nsource-api=opengl\ntranslation=compat-to-vulkan\npresent=0,0,800,600";
        assert_eq!(write(GpuEndpointKind::Device, payload), Ok(payload.len()));
        let mut request = [0u8; 256];
        let request_len = read(
            GpuEndpointKind::Driver,
            request.as_mut_ptr(),
            request.len(),
            false,
        )
        .unwrap();
        assert_eq!(&request[..request_len], payload);

        let driver_reply = b"cancel-request:1\nabort:boot-present";
        assert_eq!(
            write(GpuEndpointKind::Driver, driver_reply),
            Ok(driver_reply.len())
        );

        let device = device_record(GPU_DEVICE_PATH).unwrap();
        let driver = driver_record(GPU_DRIVER_PATH).unwrap();
        let request = device_request_record(1).unwrap();
        assert_eq!(device.last_terminal_request_id, 1);
        assert_eq!(device.last_terminal_state, GPU_REQUEST_STATE_CANCELED);
        assert_eq!(driver.last_terminal_state, GPU_REQUEST_STATE_CANCELED);
        assert_eq!(request.state, GPU_REQUEST_STATE_CANCELED);
        assert_eq!(request.response_len, 0);
        assert_eq!(text(&device.last_terminal_frame_tag), "boot-gpu-cancel-001");
        assert_eq!(text(&driver.last_terminal_source_api_name), "opengl");
        assert_eq!(
            text(&driver.last_terminal_translation_label),
            "compat-to-vulkan"
        );

        let mut completion = [0u8; 64];
        assert_eq!(
            read(
                GpuEndpointKind::Device,
                completion.as_mut_ptr(),
                completion.len(),
                false,
            ),
            Ok(0)
        );
        let scanout = gpu_scanout_record(GPU_DEVICE_PATH).unwrap();
        assert_eq!(scanout.presented_frames, 0);
    }
}
