extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

use ngos_user_abi::{
    Errno, NativeDeviceRecord, NativeDeviceRequestRecord, NativeDriverRecord, POLLIN, POLLOUT,
};

pub const INPUT_DEVICE_PATH: &str = "/dev/input0";
pub const INPUT_DRIVER_PATH: &str = "/drv/input0";

const INPUT_DEVICE_CLASS: u32 = 5;
const INPUT_DEVICE_STATE_REGISTERED: u32 = 0;
const INPUT_DRIVER_STATE_ACTIVE: u32 = 1;
const INPUT_QUEUE_CAPACITY: u64 = 128;
const INPUT_BOOT_ISSUER: u64 = 1;
const INPUT_REQUEST_KIND_WRITE: u32 = 1;
const INPUT_DELIVERY_OPCODE: u64 = 0x494e_0001;
const INPUT_REQUEST_STATE_INFLIGHT: u32 = 1;
const INPUT_REQUEST_STATE_COMPLETED: u32 = 2;
const INPUT_REQUEST_STATE_FAILED: u32 = 3;
const INPUT_REQUEST_STATE_CANCELED: u32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEndpointKind {
    Device,
    Driver,
}

#[derive(Debug, Clone)]
struct BootInputRequestRecord {
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
struct BootInputRuntimeState {
    driver_queue: VecDeque<(u64, Vec<u8>)>,
    completion_queue: VecDeque<Vec<u8>>,
    request_records: Vec<BootInputRequestRecord>,
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
    last_payload_len: u64,
    last_completion_payload: Vec<u8>,
}

impl Default for BootInputRuntimeState {
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
            last_payload_len: 0,
            last_completion_payload: Vec::new(),
        }
    }
}

struct BootInputRuntimeCell {
    locked: AtomicBool,
    state: UnsafeCell<Option<BootInputRuntimeState>>,
}

unsafe impl Sync for BootInputRuntimeCell {}

