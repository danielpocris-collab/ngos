use super::*;

pub(super) fn try_handle_network_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    variables: &mut Vec<ShellVariable>,
    line: &str,
    last_status: &mut i32,
) -> Option<Result<(), ExitCode>> {
    if let Some(path) = line.strip_prefix("netif ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        return Some(shell_render_network_interface(runtime, &resolved).map_err(|_| 205));
    }
    if let Some(rest) = line.strip_prefix("net-config ") {
        let mut parts = rest.split_whitespace();
        let path = match parts.next() {
            Some(path) => resolve_shell_path(cwd, path),
            None => {
                let _ = write_line(
                    runtime,
                    "usage: net-config <device> <addr> <netmask> <gateway>",
                );
                return Some(Err(2));
            }
        };
        let addr = match parts.next().and_then(parse_ipv4) {
            Some(addr) => addr,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: net-config <device> <addr> <netmask> <gateway>",
                );
                return Some(Err(2));
            }
        };
        let netmask = match parts.next().and_then(parse_ipv4) {
            Some(netmask) => netmask,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: net-config <device> <addr> <netmask> <gateway>",
                );
                return Some(Err(2));
            }
        };
        let gateway = match parts.next().and_then(parse_ipv4) {
            Some(gateway) => gateway,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: net-config <device> <addr> <netmask> <gateway>",
                );
                return Some(Err(2));
            }
        };
        return Some(shell_net_config(runtime, &path, addr, netmask, gateway).map_err(|_| 205));
    }
    if let Some(rest) = line.strip_prefix("net-link ") {
        let mut parts = rest.split_whitespace();
        let Some(device_path) = parts.next() else {
            let _ = write_line(runtime, "usage: net-link <device> <up|down>");
            return Some(Err(2));
        };
        let Some(state) = parts.next() else {
            let _ = write_line(runtime, "usage: net-link <device> <up|down>");
            return Some(Err(2));
        };
        let link_up = match state {
            "up" => true,
            "down" => false,
            _ => {
                let _ = write_line(runtime, "usage: net-link <device> <up|down>");
                return Some(Err(2));
            }
        };
        *last_status = match shell_set_net_link(runtime, device_path, link_up) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("net-admin ") {
        let mut parts = rest.split_whitespace();
        let Some(device_path) = parts.next() else {
            let _ = write_line(
                runtime,
                "usage: net-admin <device> <mtu> <tx-cap> <rx-cap> <inflight-limit> <up|down> <promisc|nopromisc>",
            );
            return Some(Err(2));
        };
        let Some(mtu) = parse_usize_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: net-admin <device> <mtu> <tx-cap> <rx-cap> <inflight-limit> <up|down> <promisc|nopromisc>",
            );
            return Some(Err(2));
        };
        let Some(tx_cap) = parse_usize_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: net-admin <device> <mtu> <tx-cap> <rx-cap> <inflight-limit> <up|down> <promisc|nopromisc>",
            );
            return Some(Err(2));
        };
        let Some(rx_cap) = parse_usize_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: net-admin <device> <mtu> <tx-cap> <rx-cap> <inflight-limit> <up|down> <promisc|nopromisc>",
            );
            return Some(Err(2));
        };
        let Some(inflight) = parse_usize_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: net-admin <device> <mtu> <tx-cap> <rx-cap> <inflight-limit> <up|down> <promisc|nopromisc>",
            );
            return Some(Err(2));
        };
        let Some(admin_raw) = parts.next() else {
            let _ = write_line(
                runtime,
                "usage: net-admin <device> <mtu> <tx-cap> <rx-cap> <inflight-limit> <up|down> <promisc|nopromisc>",
            );
            return Some(Err(2));
        };
        let Some(promisc_raw) = parts.next() else {
            let _ = write_line(
                runtime,
                "usage: net-admin <device> <mtu> <tx-cap> <rx-cap> <inflight-limit> <up|down> <promisc|nopromisc>",
            );
            return Some(Err(2));
        };
        let admin_up = match admin_raw {
            "up" => true,
            "down" => false,
            _ => {
                let _ = write_line(
                    runtime,
                    "usage: net-admin <device> <mtu> <tx-cap> <rx-cap> <inflight-limit> <up|down> <promisc|nopromisc>",
                );
                return Some(Err(2));
            }
        };
        let promiscuous = match promisc_raw {
            "promisc" => true,
            "nopromisc" => false,
            _ => {
                let _ = write_line(
                    runtime,
                    "usage: net-admin <device> <mtu> <tx-cap> <rx-cap> <inflight-limit> <up|down> <promisc|nopromisc>",
                );
                return Some(Err(2));
            }
        };
        *last_status = match shell_net_admin(
            runtime,
            device_path,
            mtu,
            tx_cap,
            rx_cap,
            inflight,
            admin_up,
            promiscuous,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("udp-bind ") {
        let mut parts = rest.split_whitespace();
        let socket_path = match parts.next() {
            Some(path) => resolve_shell_path(cwd, path),
            None => {
                let _ = write_line(
                    runtime,
                    "usage: udp-bind <socket> <device> <local-port> <remote-ip> <remote-port>",
                );
                return Some(Err(2));
            }
        };
        let device_path = match parts.next() {
            Some(path) => resolve_shell_path(cwd, path),
            None => {
                let _ = write_line(
                    runtime,
                    "usage: udp-bind <socket> <device> <local-port> <remote-ip> <remote-port>",
                );
                return Some(Err(2));
            }
        };
        let local_port = match parse_u16_arg(parts.next()) {
            Some(port) => port,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: udp-bind <socket> <device> <local-port> <remote-ip> <remote-port>",
                );
                return Some(Err(2));
            }
        };
        let remote_ip = match parts.next().and_then(parse_ipv4) {
            Some(addr) => addr,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: udp-bind <socket> <device> <local-port> <remote-ip> <remote-port>",
                );
                return Some(Err(2));
            }
        };
        let remote_port = match parse_u16_arg(parts.next()) {
            Some(port) => port,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: udp-bind <socket> <device> <local-port> <remote-ip> <remote-port>",
                );
                return Some(Err(2));
            }
        };
        return Some(
            shell_udp_bind(
                runtime,
                &socket_path,
                &device_path,
                local_port,
                remote_ip,
                remote_port,
            )
            .map_err(|_| 205),
        );
    }
    if let Some(rest) = line.strip_prefix("udp-connect ") {
        let mut parts = rest.split_whitespace();
        let socket_path = match parts.next() {
            Some(path) => resolve_shell_path(cwd, path),
            None => {
                let _ = write_line(
                    runtime,
                    "usage: udp-connect <socket> <remote-ip> <remote-port>",
                );
                return Some(Err(2));
            }
        };
        let remote_ip = match parts.next().and_then(parse_ipv4) {
            Some(ip) => ip,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: udp-connect <socket> <remote-ip> <remote-port>",
                );
                return Some(Err(2));
            }
        };
        let remote_port = match parse_u16_arg(parts.next()) {
            Some(port) => port,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: udp-connect <socket> <remote-ip> <remote-port>",
                );
                return Some(Err(2));
            }
        };
        *last_status = match shell_udp_connect(runtime, &socket_path, remote_ip, remote_port) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(path) = line.strip_prefix("netsock ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        return Some(shell_render_network_socket(runtime, &resolved).map_err(|_| 205));
    }
    if let Some(rest) = line.strip_prefix("queue-create ") {
        let mode = match rest.trim() {
            "kqueue" => Some(NativeEventQueueMode::Kqueue),
            "epoll" => Some(NativeEventQueueMode::Epoll),
            _ => None,
        };
        let Some(mode) = mode else {
            let _ = write_line(runtime, "usage: queue-create <kqueue|epoll>");
            return Some(Err(2));
        };
        *last_status = match shell_create_event_queue(runtime, mode) {
            Ok(fd) => {
                shell_set_variable(variables, "LAST_QUEUE_FD", fd.to_string());
                0
            }
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("net-watch ") {
        let mut parts = rest.split_whitespace();
        let Some(queue_fd) = parse_usize_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: net-watch <queue-fd> <device> <token> [socket]",
            );
            return Some(Err(2));
        };
        let Some(device_path) = parts.next() else {
            let _ = write_line(
                runtime,
                "usage: net-watch <queue-fd> <device> <token> [socket]",
            );
            return Some(Err(2));
        };
        let Some(token) = parse_u64_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: net-watch <queue-fd> <device> <token> [socket]",
            );
            return Some(Err(2));
        };
        let socket_path = parts.next();
        *last_status =
            match shell_watch_network_events(runtime, queue_fd, device_path, socket_path, token) {
                Ok(()) => 0,
                Err(code) => code,
            };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("net-unwatch ") {
        let mut parts = rest.split_whitespace();
        let Some(queue_fd) = parse_usize_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: net-unwatch <queue-fd> <device> <token> [socket]",
            );
            return Some(Err(2));
        };
        let Some(device_path) = parts.next() else {
            let _ = write_line(
                runtime,
                "usage: net-unwatch <queue-fd> <device> <token> [socket]",
            );
            return Some(Err(2));
        };
        let Some(token) = parse_u64_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: net-unwatch <queue-fd> <device> <token> [socket]",
            );
            return Some(Err(2));
        };
        let socket_path = parts.next();
        *last_status =
            match shell_remove_network_watch(runtime, queue_fd, device_path, socket_path, token) {
                Ok(()) => 0,
                Err(code) => code,
            };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("queue-wait ") {
        let Some(queue_fd) = parse_usize_arg(Some(rest.trim())) else {
            let _ = write_line(runtime, "usage: queue-wait <queue-fd>");
            return Some(Err(2));
        };
        *last_status = match shell_wait_event_queue(runtime, queue_fd) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("net-send ") {
        let mut parts = rest.splitn(2, char::is_whitespace);
        let socket = match parts.next() {
            Some(path) if !path.is_empty() => resolve_shell_path(cwd, path),
            _ => {
                let _ = write_line(runtime, "usage: net-send <socket> <payload>");
                return Some(Err(2));
            }
        };
        let payload = match parts.next() {
            Some(payload) if !payload.trim_start().is_empty() => payload.trim_start(),
            _ => {
                let _ = write_line(runtime, "usage: net-send <socket> <payload>");
                return Some(Err(2));
            }
        };
        return Some(shell_net_send(runtime, &socket, payload).map_err(|_| 205));
    }
    if let Some(rest) = line.strip_prefix("net-sendto ") {
        let mut parts = rest.splitn(4, char::is_whitespace);
        let socket = match parts.next() {
            Some(path) if !path.is_empty() => resolve_shell_path(cwd, path),
            _ => {
                let _ = write_line(
                    runtime,
                    "usage: net-sendto <socket> <remote-ip> <remote-port> <payload>",
                );
                return Some(Err(2));
            }
        };
        let remote_ip = match parts.next().and_then(parse_ipv4) {
            Some(ip) => ip,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: net-sendto <socket> <remote-ip> <remote-port> <payload>",
                );
                return Some(Err(2));
            }
        };
        let remote_port = match parse_u16_arg(parts.next()) {
            Some(port) => port,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: net-sendto <socket> <remote-ip> <remote-port> <payload>",
                );
                return Some(Err(2));
            }
        };
        let payload = match parts.next() {
            Some(payload) if !payload.trim_start().is_empty() => payload.trim_start(),
            _ => {
                let _ = write_line(
                    runtime,
                    "usage: net-sendto <socket> <remote-ip> <remote-port> <payload>",
                );
                return Some(Err(2));
            }
        };
        *last_status = match shell_net_sendto(runtime, &socket, remote_ip, remote_port, payload) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(path) = line.strip_prefix("net-recv ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        return Some(shell_net_recv(runtime, &resolved).map_err(|_| 205));
    }
    if let Some(path) = line.strip_prefix("net-recvfrom ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        return Some(shell_net_recvfrom(runtime, &resolved).map_err(|_| 205));
    }
    if let Some(path) = line.strip_prefix("net-driver-read ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        return Some(shell_driver_read(runtime, &resolved).map_err(|_| 205));
    }
    if let Some(rest) = line.strip_prefix("net-complete ") {
        let mut parts = rest.split_whitespace();
        let driver_path = match parts.next() {
            Some(path) => resolve_shell_path(cwd, path),
            None => {
                let _ = write_line(runtime, "usage: net-complete <driver> <count>");
                return Some(Err(2));
            }
        };
        let Some(count) = parse_usize_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: net-complete <driver> <count>");
            return Some(Err(2));
        };
        *last_status = match shell_complete_net_tx(runtime, &driver_path, count) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("net-inject-udp ") {
        let mut parts = rest.splitn(6, char::is_whitespace);
        let driver_path = match parts.next() {
            Some(path) if !path.is_empty() => resolve_shell_path(cwd, path),
            _ => {
                let _ = write_line(
                    runtime,
                    "usage: net-inject-udp <driver> <src-ip> <src-port> <dst-ip> <dst-port> <payload>",
                );
                return Some(Err(2));
            }
        };
        let src_ip = match parts.next().and_then(parse_ipv4) {
            Some(ip) => ip,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: net-inject-udp <driver> <src-ip> <src-port> <dst-ip> <dst-port> <payload>",
                );
                return Some(Err(2));
            }
        };
        let src_port = match parse_u16_arg(parts.next()) {
            Some(port) => port,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: net-inject-udp <driver> <src-ip> <src-port> <dst-ip> <dst-port> <payload>",
                );
                return Some(Err(2));
            }
        };
        let dst_ip = match parts.next().and_then(parse_ipv4) {
            Some(ip) => ip,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: net-inject-udp <driver> <src-ip> <src-port> <dst-ip> <dst-port> <payload>",
                );
                return Some(Err(2));
            }
        };
        let dst_port = match parse_u16_arg(parts.next()) {
            Some(port) => port,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: net-inject-udp <driver> <src-ip> <src-port> <dst-ip> <dst-port> <payload>",
                );
                return Some(Err(2));
            }
        };
        let payload = match parts.next() {
            Some(payload) if !payload.is_empty() => payload,
            _ => {
                let _ = write_line(
                    runtime,
                    "usage: net-inject-udp <driver> <src-ip> <src-port> <dst-ip> <dst-port> <payload>",
                );
                return Some(Err(2));
            }
        };
        return Some(
            shell_driver_inject_udp(
                runtime,
                &driver_path,
                src_ip,
                src_port,
                dst_ip,
                dst_port,
                payload,
            )
            .map_err(|_| 205),
        );
    }
    if let Some(rest) = line.strip_prefix("poll-path ") {
        let mut parts = rest.split_whitespace();
        let path = match parts.next() {
            Some(path) => resolve_shell_path(cwd, path),
            None => {
                let _ = write_line(runtime, "usage: poll-path <path> <read|write|readwrite>");
                return Some(Err(2));
            }
        };
        let interest = match parts.next() {
            Some("read") => POLLIN,
            Some("write") => POLLOUT,
            Some("readwrite") => POLLIN | POLLOUT,
            _ => {
                let _ = write_line(runtime, "usage: poll-path <path> <read|write|readwrite>");
                return Some(Err(2));
            }
        };
        return Some(shell_poll_path(runtime, &path, interest).map_err(|_| 205));
    }
    None
}
