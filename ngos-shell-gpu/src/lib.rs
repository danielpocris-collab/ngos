#![no_std]
extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::fmt::Write;

use ngos_shell_proc::fixed_text_field;
use ngos_shell_types::{
    ShellVariable, parse_u64_arg, parse_usize_arg, resolve_shell_path, shell_set_variable,
};
use ngos_shell_vfs::{shell_write_all, write_line};
use ngos_user_abi::{
    ExitCode, NativeDeviceRecord, NativeDeviceRequestRecord, NativeDriverRecord,
    NativeEventQueueMode, NativeEventRecord, NativeEventSourceKind, NativeGpuScanoutRecord,
    POLLPRI, SyscallBackend,
};
use ngos_user_runtime::Runtime;

fn device_class_name(raw: u32) -> &'static str {
    match raw {
        0 => "generic",
        1 => "network",
        2 => "storage",
        3 => "graphics",
        4 => "audio",
        5 => "input",
        _ => "unknown",
    }
}

pub fn try_handle_gpu_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    variables: &mut Vec<ShellVariable>,
    line: &str,
    last_status: &mut i32,
) -> Option<Result<(), ExitCode>> {
    if let Some(path) = line.strip_prefix("device ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        return Some(shell_render_device(runtime, &resolved).map_err(|_| 205));
    }
    if let Some(path) = line.strip_prefix("gpu-evidence ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        return Some(shell_render_gpu_binding(runtime, &resolved).map_err(|_| 205));
    }
    if let Some(path) = line.strip_prefix("gpu-vbios ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        return Some(shell_render_gpu_vbios(runtime, &resolved).map_err(|_| 205));
    }
    if let Some(path) = line.strip_prefix("gpu-gsp ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        return Some(shell_render_gpu_gsp(runtime, &resolved).map_err(|_| 205));
    }
    if let Some(path) = line.strip_prefix("gpu-irq ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        return Some(shell_render_gpu_interrupt(runtime, &resolved).map_err(|_| 205));
    }
    if let Some(path) = line.strip_prefix("gpu-display ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        return Some(shell_render_gpu_display(runtime, &resolved).map_err(|_| 205));
    }
    if let Some(path) = line.strip_prefix("gpu-power ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        return Some(shell_render_gpu_power(runtime, &resolved).map_err(|_| 205));
    }
    if let Some(path) = line.strip_prefix("gpu-media ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        return Some(shell_render_gpu_media(runtime, &resolved).map_err(|_| 205));
    }
    if let Some(path) = line.strip_prefix("gpu-neural ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        return Some(shell_render_gpu_neural(runtime, &resolved).map_err(|_| 205));
    }
    if let Some(path) = line.strip_prefix("gpu-tensor ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        return Some(shell_render_gpu_tensor(runtime, &resolved).map_err(|_| 205));
    }
    if let Some(args) = line.strip_prefix("gpu-power-set ") {
        let mut parts = args.split_whitespace();
        let Some(path) = parts.next() else {
            return Some(Err(205));
        };
        let Some(state) = parts.next() else {
            return Some(Err(205));
        };
        let resolved = resolve_shell_path(cwd, path);
        return Some(shell_set_gpu_power(runtime, &resolved, state).map_err(|_| 205));
    }
    if let Some(args) = line.strip_prefix("gpu-media-start ") {
        let mut parts = args.split_whitespace();
        let (Some(path), Some(width), Some(height), Some(bitrate), Some(codec)) = (
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
        ) else {
            return Some(Err(205));
        };
        let resolved = resolve_shell_path(cwd, path);
        return Some(
            shell_start_gpu_media(runtime, &resolved, width, height, bitrate, codec)
                .map_err(|_| 205),
        );
    }
    if let Some(args) = line.strip_prefix("gpu-neural-inject ") {
        let mut parts = args.split_whitespace();
        let Some(path) = parts.next() else {
            return Some(Err(205));
        };
        let semantic = parts.collect::<Vec<_>>().join(" ");
        let resolved = resolve_shell_path(cwd, path);
        return Some(shell_inject_gpu_neural(runtime, &resolved, &semantic).map_err(|_| 205));
    }
    if let Some(path) = line.strip_prefix("gpu-neural-commit ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        return Some(shell_commit_gpu_neural(runtime, &resolved).map_err(|_| 205));
    }
    if let Some(args) = line.strip_prefix("gpu-tensor-dispatch ") {
        let mut parts = args.split_whitespace();
        let (Some(path), Some(kernel_id)) = (parts.next(), parts.next()) else {
            return Some(Err(205));
        };
        let resolved = resolve_shell_path(cwd, path);
        return Some(shell_dispatch_gpu_tensor(runtime, &resolved, kernel_id).map_err(|_| 205));
    }
    if let Some(path) = line.strip_prefix("driver ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        return Some(shell_render_driver(runtime, &resolved).map_err(|_| 205));
    }
    if let Some(path) = line.strip_prefix("gpu-driver-read ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        *last_status = match shell_gpu_driver_read(runtime, &resolved) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(path) = line.strip_prefix("gpu-driver-reset ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        *last_status = match shell_gpu_driver_reset(runtime, &resolved) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(path) = line.strip_prefix("gpu-probe-driver-reset ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        *last_status = match shell_gpu_probe_driver_reset(runtime, &resolved) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(path) = line.strip_prefix("gpu-driver-retire ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        *last_status = match shell_gpu_driver_retire(runtime, &resolved) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(path) = line.strip_prefix("gpu-probe-driver-retire ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        *last_status = match shell_gpu_probe_driver_retire(runtime, &resolved) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(path) = line.strip_prefix("gpu-read ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        *last_status = match shell_gpu_read(runtime, &resolved) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-lease-watch ") {
        let mut parts = rest.split_whitespace();
        let Some(resource) = parse_usize_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: gpu-lease-watch <resource> <token>");
            return Some(Err(2));
        };
        let Some(token) = parse_u64_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: gpu-lease-watch <resource> <token>");
            return Some(Err(2));
        };
        *last_status = match shell_watch_gpu_lease(runtime, resource, token) {
            Ok(fd) => {
                shell_set_variable(variables, "LAST_QUEUE_FD", fd.to_string());
                0
            }
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-lease-unwatch ") {
        let mut parts = rest.split_whitespace();
        let Some(queue_fd) = parse_usize_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: gpu-lease-unwatch <queue-fd> <resource> <token>",
            );
            return Some(Err(2));
        };
        let Some(resource) = parse_usize_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: gpu-lease-unwatch <queue-fd> <resource> <token>",
            );
            return Some(Err(2));
        };
        let Some(token) = parse_u64_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: gpu-lease-unwatch <queue-fd> <resource> <token>",
            );
            return Some(Err(2));
        };
        *last_status = match shell_remove_gpu_lease_watch(runtime, queue_fd, resource, token) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-watch ") {
        let mut parts = rest.split_whitespace();
        let Some(queue_fd) = parse_usize_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: gpu-watch <queue-fd> <device> <token>");
            return Some(Err(2));
        };
        let Some(device_path) = parts.next() else {
            let _ = write_line(runtime, "usage: gpu-watch <queue-fd> <device> <token>");
            return Some(Err(2));
        };
        let Some(token) = parse_u64_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: gpu-watch <queue-fd> <device> <token>");
            return Some(Err(2));
        };
        *last_status = match shell_watch_graphics_events(
            runtime,
            queue_fd,
            &resolve_shell_path(cwd, device_path),
            token,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-unwatch ") {
        let mut parts = rest.split_whitespace();
        let Some(queue_fd) = parse_usize_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: gpu-unwatch <queue-fd> <device> <token>");
            return Some(Err(2));
        };
        let Some(device_path) = parts.next() else {
            let _ = write_line(runtime, "usage: gpu-unwatch <queue-fd> <device> <token>");
            return Some(Err(2));
        };
        let Some(token) = parse_u64_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: gpu-unwatch <queue-fd> <device> <token>");
            return Some(Err(2));
        };
        *last_status = match shell_remove_graphics_watch(
            runtime,
            queue_fd,
            &resolve_shell_path(cwd, device_path),
            token,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-lease-wait ") {
        let Some(queue_fd) = parse_usize_arg(Some(rest.trim())) else {
            let _ = write_line(runtime, "usage: gpu-lease-wait <queue-fd>");
            return Some(Err(2));
        };
        *last_status = match shell_wait_gpu_lease(runtime, queue_fd) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-submit ") {
        let mut parts = rest.split_whitespace();
        let Some(device_path) = parts.next() else {
            let _ = write_line(runtime, "usage: gpu-submit <device> <payload>");
            return Some(Err(2));
        };
        let payload = parts.collect::<Vec<_>>().join(" ");
        if payload.is_empty() {
            let _ = write_line(runtime, "usage: gpu-submit <device> <payload>");
            return Some(Err(2));
        }
        *last_status =
            match shell_gpu_submit(runtime, &resolve_shell_path(cwd, device_path), &payload) {
                Ok(()) => 0,
                Err(code) => code,
            };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-queue-capacity ") {
        let mut parts = rest.split_whitespace();
        let Some(device_path) = parts.next() else {
            let _ = write_line(runtime, "usage: gpu-queue-capacity <device> <count>");
            return Some(Err(2));
        };
        let Some(queue_capacity) = parse_usize_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: gpu-queue-capacity <device> <count>");
            return Some(Err(2));
        };
        *last_status = match shell_gpu_queue_capacity(
            runtime,
            &resolve_shell_path(cwd, device_path),
            queue_capacity,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-request ") {
        let Some(request_id) = parse_u64_arg(Some(rest.trim())) else {
            let _ = write_line(runtime, "usage: gpu-request <id>");
            return Some(Err(2));
        };
        *last_status = match shell_gpu_request(runtime, request_id) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-present ") {
        let mut parts = rest.split_whitespace();
        let Some(device_path) = parts.next() else {
            let _ = write_line(runtime, "usage: gpu-present <device> <frame>");
            return Some(Err(2));
        };
        let frame = parts.collect::<Vec<_>>().join(" ");
        if frame.is_empty() {
            let _ = write_line(runtime, "usage: gpu-present <device> <frame>");
            return Some(Err(2));
        }
        *last_status =
            match shell_gpu_present(runtime, &resolve_shell_path(cwd, device_path), &frame) {
                Ok(()) => 0,
                Err(code) => code,
            };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-probe-submit ") {
        let mut parts = rest.split_whitespace();
        let Some(device_path) = parts.next() else {
            let _ = write_line(runtime, "usage: gpu-probe-submit <device> <payload>");
            return Some(Err(2));
        };
        let payload = parts.collect::<Vec<_>>().join(" ");
        if payload.is_empty() {
            let _ = write_line(runtime, "usage: gpu-probe-submit <device> <payload>");
            return Some(Err(2));
        }
        *last_status = match shell_gpu_probe_submit(
            runtime,
            &resolve_shell_path(cwd, device_path),
            &payload,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-probe-present ") {
        let mut parts = rest.split_whitespace();
        let Some(device_path) = parts.next() else {
            let _ = write_line(runtime, "usage: gpu-probe-present <device> <frame>");
            return Some(Err(2));
        };
        let frame = parts.collect::<Vec<_>>().join(" ");
        if frame.is_empty() {
            let _ = write_line(runtime, "usage: gpu-probe-present <device> <frame>");
            return Some(Err(2));
        }
        *last_status =
            match shell_gpu_probe_present(runtime, &resolve_shell_path(cwd, device_path), &frame) {
                Ok(()) => 0,
                Err(code) => code,
            };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-buffer-create ") {
        let Some(length) = parse_u64_arg(Some(rest.trim())) else {
            let _ = write_line(runtime, "usage: gpu-buffer-create <length>");
            return Some(Err(2));
        };
        *last_status = match shell_gpu_buffer_create(runtime, length as usize) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-buffer-write ") {
        let mut parts = rest.split_whitespace();
        let Some(buffer_id) = parse_u64_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: gpu-buffer-write <buffer> <offset> <payload>",
            );
            return Some(Err(2));
        };
        let Some(offset) = parse_u64_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: gpu-buffer-write <buffer> <offset> <payload>",
            );
            return Some(Err(2));
        };
        let payload = parts.collect::<Vec<_>>().join(" ");
        if payload.is_empty() {
            let _ = write_line(
                runtime,
                "usage: gpu-buffer-write <buffer> <offset> <payload>",
            );
            return Some(Err(2));
        }
        *last_status = match shell_gpu_buffer_write(runtime, buffer_id, offset as usize, &payload) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-buffer ") {
        let Some(buffer_id) = parse_u64_arg(Some(rest.trim())) else {
            let _ = write_line(runtime, "usage: gpu-buffer <buffer>");
            return Some(Err(2));
        };
        *last_status = match shell_gpu_buffer(runtime, buffer_id) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(path) = line.strip_prefix("gpu-scanout ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        *last_status = match shell_gpu_scanout(runtime, &resolved) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(path) = line.strip_prefix("gpu-perf ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        *last_status = match shell_gpu_perf(runtime, &resolved) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-submit-buffer ") {
        let mut parts = rest.split_whitespace();
        let Some(device_path) = parts.next() else {
            let _ = write_line(runtime, "usage: gpu-submit-buffer <device> <buffer>");
            return Some(Err(2));
        };
        let Some(buffer_id) = parse_u64_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: gpu-submit-buffer <device> <buffer>");
            return Some(Err(2));
        };
        *last_status = match shell_gpu_submit_buffer(
            runtime,
            &resolve_shell_path(cwd, device_path),
            buffer_id,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-probe-submit-buffer ") {
        let mut parts = rest.split_whitespace();
        let Some(device_path) = parts.next() else {
            let _ = write_line(runtime, "usage: gpu-probe-submit-buffer <device> <buffer>");
            return Some(Err(2));
        };
        let Some(buffer_id) = parse_u64_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: gpu-probe-submit-buffer <device> <buffer>");
            return Some(Err(2));
        };
        *last_status = match shell_gpu_probe_submit_buffer(
            runtime,
            &resolve_shell_path(cwd, device_path),
            buffer_id,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-driver-bind ") {
        let mut parts = rest.split_whitespace();
        let Some(device_path) = parts.next() else {
            let _ = write_line(runtime, "usage: gpu-driver-bind <device> <driver>");
            return Some(Err(2));
        };
        let Some(driver_path) = parts.next() else {
            let _ = write_line(runtime, "usage: gpu-driver-bind <device> <driver>");
            return Some(Err(2));
        };
        *last_status = match shell_gpu_driver_bind(
            runtime,
            &resolve_shell_path(cwd, device_path),
            &resolve_shell_path(cwd, driver_path),
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-probe-driver-bind ") {
        let mut parts = rest.split_whitespace();
        let Some(device_path) = parts.next() else {
            let _ = write_line(runtime, "usage: gpu-probe-driver-bind <device> <driver>");
            return Some(Err(2));
        };
        let Some(driver_path) = parts.next() else {
            let _ = write_line(runtime, "usage: gpu-probe-driver-bind <device> <driver>");
            return Some(Err(2));
        };
        *last_status = match shell_gpu_probe_driver_bind(
            runtime,
            &resolve_shell_path(cwd, device_path),
            &resolve_shell_path(cwd, driver_path),
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(path) = line.strip_prefix("gpu-driver-unbind ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        *last_status = match shell_gpu_driver_unbind(runtime, &resolved) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(path) = line.strip_prefix("gpu-probe-driver-unbind ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        *last_status = match shell_gpu_probe_driver_unbind(runtime, &resolved) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-complete ") {
        let mut parts = rest.split_whitespace();
        let Some(driver_path) = parts.next() else {
            let _ = write_line(runtime, "usage: gpu-complete <driver> <payload>");
            return Some(Err(2));
        };
        let payload = parts.collect::<Vec<_>>().join(" ");
        if payload.is_empty() {
            let _ = write_line(runtime, "usage: gpu-complete <driver> <payload>");
            return Some(Err(2));
        }
        *last_status =
            match shell_gpu_complete(runtime, &resolve_shell_path(cwd, driver_path), &payload) {
                Ok(()) => 0,
                Err(code) => code,
            };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-complete-request ") {
        let mut parts = rest.split_whitespace();
        let Some(driver_path) = parts.next() else {
            let _ = write_line(
                runtime,
                "usage: gpu-complete-request <driver> <request> <payload>",
            );
            return Some(Err(2));
        };
        let Some(request_id) = parse_u64_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: gpu-complete-request <driver> <request> <payload>",
            );
            return Some(Err(2));
        };
        let payload = parts.collect::<Vec<_>>().join(" ");
        if payload.is_empty() {
            let _ = write_line(
                runtime,
                "usage: gpu-complete-request <driver> <request> <payload>",
            );
            return Some(Err(2));
        }
        *last_status = match shell_gpu_complete_request(
            runtime,
            &resolve_shell_path(cwd, driver_path),
            request_id,
            &payload,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-fail-request ") {
        let mut parts = rest.split_whitespace();
        let Some(driver_path) = parts.next() else {
            let _ = write_line(
                runtime,
                "usage: gpu-fail-request <driver> <request> <payload>",
            );
            return Some(Err(2));
        };
        let Some(request_id) = parse_u64_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: gpu-fail-request <driver> <request> <payload>",
            );
            return Some(Err(2));
        };
        let payload = parts.collect::<Vec<_>>().join(" ");
        if payload.is_empty() {
            let _ = write_line(
                runtime,
                "usage: gpu-fail-request <driver> <request> <payload>",
            );
            return Some(Err(2));
        }
        *last_status = match shell_gpu_fail_request(
            runtime,
            &resolve_shell_path(cwd, driver_path),
            request_id,
            &payload,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-cancel-request ") {
        let mut parts = rest.split_whitespace();
        let Some(driver_path) = parts.next() else {
            let _ = write_line(
                runtime,
                "usage: gpu-cancel-request <driver> <request> <payload>",
            );
            return Some(Err(2));
        };
        let Some(request_id) = parse_u64_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: gpu-cancel-request <driver> <request> <payload>",
            );
            return Some(Err(2));
        };
        let payload = parts.collect::<Vec<_>>().join(" ");
        if payload.is_empty() {
            let _ = write_line(
                runtime,
                "usage: gpu-cancel-request <driver> <request> <payload>",
            );
            return Some(Err(2));
        }
        *last_status = match shell_gpu_cancel_request(
            runtime,
            &resolve_shell_path(cwd, driver_path),
            request_id,
            &payload,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-probe-complete ") {
        let mut parts = rest.split_whitespace();
        let Some(driver_path) = parts.next() else {
            let _ = write_line(runtime, "usage: gpu-probe-complete <driver> <payload>");
            return Some(Err(2));
        };
        let payload = parts.collect::<Vec<_>>().join(" ");
        if payload.is_empty() {
            let _ = write_line(runtime, "usage: gpu-probe-complete <driver> <payload>");
            return Some(Err(2));
        }
        *last_status = match shell_gpu_probe_complete(
            runtime,
            &resolve_shell_path(cwd, driver_path),
            &payload,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-probe-fail-request ") {
        let mut parts = rest.split_whitespace();
        let Some(driver_path) = parts.next() else {
            let _ = write_line(
                runtime,
                "usage: gpu-probe-fail-request <driver> <request> <payload>",
            );
            return Some(Err(2));
        };
        let Some(request_id) = parse_u64_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: gpu-probe-fail-request <driver> <request> <payload>",
            );
            return Some(Err(2));
        };
        let payload = parts.collect::<Vec<_>>().join(" ");
        if payload.is_empty() {
            let _ = write_line(
                runtime,
                "usage: gpu-probe-fail-request <driver> <request> <payload>",
            );
            return Some(Err(2));
        }
        *last_status = match shell_gpu_probe_fail_request(
            runtime,
            &resolve_shell_path(cwd, driver_path),
            request_id,
            &payload,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-probe-cancel-request ") {
        let mut parts = rest.split_whitespace();
        let Some(driver_path) = parts.next() else {
            let _ = write_line(
                runtime,
                "usage: gpu-probe-cancel-request <driver> <request> <payload>",
            );
            return Some(Err(2));
        };
        let Some(request_id) = parse_u64_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: gpu-probe-cancel-request <driver> <request> <payload>",
            );
            return Some(Err(2));
        };
        let payload = parts.collect::<Vec<_>>().join(" ");
        if payload.is_empty() {
            let _ = write_line(
                runtime,
                "usage: gpu-probe-cancel-request <driver> <request> <payload>",
            );
            return Some(Err(2));
        }
        *last_status = match shell_gpu_probe_cancel_request(
            runtime,
            &resolve_shell_path(cwd, driver_path),
            request_id,
            &payload,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("gpu-probe-complete-request ") {
        let mut parts = rest.split_whitespace();
        let Some(driver_path) = parts.next() else {
            let _ = write_line(
                runtime,
                "usage: gpu-probe-complete-request <driver> <request> <payload>",
            );
            return Some(Err(2));
        };
        let Some(request_id) = parse_u64_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: gpu-probe-complete-request <driver> <request> <payload>",
            );
            return Some(Err(2));
        };
        let payload = parts.collect::<Vec<_>>().join(" ");
        if payload.is_empty() {
            let _ = write_line(
                runtime,
                "usage: gpu-probe-complete-request <driver> <request> <payload>",
            );
            return Some(Err(2));
        }
        *last_status = match shell_gpu_probe_complete_request(
            runtime,
            &resolve_shell_path(cwd, driver_path),
            request_id,
            &payload,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    None
}