impl BootInputRuntimeCell {
    const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
            state: UnsafeCell::new(None),
        }
    }

    fn initialize(&self) {
        self.with_mut(|state| {
            if state.is_none() {
                *state = Some(BootInputRuntimeState::default());
            }
        });
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut Option<BootInputRuntimeState>) -> R) -> R {
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

static INPUT_RUNTIME: BootInputRuntimeCell = BootInputRuntimeCell::new();

pub fn reset() {
    INPUT_RUNTIME.with_mut(|state| {
        *state = Some(BootInputRuntimeState::default());
    });
}

pub fn endpoint_for_path(path: &str) -> Option<InputEndpointKind> {
    match path {
        INPUT_DEVICE_PATH => Some(InputEndpointKind::Device),
        INPUT_DRIVER_PATH => Some(InputEndpointKind::Driver),
        _ => None,
    }
}

pub fn device_record(path: &str) -> Option<NativeDeviceRecord> {
    if path != INPUT_DEVICE_PATH {
        return None;
    }
    INPUT_RUNTIME.initialize();
    INPUT_RUNTIME.with_mut(|state| {
        let state = state.as_mut()?;
        Some(NativeDeviceRecord {
            class: INPUT_DEVICE_CLASS,
            state: INPUT_DEVICE_STATE_REGISTERED,
            reserved0: 0,
            queue_depth: state.driver_queue.len() as u64,
            queue_capacity: INPUT_QUEUE_CAPACITY,
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
            capacity_bytes: state.last_payload_len,
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
    if path != INPUT_DRIVER_PATH {
        return None;
    }
    INPUT_RUNTIME.initialize();
    INPUT_RUNTIME.with_mut(|state| {
        let state = state.as_mut()?;
        Some(NativeDriverRecord {
            state: INPUT_DRIVER_STATE_ACTIVE,
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
    INPUT_RUNTIME.initialize();
    INPUT_RUNTIME.with_mut(|state| {
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

pub fn poll(endpoint: InputEndpointKind, interest: u32) -> usize {
    INPUT_RUNTIME.initialize();
    INPUT_RUNTIME.with_mut(|state| {
        let Some(state) = state.as_mut() else {
            return 0;
        };
        let ready = match endpoint {
            InputEndpointKind::Device => {
                let mut ready = POLLOUT as usize;
                if !state.completion_queue.is_empty() {
                    ready |= POLLIN as usize;
                }
                ready
            }
            InputEndpointKind::Driver => {
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
    endpoint: InputEndpointKind,
    buffer: *mut u8,
    len: usize,
    nonblock: bool,
) -> Result<usize, Errno> {
    if buffer.is_null() {
        return Err(Errno::Fault);
    }
    INPUT_RUNTIME.initialize();
    INPUT_RUNTIME.with_mut(|state| {
        let state = state.as_mut().ok_or(Errno::NoEnt)?;
        let queue = match endpoint {
            InputEndpointKind::Device => &mut state.completion_queue,
            InputEndpointKind::Driver => {
                let Some((request_id, bytes)) = state.driver_queue.pop_front() else {
                    return if nonblock { Err(Errno::Again) } else { Ok(0) };
                };
                let started_tick = next_tick(state);
                if let Some(record) = state
                    .request_records
                    .iter_mut()
                    .find(|record| record.request_id == request_id)
                {
                    record.state = INPUT_REQUEST_STATE_INFLIGHT;
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

pub fn write(endpoint: InputEndpointKind, bytes: &[u8]) -> Result<usize, Errno> {
    INPUT_RUNTIME.initialize();
    INPUT_RUNTIME.with_mut(|state| {
        let state = state.as_mut().ok_or(Errno::NoEnt)?;
        match endpoint {
            InputEndpointKind::Device => {
                enqueue_input_request(state, bytes);
                Ok(bytes.len())
            }
            InputEndpointKind::Driver => complete_driver_request(state, bytes),
        }
    })
}

fn enqueue_input_request(state: &mut BootInputRuntimeState, bytes: &[u8]) -> u64 {
    state.last_request_id = state.last_request_id.saturating_add(1);
    state.submitted_requests = state.submitted_requests.saturating_add(1);
    state.in_flight_requests = state.in_flight_requests.saturating_add(1);
    state.last_payload_len = bytes.len() as u64;
    state.last_terminal_request_id = state.last_request_id;
    state.last_terminal_state = INPUT_REQUEST_STATE_INFLIGHT;
    set_metadata(
        bytes,
        &mut state.last_terminal_frame_tag,
        &mut state.last_terminal_source_api_name,
        &mut state.last_terminal_translation_label,
    );
    let submitted_tick = next_tick(state);
    state.request_records.push(BootInputRequestRecord {
        request_id: state.last_request_id,
        issuer: INPUT_BOOT_ISSUER,
        kind: INPUT_REQUEST_KIND_WRITE,
        state: 0,
        opcode: INPUT_DELIVERY_OPCODE,
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

fn complete_driver_request(
    state: &mut BootInputRuntimeState,
    bytes: &[u8],
) -> Result<usize, Errno> {
    let outcome = parse_driver_outcome(bytes);
    if state.in_flight_requests != 0 {
        state.in_flight_requests -= 1;
    }
    state.last_terminal_request_id = outcome.request_id.unwrap_or(state.last_request_id);
    match outcome.kind {
        DriverOutcomeKind::Complete => {
            state.completed_requests = state.completed_requests.saturating_add(1);
            state.last_completed_request_id = state.last_terminal_request_id;
            state.last_terminal_state = INPUT_REQUEST_STATE_COMPLETED;
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
            state.last_payload_len = outcome.payload.len() as u64;
            state.last_completion_payload.clear();
            state
                .last_completion_payload
                .extend_from_slice(outcome.payload);
            state.completion_queue.push_back(outcome.payload.to_vec());
            if let Some(record) = state
                .request_records
                .iter_mut()
                .find(|record| record.request_id == state.last_terminal_request_id)
            {
                record.state = INPUT_REQUEST_STATE_COMPLETED;
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
            state.last_terminal_state = INPUT_REQUEST_STATE_FAILED;
            let completed_tick = next_tick(state);
            if let Some(record) = state
                .request_records
                .iter_mut()
                .find(|record| record.request_id == state.last_terminal_request_id)
            {
                record.state = INPUT_REQUEST_STATE_FAILED;
                record.response_len = outcome.payload.len() as u64;
                record.completed_tick = completed_tick;
            }
            state.last_payload_len = outcome.payload.len() as u64;
            state.last_completion_payload.clear();
            state
                .last_completion_payload
                .extend_from_slice(outcome.payload);
            state.completion_queue.push_back(outcome.payload.to_vec());
            Ok(bytes.len())
        }
        DriverOutcomeKind::Cancel => {
            state.last_terminal_state = INPUT_REQUEST_STATE_CANCELED;
            let completed_tick = next_tick(state);
            if let Some(record) = state
                .request_records
                .iter_mut()
                .find(|record| record.request_id == state.last_terminal_request_id)
            {
                record.state = INPUT_REQUEST_STATE_CANCELED;
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

fn next_tick(state: &mut BootInputRuntimeState) -> u64 {
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

    fn acquire_guard() -> TestGuard {
        while TEST_LOCK
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        reset();
        TestGuard
    }

    #[test]
    fn boot_input_runtime_tracks_submit_complete_and_terminal_metadata() {
        let _guard = acquire_guard();
        let payload = b"ngos-input-translate/v1\nframe=boot-input-001\nsource-api=xinput\ntranslation=compat-to-input\ndevice=gamepad\n";
        assert_eq!(
            write(InputEndpointKind::Device, payload).unwrap(),
            payload.len()
        );

        let mut driver_bytes = [0u8; 256];
        let driver_len = read(
            InputEndpointKind::Driver,
            driver_bytes.as_mut_ptr(),
            driver_bytes.len(),
            false,
        )
        .unwrap();
        assert!(driver_len != 0);

        let mut completion = b"complete-request:1\n".to_vec();
        completion.extend_from_slice(payload);
        assert_eq!(
            write(InputEndpointKind::Driver, &completion).unwrap(),
            completion.len()
        );

        let device = device_record(INPUT_DEVICE_PATH).unwrap();
        let driver = driver_record(INPUT_DRIVER_PATH).unwrap();
        let request = device_request_record(1).unwrap();

        assert_eq!(device.submitted_requests, 1);
        assert_eq!(device.completed_requests, 1);
        assert_eq!(driver.completed_requests, 1);
        assert_eq!(request.state, INPUT_REQUEST_STATE_COMPLETED);
        assert_eq!(&request.frame_tag[..14], b"boot-input-001");
        assert_eq!(&request.source_api_name[..6], b"xinput");
        assert_eq!(&request.translation_label[..15], b"compat-to-input");
    }

    #[test]
    fn boot_input_runtime_retains_failed_terminal_metadata_and_device_error_payload() {
        let _guard = acquire_guard();
        let payload = b"ngos-input-translate/v1\nframe=boot-input-fail\nsource-api=sdl\ntranslation=compat-to-input\n";
        assert_eq!(
            write(InputEndpointKind::Device, payload).unwrap(),
            payload.len()
        );
        let mut driver_bytes = [0u8; 256];
        let _ = read(
            InputEndpointKind::Driver,
            driver_bytes.as_mut_ptr(),
            driver_bytes.len(),
            false,
        )
        .unwrap();

        let reply = b"failed-request:1\nerror:boot-input";
        assert_eq!(
            write(InputEndpointKind::Driver, reply).unwrap(),
            reply.len()
        );

        let device = device_record(INPUT_DEVICE_PATH).unwrap();
        let driver = driver_record(INPUT_DRIVER_PATH).unwrap();
        let request = device_request_record(1).unwrap();
        assert_eq!(device.last_terminal_state, INPUT_REQUEST_STATE_FAILED);
        assert_eq!(driver.last_terminal_state, INPUT_REQUEST_STATE_FAILED);
        assert_eq!(request.state, INPUT_REQUEST_STATE_FAILED);

        let mut completion = [0u8; 64];
        let count = read(
            InputEndpointKind::Device,
            completion.as_mut_ptr(),
            completion.len(),
            false,
        )
        .unwrap();
        assert_eq!(&completion[..count], b"error:boot-input");
    }

    #[test]
    fn boot_input_runtime_retains_canceled_terminal_metadata_without_completion_payload() {
        let _guard = acquire_guard();
        let payload =
            b"ngos-input-translate/v1\nframe=boot-input-cancel\nsource-api=evdev\ntranslation=native-input\n";
        assert_eq!(
            write(InputEndpointKind::Device, payload).unwrap(),
            payload.len()
        );
        let mut driver_bytes = [0u8; 256];
        let _ = read(
            InputEndpointKind::Driver,
            driver_bytes.as_mut_ptr(),
            driver_bytes.len(),
            false,
        )
        .unwrap();

        let reply = b"cancel-request:1\nabort:boot-input";
        assert_eq!(
            write(InputEndpointKind::Driver, reply).unwrap(),
            reply.len()
        );

        let device = device_record(INPUT_DEVICE_PATH).unwrap();
        let driver = driver_record(INPUT_DRIVER_PATH).unwrap();
        let request = device_request_record(1).unwrap();
        assert_eq!(device.last_terminal_state, INPUT_REQUEST_STATE_CANCELED);
        assert_eq!(driver.last_terminal_state, INPUT_REQUEST_STATE_CANCELED);
        assert_eq!(request.state, INPUT_REQUEST_STATE_CANCELED);

        let mut completion = [0u8; 64];
        assert_eq!(
            read(
                InputEndpointKind::Device,
                completion.as_mut_ptr(),
                completion.len(),
                false,
            )
            .unwrap(),
            0
        );
    }
}
