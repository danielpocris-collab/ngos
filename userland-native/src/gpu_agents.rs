use super::*;

pub(super) fn try_handle_gpu_agent_command<B: SyscallBackend>(
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