fn shell_render_device<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record: NativeDeviceRecord = runtime.inspect_device(device_path).map_err(|_| 246)?;
    let graphics_control_reserve = if record.class == 3 {
        if record.reserved0 != 0 {
            "armed"
        } else {
            "released"
        }
    } else {
        "n/a"
    };
    write_line(
        runtime,
        &format!(
            "device path={} class={} state={} queue-depth={} queue-capacity={} control-reserve={} submitted={} completed={} last-request={} last-frame={} last-api={} last-translation={} last-terminal-request={} last-terminal-state={} last-terminal-frame={} last-terminal-api={} last-terminal-translation={} total-latency={} max-latency={} total-queue-wait={} max-queue-wait={} link={} block-size={} capacity-bytes={}",
            device_path,
            device_class_name(record.class),
            record.state,
            record.queue_depth,
            record.queue_capacity,
            graphics_control_reserve,
            record.submitted_requests,
            record.completed_requests,
            record.last_completed_request_id,
            fixed_text_field(&record.last_completed_frame_tag),
            fixed_text_field(&record.last_completed_source_api_name),
            fixed_text_field(&record.last_completed_translation_label),
            record.last_terminal_request_id,
            gpu_request_state_name(record.last_terminal_state),
            fixed_text_field(&record.last_terminal_frame_tag),
            fixed_text_field(&record.last_terminal_source_api_name),
            fixed_text_field(&record.last_terminal_translation_label),
            record.total_latency_ticks,
            record.max_latency_ticks,
            record.total_queue_wait_ticks,
            record.max_queue_wait_ticks,
            if record.link_up != 0 { "up" } else { "down" },
            record.block_size,
            record.capacity_bytes
        ),
    )
}

pub fn summarize_graphics_deep_ops(payload: &str) -> String {
    let mut ops = Vec::<String>::new();
    for line in payload.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("op=") {
            let op = rest.split_whitespace().next().unwrap_or(rest);
            if !op.is_empty() {
                ops.push(op.to_string());
            }
        }
    }
    if ops.is_empty() {
        String::from("-")
    } else {
        ops.join(",")
    }
}

fn shell_render_gpu_binding<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record = runtime.inspect_gpu_binding(device_path).map_err(|_| 246)?;
    if record.present == 0 {
        return write_line(
            runtime,
            &format!("gpu-binding device={} status=unavailable", device_path),
        );
    }
    write_line(
        runtime,
        &format!(
            "gpu-binding device={} architecture={} product={} die={} bus-interface={} inf-section={} kernel-service={} vbios={} part={} subsystem=0x{:08x} bar1-mib={} framebuffer-mib={} resizable-bar={} display-engine-confirmed={} msi-source={} msi-supported={} msi-limit={}",
            device_path,
            fixed_text_field(&record.architecture_name),
            fixed_text_field(&record.product_name),
            fixed_text_field(&record.die_name),
            fixed_text_field(&record.bus_interface),
            fixed_text_field(&record.inf_section),
            fixed_text_field(&record.kernel_service),
            fixed_text_field(&record.vbios_version),
            fixed_text_field(&record.part_number),
            record.subsystem_id,
            record.bar1_total_mib,
            record.framebuffer_total_mib,
            record.resizable_bar_enabled,
            record.display_engine_confirmed,
            fixed_text_field(&record.msi_source_name),
            record.msi_supported,
            record.msi_message_limit
        ),
    )
}

fn shell_render_gpu_vbios<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record = runtime.inspect_gpu_vbios(device_path).map_err(|_| 246)?;
    if record.present == 0 {
        return write_line(
            runtime,
            &format!("gpu-vbios device={} status=unavailable", device_path),
        );
    }
    let header_len = core::cmp::min(record.header_len as usize, record.header.len());
    let mut header_hex = String::new();
    for (index, byte) in record.header[..header_len].iter().enumerate() {
        if index != 0 {
            header_hex.push(':');
        }
        let _ = write!(&mut header_hex, "{:02x}", byte);
    }
    write_line(
        runtime,
        &format!(
            "gpu-vbios device={} enabled={} rom-bar=0x{:08x} physical-base=0x{:x} image-len={} vendor=0x{:04x} device=0x{:04x} pcir=0x{:x} bit=0x{:x} nvfw=0x{:x} board={} code={} version={} header-len={} header={}",
            device_path,
            record.enabled,
            record.rom_bar_raw,
            record.physical_base,
            record.image_len,
            record.vendor_id,
            record.device_id,
            record.pcir_offset,
            record.bit_offset,
            record.nvfw_offset,
            fixed_text_field(&record.board_name),
            fixed_text_field(&record.board_code),
            fixed_text_field(&record.version),
            record.header_len,
            header_hex
        ),
    )
}

fn shell_render_gpu_gsp<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record = runtime.inspect_gpu_gsp(device_path).map_err(|_| 246)?;
    if record.present == 0 {
        return write_line(
            runtime,
            &format!("gpu-gsp device={} status=unavailable", device_path),
        );
    }
    write_line(
        runtime,
        &format!(
            "gpu-gsp device={} ready={} completions={} failures={} firmware-known={} firmware-version={} blackwell-blob={} blobs={}",
            device_path,
            record.loopback_ready,
            record.loopback_completions,
            record.loopback_failures,
            record.firmware_known,
            fixed_text_field(&record.firmware_version),
            record.blackwell_blob_present,
            fixed_text_field(&record.blob_summary),
        ),
    )
}

fn shell_render_gpu_interrupt<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record = runtime
        .inspect_gpu_interrupt(device_path)
        .map_err(|_| 246)?;
    if record.present == 0 {
        return write_line(
            runtime,
            &format!("gpu-irq device={} status=unavailable", device_path),
        );
    }
    write_line(
        runtime,
        &format!(
            "gpu-irq device={} vector={} delivered={} msi-supported={} message-limit={} windows-max={} hardware-confirmed={}",
            device_path,
            record.vector,
            record.delivered_count,
            record.msi_supported,
            record.message_limit,
            record.windows_interrupt_message_maximum,
            record.hardware_servicing_confirmed
        ),
    )
}

fn shell_render_gpu_display<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record = runtime.inspect_gpu_display(device_path).map_err(|_| 246)?;
    if record.present == 0 {
        return write_line(
            runtime,
            &format!("gpu-display device={} status=unavailable", device_path),
        );
    }
    write_line(
        runtime,
        &format!(
            "gpu-display device={} pipes={} planned={} hardware-confirmed={}",
            device_path,
            record.active_pipes,
            record.planned_frames,
            record.hardware_programming_confirmed
        ),
    )
}

fn shell_render_gpu_power<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record = runtime.inspect_gpu_power(device_path).map_err(|_| 247)?;
    if record.present == 0 {
        return write_line(
            runtime,
            &format!("gpu-power device={} status=unavailable", device_path),
        );
    }
    write_line(
        runtime,
        &format!(
            "gpu-power device={} pstate=P{} graphics-mhz={} memory-mhz={} boost-mhz={} hardware-confirmed={}",
            device_path,
            record.pstate,
            record.graphics_clock_mhz,
            record.memory_clock_mhz,
            record.boost_clock_mhz,
            record.hardware_power_management_confirmed
        ),
    )
}

fn parse_gpu_power_state(text: &str) -> Option<u32> {
    match text.trim() {
        "P0" | "p0" => Some(0),
        "P5" | "p5" => Some(5),
        "P8" | "p8" => Some(8),
        "P12" | "p12" => Some(12),
        _ => None,
    }
}

fn shell_set_gpu_power<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    state_text: &str,
) -> Result<(), ExitCode> {
    let Some(pstate) = parse_gpu_power_state(state_text) else {
        return write_line(
            runtime,
            &format!(
                "gpu-power-set device={} state={} status=invalid",
                device_path, state_text
            ),
        );
    };
    if runtime.set_gpu_power_state(device_path, pstate).is_err() {
        return write_line(
            runtime,
            &format!(
                "gpu-power-set device={} state=P{} status=unavailable",
                device_path, pstate
            ),
        );
    }
    write_line(
        runtime,
        &format!(
            "gpu-power-set device={} state=P{} status=ok",
            device_path, pstate
        ),
    )
}

fn shell_render_gpu_media<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record = runtime.inspect_gpu_media(device_path).map_err(|_| 247)?;
    if record.present == 0 {
        return write_line(
            runtime,
            &format!("gpu-media device={} status=unavailable", device_path),
        );
    }
    write_line(
        runtime,
        &format!(
            "gpu-media device={} sessions={} codec={} width={} height={} bitrate-kbps={} hardware-confirmed={}",
            device_path,
            record.sessions,
            record.codec,
            record.width,
            record.height,
            record.bitrate_kbps,
            record.hardware_media_confirmed
        ),
    )
}

fn shell_start_gpu_media<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    width_text: &str,
    height_text: &str,
    bitrate_text: &str,
    codec_text: &str,
) -> Result<(), ExitCode> {
    let Ok(width) = width_text.parse::<u32>() else {
        return write_line(
            runtime,
            &format!("gpu-media-start device={} status=invalid", device_path),
        );
    };
    let Ok(height) = height_text.parse::<u32>() else {
        return write_line(
            runtime,
            &format!("gpu-media-start device={} status=invalid", device_path),
        );
    };
    let Ok(bitrate_kbps) = bitrate_text.parse::<u32>() else {
        return write_line(
            runtime,
            &format!("gpu-media-start device={} status=invalid", device_path),
        );
    };
    if width == 0 || height == 0 || bitrate_kbps == 0 {
        return write_line(
            runtime,
            &format!("gpu-media-start device={} status=invalid", device_path),
        );
    }
    let codec = match codec_text {
        "h264" => 0,
        "hevc" => 1,
        "av1" => 2,
        _ => {
            return write_line(
                runtime,
                &format!("gpu-media-start device={} status=invalid", device_path),
            );
        }
    };
    if runtime
        .start_gpu_media_session(device_path, width, height, bitrate_kbps, codec)
        .is_err()
    {
        return write_line(
            runtime,
            &format!("gpu-media-start device={} status=unavailable", device_path),
        );
    }
    write_line(
        runtime,
        &format!(
            "gpu-media-start device={} width={} height={} bitrate-kbps={} codec={} status=ok",
            device_path, width, height, bitrate_kbps, codec_text
        ),
    )
}

fn shell_render_gpu_neural<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record = runtime.inspect_gpu_neural(device_path).map_err(|_| 247)?;
    if record.present == 0 {
        return write_line(
            runtime,
            &format!("gpu-neural device={} status=unavailable", device_path),
        );
    }
    write_line(
        runtime,
        &format!(
            "gpu-neural device={} model-loaded={} semantics={} committed={} hardware-confirmed={}",
            device_path,
            record.model_loaded,
            record.active_semantics,
            record.last_commit_completed,
            record.hardware_neural_confirmed
        ),
    )
}

fn shell_inject_gpu_neural<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    semantic_label: &str,
) -> Result<(), ExitCode> {
    if semantic_label.trim().is_empty() {
        return write_line(
            runtime,
            &format!("gpu-neural-inject device={} status=invalid", device_path),
        );
    }
    if runtime
        .inject_gpu_neural_semantic(device_path, semantic_label)
        .is_err()
    {
        return write_line(
            runtime,
            &format!(
                "gpu-neural-inject device={} status=unavailable",
                device_path
            ),
        );
    }
    write_line(
        runtime,
        &format!(
            "gpu-neural-inject device={} semantic={} status=ok",
            device_path, semantic_label
        ),
    )
}

fn shell_commit_gpu_neural<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    if runtime.commit_gpu_neural_frame(device_path).is_err() {
        return write_line(
            runtime,
            &format!(
                "gpu-neural-commit device={} status=unavailable",
                device_path
            ),
        );
    }
    write_line(
        runtime,
        &format!("gpu-neural-commit device={} status=ok", device_path),
    )
}

fn shell_render_gpu_tensor<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record = runtime.inspect_gpu_tensor(device_path).map_err(|_| 247)?;
    if record.present == 0 {
        return write_line(
            runtime,
            &format!("gpu-tensor device={} status=unavailable", device_path),
        );
    }
    write_line(
        runtime,
        &format!(
            "gpu-tensor device={} active-jobs={} last-kernel={} hardware-confirmed={}",
            device_path,
            record.active_jobs,
            record.last_kernel_id,
            record.hardware_tensor_confirmed
        ),
    )
}

fn shell_dispatch_gpu_tensor<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    kernel_text: &str,
) -> Result<(), ExitCode> {
    let Ok(kernel_id) = kernel_text.parse::<u32>() else {
        return write_line(
            runtime,
            &format!("gpu-tensor-dispatch device={} status=invalid", device_path),
        );
    };
    if kernel_id == 0 {
        return write_line(
            runtime,
            &format!("gpu-tensor-dispatch device={} status=invalid", device_path),
        );
    }
    if runtime
        .dispatch_gpu_tensor_kernel(device_path, kernel_id)
        .is_err()
    {
        return write_line(
            runtime,
            &format!(
                "gpu-tensor-dispatch device={} status=unavailable",
                device_path
            ),
        );
    }
    write_line(
        runtime,
        &format!(
            "gpu-tensor-dispatch device={} kernel={} status=ok",
            device_path, kernel_id
        ),
    )
}

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

pub fn shell_gpu_queue_capacity<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    queue_capacity: usize,
) -> Result<(), ExitCode> {
    runtime
        .configure_device_queue(device_path, queue_capacity)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "gpu-queue-capacity device={} queue-capacity={}",
            device_path, queue_capacity
        ),
    )
}

const GPU_PRESENT_OPCODE: u32 = 0x4750_0001;
const GPU_DRIVER_RESET_OPCODE: u32 = 0x4750_1001;
const GPU_DRIVER_RETIRE_OPCODE: u32 = 0x4750_1002;

fn shell_gpu_probe_submit<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    payload: &str,
) -> Result<(), ExitCode> {
    let (source_api, translation) = parse_gfx_payload_translation_metadata(payload);
    let before = runtime.inspect_device(device_path).ok();
    match runtime.open_path(device_path) {
        Ok(fd) => {
            let outcome = shell_write_all(runtime, fd, payload.as_bytes());
            let close_result = runtime.close(fd);
            let after = runtime.inspect_device(device_path).ok();
            match (outcome, close_result, before, after) {
                (Ok(()), Ok(()), Some(before), Some(after))
                    if after.submitted_requests > before.submitted_requests =>
                {
                    write_line(
                        runtime,
                        &format!(
                            "gpu-probe-submit device={} bytes={} source-api={} translation={} outcome=submitted payload={}",
                            device_path,
                            payload.len(),
                            source_api,
                            translation,
                            payload
                        ),
                    )
                }
                _ => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-submit device={} bytes={} outcome=error",
                        device_path,
                        payload.len()
                    ),
                ),
            }
        }
        Err(_) => write_line(
            runtime,
            &format!(
                "gpu-probe-submit device={} bytes={} outcome=error",
                device_path,
                payload.len()
            ),
        ),
    }
}

pub fn shell_gpu_present<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    frame_token: &str,
) -> Result<(), ExitCode> {
    let response = runtime
        .present_gpu_frame(device_path, frame_token.as_bytes())
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "gpu-present device={} opcode=0x{:08x} response=0x{:08x} frame={}",
            device_path, GPU_PRESENT_OPCODE, response, frame_token
        ),
    )
}

fn shell_gpu_probe_present<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    frame_token: &str,
) -> Result<(), ExitCode> {
    let before = runtime.inspect_device(device_path).ok();
    match runtime.open_path(device_path) {
        Ok(fd) => {
            let outcome = runtime.control(fd, GPU_PRESENT_OPCODE);
            let close_result = runtime.close(fd);
            let after = runtime.inspect_device(device_path).ok();
            match (outcome, close_result, before, after) {
                (Ok(response), Ok(()), Some(before), Some(after))
                    if after.submitted_requests > before.submitted_requests =>
                {
                    write_line(
                        runtime,
                        &format!(
                            "gpu-probe-present device={} opcode=0x{:08x} response=0x{:08x} outcome=presented frame={}",
                            device_path, GPU_PRESENT_OPCODE, response, frame_token
                        ),
                    )
                }
                _ => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-present device={} opcode=0x{:08x} outcome=error frame={}",
                        device_path, GPU_PRESENT_OPCODE, frame_token
                    ),
                ),
            }
        }
        Err(_) => write_line(
            runtime,
            &format!(
                "gpu-probe-present device={} opcode=0x{:08x} outcome=error frame={}",
                device_path, GPU_PRESENT_OPCODE, frame_token
            ),
        ),
    }
}

fn shell_gpu_driver_reset<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
) -> Result<(), ExitCode> {
    let fd = runtime.open_path(driver_path).map_err(|_| 234)?;
    let canceled = runtime
        .control(fd, GPU_DRIVER_RESET_OPCODE)
        .map_err(|_| 246)?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!(
            "gpu-driver-reset driver={} opcode=0x{:08x} canceled={}",
            driver_path, GPU_DRIVER_RESET_OPCODE, canceled
        ),
    )
}

fn shell_gpu_probe_driver_reset<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
) -> Result<(), ExitCode> {
    match runtime.open_path(driver_path) {
        Ok(fd) => {
            let outcome = runtime.control(fd, GPU_DRIVER_RESET_OPCODE);
            let close_result = runtime.close(fd);
            match (outcome, close_result) {
                (Ok(canceled), Ok(())) => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-driver-reset driver={} opcode=0x{:08x} canceled={} outcome=reset",
                        driver_path, GPU_DRIVER_RESET_OPCODE, canceled
                    ),
                ),
                _ => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-driver-reset driver={} opcode=0x{:08x} outcome=error",
                        driver_path, GPU_DRIVER_RESET_OPCODE
                    ),
                ),
            }
        }
        Err(_) => write_line(
            runtime,
            &format!(
                "gpu-probe-driver-reset driver={} opcode=0x{:08x} outcome=error",
                driver_path, GPU_DRIVER_RESET_OPCODE
            ),
        ),
    }
}

fn shell_gpu_driver_retire<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
) -> Result<(), ExitCode> {
    let fd = runtime.open_path(driver_path).map_err(|_| 234)?;
    let canceled = runtime
        .control(fd, GPU_DRIVER_RETIRE_OPCODE)
        .map_err(|_| 246)?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!(
            "gpu-driver-retire driver={} opcode=0x{:08x} canceled={}",
            driver_path, GPU_DRIVER_RETIRE_OPCODE, canceled
        ),
    )
}

fn shell_gpu_probe_driver_retire<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
) -> Result<(), ExitCode> {
    match runtime.open_path(driver_path) {
        Ok(fd) => {
            let outcome = runtime.control(fd, GPU_DRIVER_RETIRE_OPCODE);
            let close_result = runtime.close(fd);
            match (outcome, close_result) {
                (Ok(canceled), Ok(())) => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-driver-retire driver={} opcode=0x{:08x} canceled={} outcome=retired",
                        driver_path, GPU_DRIVER_RETIRE_OPCODE, canceled
                    ),
                ),
                _ => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-driver-retire driver={} opcode=0x{:08x} outcome=error",
                        driver_path, GPU_DRIVER_RETIRE_OPCODE
                    ),
                ),
            }
        }
        Err(_) => write_line(
            runtime,
            &format!(
                "gpu-probe-driver-retire driver={} opcode=0x{:08x} outcome=error",
                driver_path, GPU_DRIVER_RETIRE_OPCODE
            ),
        ),
    }
}

fn shell_gpu_driver_bind<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    driver_path: &str,
) -> Result<(), ExitCode> {
    runtime
        .bind_device_driver(device_path, driver_path)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "gpu-driver-bind device={} driver={}",
            device_path, driver_path
        ),
    )
}

fn shell_gpu_probe_driver_bind<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    driver_path: &str,
) -> Result<(), ExitCode> {
    let before_device = runtime.inspect_device(device_path).ok();
    let before_driver = runtime.inspect_driver(driver_path).ok();
    match runtime.bind_device_driver(device_path, driver_path) {
        Ok(()) => {
            let after_device = runtime.inspect_device(device_path).ok();
            let after_driver = runtime.inspect_driver(driver_path).ok();
            match (before_device, after_device, before_driver, after_driver) {
                (
                    Some(before_device),
                    Some(after_device),
                    Some(before_driver),
                    Some(after_driver),
                ) if before_device.state != after_device.state
                    || before_driver.bound_device_count != after_driver.bound_device_count =>
                {
                    write_line(
                        runtime,
                        &format!(
                            "gpu-probe-driver-bind device={} driver={} outcome=bound",
                            device_path, driver_path
                        ),
                    )
                }
                _ => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-driver-bind device={} driver={} outcome=error",
                        device_path, driver_path
                    ),
                ),
            }
        }
        Err(_) => write_line(
            runtime,
            &format!(
                "gpu-probe-driver-bind device={} driver={} outcome=error",
                device_path, driver_path
            ),
        ),
    }
}

fn shell_gpu_driver_unbind<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    runtime.unbind_device_driver(device_path).map_err(|_| 246)?;
    write_line(
        runtime,
        &format!("gpu-driver-unbind device={}", device_path),
    )
}

fn shell_gpu_probe_driver_unbind<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let before_device = runtime.inspect_device(device_path).ok();
    match runtime.unbind_device_driver(device_path) {
        Ok(()) => {
            let after_device = runtime.inspect_device(device_path).ok();
            match (before_device, after_device) {
                (Some(before_device), Some(after_device))
                    if before_device.state != after_device.state =>
                {
                    write_line(
                        runtime,
                        &format!(
                            "gpu-probe-driver-unbind device={} outcome=unbound",
                            device_path
                        ),
                    )
                }
                _ => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-driver-unbind device={} outcome=error",
                        device_path
                    ),
                ),
            }
        }
        Err(_) => write_line(
            runtime,
            &format!(
                "gpu-probe-driver-unbind device={} outcome=error",
                device_path
            ),
        ),
    }
}

fn gpu_request_kind_name(kind: u32) -> &'static str {
    match kind {
        0 => "read",
        1 => "write",
        2 => "control",
        _ => "unknown",
    }
}

fn gpu_request_state_name(state: u32) -> &'static str {
    match state {
        0 => "queued",
        1 => "inflight",
        2 => "completed",
        3 => "failed",
        4 => "canceled",
        _ => "unknown",
    }
}

fn shell_gpu_request<B: SyscallBackend>(
    runtime: &Runtime<B>,
    request_id: u64,
) -> Result<(), ExitCode> {
    let record: NativeDeviceRequestRecord = runtime
        .inspect_device_request(request_id)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "gpu-request id={} issuer={} kind={} state={} opcode=0x{:08x} buffer={} payload={} response={} submitted={} started={} completed={} frame={} api={} translation={}",
            request_id,
            record.issuer,
            gpu_request_kind_name(record.kind),
            gpu_request_state_name(record.state),
            record.opcode as u32,
            record.buffer_id,
            record.payload_len,
            record.response_len,
            record.submitted_tick,
            record.started_tick,
            record.completed_tick,
            fixed_text_field(&record.frame_tag),
            fixed_text_field(&record.source_api_name),
            fixed_text_field(&record.translation_label)
        ),
    )
}

fn shell_gpu_buffer_create<B: SyscallBackend>(
    runtime: &Runtime<B>,
    length: usize,
) -> Result<(), ExitCode> {
    let buffer_id = runtime.create_gpu_buffer(length).map_err(|_| 246)?;
    write_line(
        runtime,
        &format!("gpu-buffer-create id={} length={}", buffer_id, length),
    )
}

fn shell_gpu_buffer_write<B: SyscallBackend>(
    runtime: &Runtime<B>,
    buffer_id: u64,
    offset: usize,
    payload: &str,
) -> Result<(), ExitCode> {
    let written = runtime
        .write_gpu_buffer(buffer_id, offset, payload.as_bytes())
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "gpu-buffer-write id={} offset={} bytes={} payload={}",
            buffer_id, offset, written, payload
        ),
    )
}

fn shell_gpu_buffer<B: SyscallBackend>(
    runtime: &Runtime<B>,
    buffer_id: u64,
) -> Result<(), ExitCode> {
    let record = runtime.inspect_gpu_buffer(buffer_id).map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "gpu-buffer id={} owner={} length={} used={}",
            buffer_id, record.owner, record.length, record.used_len
        ),
    )
}

fn shell_gpu_scanout<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record: NativeGpuScanoutRecord =
        runtime.inspect_gpu_scanout(device_path).map_err(|_| 246)?;
    let mut buffer = vec![0u8; record.last_frame_len as usize];
    let copied = runtime
        .read_gpu_scanout_frame(device_path, &mut buffer)
        .map_err(|_| 246)?;
    buffer.truncate(copied);
    let frame = String::from_utf8_lossy(&buffer);
    write_line(
        runtime,
        &format!(
            "gpu-scanout device={} presented={} last-frame-bytes={} frame-tag={} api={} translation={} frame={}",
            device_path,
            record.presented_frames,
            copied,
            fixed_text_field(&record.last_frame_tag),
            fixed_text_field(&record.last_source_api_name),
            fixed_text_field(&record.last_translation_label),
            frame
        ),
    )
}

fn shell_gpu_perf<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record: NativeDeviceRecord = runtime.inspect_device(device_path).map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "gpu-perf device={} submitted={} completed={} total-latency={} max-latency={} total-queue-wait={} max-queue-wait={}",
            device_path,
            record.submitted_requests,
            record.completed_requests,
            record.total_latency_ticks,
            record.max_latency_ticks,
            record.total_queue_wait_ticks,
            record.max_queue_wait_ticks
        ),
    )
}

fn shell_gpu_submit_buffer<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    buffer_id: u64,
) -> Result<(), ExitCode> {
    let submitted = runtime
        .submit_gpu_buffer(device_path, buffer_id)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "gpu-submit-buffer device={} buffer={} submitted={}",
            device_path, buffer_id, submitted
        ),
    )
}

fn shell_gpu_probe_submit_buffer<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    buffer_id: u64,
) -> Result<(), ExitCode> {
    let before = runtime.inspect_device(device_path).ok();
    match runtime.submit_gpu_buffer(device_path, buffer_id) {
        Ok(submitted) => {
            let after = runtime.inspect_device(device_path).ok();
            match (before, after) {
                (Some(before), Some(after))
                    if after.submitted_requests > before.submitted_requests =>
                {
                    write_line(
                        runtime,
                        &format!(
                            "gpu-probe-submit-buffer device={} buffer={} submitted={} outcome=submitted",
                            device_path, buffer_id, submitted
                        ),
                    )
                }
                _ => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-submit-buffer device={} buffer={} outcome=error",
                        device_path, buffer_id
                    ),
                ),
            }
        }
        Err(_) => write_line(
            runtime,
            &format!(
                "gpu-probe-submit-buffer device={} buffer={} outcome=error",
                device_path, buffer_id
            ),
        ),
    }
}

fn shell_gpu_driver_read<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
) -> Result<(), ExitCode> {
    if let Ok(record) = runtime.inspect_driver(driver_path)
        && record.queued_requests == 0
        && record.in_flight_requests == 0
    {
        return write_line(
            runtime,
            &format!("gpu-driver-read driver={} outcome=empty", driver_path),
        );
    }
    let fd = runtime.open_path(driver_path).map_err(|_| 234)?;
    let mut buffer = [0u8; 512];
    let count = runtime.read(fd, &mut buffer).map_err(|_| 238)?;
    runtime.close(fd).map_err(|_| 240)?;
    if count == 0 {
        return write_line(
            runtime,
            &format!("gpu-driver-read driver={} outcome=empty", driver_path),
        );
    }
    let prefix_len = buffer[..count]
        .iter()
        .position(|byte| *byte == b'\n')
        .map(|index| index + 1)
        .unwrap_or(count);
    let header = core::str::from_utf8(&buffer[..prefix_len]).map_err(|_| 239)?;
    let payload = core::str::from_utf8(&buffer[prefix_len..count]).map_err(|_| 239)?;
    let header = header.trim_end();
    write_line(
        runtime,
        &format!(
            "gpu-driver-read driver={} outcome=request header={} payload={}",
            driver_path, header, payload
        ),
    )
}

fn complete_graphics_driver_request<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
    request_id: u64,
    payload: &str,
) -> Result<(), ExitCode> {
    let encoded = format!("request:{request_id}\n{payload}");
    let fd = runtime.open_path(driver_path).map_err(|_| 234)?;
    shell_write_all(runtime, fd, encoded.as_bytes())?;
    runtime.close(fd).map_err(|_| 240)
}

fn shell_gpu_complete<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
    payload: &str,
) -> Result<(), ExitCode> {
    let (source_api, translation) = parse_gfx_payload_translation_metadata(payload);
    let fd = runtime.open_path(driver_path).map_err(|_| 234)?;
    shell_write_all(runtime, fd, payload.as_bytes())?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!(
            "gpu-complete driver={} bytes={} source-api={} translation={} payload={}",
            driver_path,
            payload.len(),
            source_api,
            translation,
            payload
        ),
    )
}

fn parse_gfx_payload_translation_metadata(payload: &str) -> (&str, &str) {
    let mut source_api = "-";
    let mut translation = "-";
    for line in payload.lines() {
        if let Some(value) = line.strip_prefix("source-api=") {
            if !value.is_empty() {
                source_api = value;
            }
        } else if let Some(value) = line.strip_prefix("translation=") {
            if !value.is_empty() {
                translation = value;
            }
        }
    }
    (source_api, translation)
}

fn shell_gpu_complete_request<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
    request_id: u64,
    payload: &str,
) -> Result<(), ExitCode> {
    complete_graphics_driver_request(runtime, driver_path, request_id, payload)?;
    write_line(
        runtime,
        &format!(
            "gpu-complete-request driver={} request={} bytes={} payload={}",
            driver_path,
            request_id,
            payload.len(),
            payload
        ),
    )
}

fn shell_gpu_fail_request<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
    request_id: u64,
    payload: &str,
) -> Result<(), ExitCode> {
    let encoded = format!("failed-request:{request_id}\n{payload}");
    let fd = runtime.open_path(driver_path).map_err(|_| 234)?;
    shell_write_all(runtime, fd, encoded.as_bytes())?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!(
            "gpu-fail-request driver={} request={} bytes={} payload={}",
            driver_path,
            request_id,
            payload.len(),
            payload
        ),
    )
}

fn shell_gpu_cancel_request<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
    request_id: u64,
    payload: &str,
) -> Result<(), ExitCode> {
    let encoded = format!("cancel-request:{request_id}\n{payload}");
    let fd = runtime.open_path(driver_path).map_err(|_| 234)?;
    shell_write_all(runtime, fd, encoded.as_bytes())?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!(
            "gpu-cancel-request driver={} request={} bytes={} payload={}",
            driver_path,
            request_id,
            payload.len(),
            payload
        ),
    )
}

fn shell_gpu_probe_complete<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
    payload: &str,
) -> Result<(), ExitCode> {
    let before = runtime.inspect_driver(driver_path).ok();
    match runtime.open_path(driver_path) {
        Ok(fd) => {
            let outcome = shell_write_all(runtime, fd, payload.as_bytes());
            let close_result = runtime.close(fd);
            let after = runtime.inspect_driver(driver_path).ok();
            match (outcome, close_result, before, after) {
                (Ok(()), Ok(()), Some(before), Some(after))
                    if after.completed_requests > before.completed_requests =>
                {
                    write_line(
                        runtime,
                        &format!(
                            "gpu-probe-complete driver={} bytes={} outcome=completed payload={}",
                            driver_path,
                            payload.len(),
                            payload
                        ),
                    )
                }
                _ => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-complete driver={} bytes={} outcome=error",
                        driver_path,
                        payload.len()
                    ),
                ),
            }
        }
        Err(_) => write_line(
            runtime,
            &format!(
                "gpu-probe-complete driver={} bytes={} outcome=error",
                driver_path,
                payload.len()
            ),
        ),
    }
}

fn shell_gpu_probe_complete_request<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
    request_id: u64,
    payload: &str,
) -> Result<(), ExitCode> {
    let before = runtime.inspect_driver(driver_path).ok();
    let encoded = format!("request:{request_id}\n{payload}");
    match runtime.open_path(driver_path) {
        Ok(fd) => {
            let outcome = shell_write_all(runtime, fd, encoded.as_bytes());
            let close_result = runtime.close(fd);
            let after = runtime.inspect_driver(driver_path).ok();
            match (outcome, close_result, before, after) {
                (Ok(()), Ok(()), Some(before), Some(after))
                    if after.completed_requests > before.completed_requests =>
                {
                    write_line(
                        runtime,
                        &format!(
                            "gpu-probe-complete-request driver={} request={} bytes={} outcome=completed payload={}",
                            driver_path,
                            request_id,
                            payload.len(),
                            payload
                        ),
                    )
                }
                _ => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-complete-request driver={} request={} bytes={} outcome=error",
                        driver_path,
                        request_id,
                        payload.len()
                    ),
                ),
            }
        }
        Err(_) => write_line(
            runtime,
            &format!(
                "gpu-probe-complete-request driver={} request={} bytes={} outcome=error",
                driver_path,
                request_id,
                payload.len()
            ),
        ),
    }
}

fn shell_gpu_read<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    if let Ok(record) = runtime.inspect_device(device_path)
        && record.submitted_requests == 0
        && record.completed_requests == 0
    {
        return write_line(
            runtime,
            &format!("gpu-read device={} outcome=empty", device_path),
        );
    }
    let fd = runtime.open_path(device_path).map_err(|_| 234)?;
    let mut buffer = [0u8; 512];
    let count = runtime.read(fd, &mut buffer).map_err(|_| 238)?;
    runtime.close(fd).map_err(|_| 240)?;
    if count == 0 {
        return write_line(
            runtime,
            &format!("gpu-read device={} outcome=empty", device_path),
        );
    }
    let payload = core::str::from_utf8(&buffer[..count]).map_err(|_| 239)?;
    write_line(
        runtime,
        &format!(
            "gpu-read device={} bytes={} payload={}",
            device_path, count, payload
        ),
    )
}

fn shell_gpu_probe_fail_request<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
    request_id: u64,
    payload: &str,
) -> Result<(), ExitCode> {
    let before = runtime.inspect_driver(driver_path).ok();
    let encoded = format!("failed-request:{request_id}\n{payload}");
    match runtime.open_path(driver_path) {
        Ok(fd) => {
            let outcome = shell_write_all(runtime, fd, encoded.as_bytes());
            let close_result = runtime.close(fd);
            let after = runtime.inspect_driver(driver_path).ok();
            match (outcome, close_result, before, after) {
                (Ok(()), Ok(()), Some(before), Some(after))
                    if after.completed_requests > before.completed_requests =>
                {
                    write_line(
                        runtime,
                        &format!(
                            "gpu-probe-fail-request driver={} request={} bytes={} outcome=failed payload={}",
                            driver_path,
                            request_id,
                            payload.len(),
                            payload
                        ),
                    )
                }
                _ => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-fail-request driver={} request={} bytes={} outcome=error",
                        driver_path,
                        request_id,
                        payload.len()
                    ),
                ),
            }
        }
        Err(_) => write_line(
            runtime,
            &format!(
                "gpu-probe-fail-request driver={} request={} bytes={} outcome=error",
                driver_path,
                request_id,
                payload.len()
            ),
        ),
    }
}

fn shell_gpu_probe_cancel_request<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
    request_id: u64,
    payload: &str,
) -> Result<(), ExitCode> {
    let before = runtime.inspect_device_request(request_id).ok();
    let encoded = format!("cancel-request:{request_id}\n{payload}");
    match runtime.open_path(driver_path) {
        Ok(fd) => {
            let outcome = shell_write_all(runtime, fd, encoded.as_bytes());
            let close_result = runtime.close(fd);
            let after = runtime.inspect_device_request(request_id).ok();
            match (outcome, close_result, before, after) {
                (Ok(()), Ok(()), Some(before), Some(after))
                    if before.state != 4 && after.state == 4 =>
                {
                    write_line(
                        runtime,
                        &format!(
                            "gpu-probe-cancel-request driver={} request={} bytes={} outcome=canceled payload={}",
                            driver_path,
                            request_id,
                            payload.len(),
                            payload
                        ),
                    )
                }
                _ => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-cancel-request driver={} request={} bytes={} outcome=error",
                        driver_path,
                        request_id,
                        payload.len()
                    ),
                ),
            }
        }
        Err(_) => write_line(
            runtime,
            &format!(
                "gpu-probe-cancel-request driver={} request={} bytes={} outcome=error",
                driver_path,
                request_id,
                payload.len()
            ),
        ),
    }
}

fn shell_watch_gpu_lease<B: SyscallBackend>(
    runtime: &Runtime<B>,
    resource: usize,
    token: u64,
) -> Result<usize, ExitCode> {
    let queue_fd = runtime
        .create_event_queue(NativeEventQueueMode::Kqueue)
        .map_err(|_| 246)?;
    runtime
        .watch_resource_events(
            queue_fd, resource, token, true, true, true, true, true, true, POLLPRI,
        )
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "gpu-lease-watch queue={} resource={} token={}",
            queue_fd, resource, token
        ),
    )?;
    Ok(queue_fd)
}

fn shell_remove_gpu_lease_watch<B: SyscallBackend>(
    runtime: &Runtime<B>,
    queue_fd: usize,
    resource: usize,
    token: u64,
) -> Result<(), ExitCode> {
    runtime
        .remove_resource_events(queue_fd, resource, token)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "gpu-lease-unwatch queue={} resource={} token={}",
            queue_fd, resource, token
        ),
    )
}

fn shell_wait_gpu_lease<B: SyscallBackend>(
    runtime: &Runtime<B>,
    queue_fd: usize,
) -> Result<(), ExitCode> {
    let mut records = [NativeEventRecord {
        token: 0,
        events: 0,
        source_kind: 0,
        source_arg0: 0,
        source_arg1: 0,
        source_arg2: 0,
        detail0: 0,
        detail1: 0,
    }; 16];
    let count = runtime
        .wait_event_queue(queue_fd, &mut records)
        .map_err(|_| 246)?;
    for record in &records[..count] {
        let kind = if record.source_kind == NativeEventSourceKind::Resource as u32 {
            match record.detail0 {
                0 => "claimed",
                1 => "queued",
                2 => "canceled",
                3 => "released",
                4 => "handed-off",
                5 => "revoked",
                _ => "unknown",
            }
        } else {
            "unknown"
        };
        write_line(
            runtime,
            &format!(
                "gpu-lease-event queue={} token={} resource={} contract={} kind={} events=0x{:x}",
                queue_fd, record.token, record.source_arg0, record.source_arg1, kind, record.events
            ),
        )?;
    }
    Ok(())
}

fn shell_render_driver<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
) -> Result<(), ExitCode> {
    let record: NativeDriverRecord = runtime.inspect_driver(driver_path).map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "driver path={} state={} bound-devices={} queued={} inflight={} completed={} last-request={} last-frame={} last-api={} last-translation={} last-terminal-request={} last-terminal-state={} last-terminal-frame={} last-terminal-api={} last-terminal-translation={}",
            driver_path,
            record.state,
            record.bound_device_count,
            record.queued_requests,
            record.in_flight_requests,
            record.completed_requests,
            record.last_completed_request_id,
            fixed_text_field(&record.last_completed_frame_tag),
            fixed_text_field(&record.last_completed_source_api_name),
            fixed_text_field(&record.last_completed_translation_label),
            record.last_terminal_request_id,
            gpu_request_state_name(record.last_terminal_state),
            fixed_text_field(&record.last_terminal_frame_tag),
            fixed_text_field(&record.last_terminal_source_api_name),
            fixed_text_field(&record.last_terminal_translation_label)
        ),
    )
}

fn shell_watch_graphics_events<B: SyscallBackend>(
    runtime: &Runtime<B>,
    queue_fd: usize,
    device_path: &str,
    token: u64,
) -> Result<(), ExitCode> {
    runtime
        .watch_graphics_events(
            queue_fd,
            device_path,
            token,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            POLLPRI,
        )
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "gpu-watch queue={} device={} token={}",
            queue_fd, device_path, token
        ),
    )
}

fn shell_remove_graphics_watch<B: SyscallBackend>(
    runtime: &Runtime<B>,
    queue_fd: usize,
    device_path: &str,
    token: u64,
) -> Result<(), ExitCode> {
    runtime
        .remove_graphics_events(queue_fd, device_path, token)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "gpu-unwatch queue={} device={} token={}",
            queue_fd, device_path, token
        ),
    )
}
