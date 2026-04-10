//! Canonical subsystem role:
//! - subsystem: native proof and smoke orchestration
//! - owner layer: Layer 3
//! - semantic owner: `ngos-shell-proof`
//! - truth path role: execution and reporting of end-to-end proof fronts over
//!   canonical system surfaces
//!
//! Canonical contract families handled here:
//! - proof command contracts
//! - boot proof selection contracts
//! - smoke demonstration contracts
//!
//! This module may orchestrate proofs and report observable outcomes, but it
//! must not redefine subsystem truth beyond the evidence it collects.

#![no_std]
extern crate alloc;

use alloc::string::String;

use ngos_user_abi::{
    ExitCode, NativeDeviceRecord, NativeDriverRecord, NativeNetworkInterfaceRecord,
    NativeNetworkSocketRecord, NativeStorageVolumeRecord, SyscallBackend,
};
use ngos_user_runtime::Runtime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootProofKind {
    Vm,
    Wasm,
    Shell,
    Scheduler,
    ProcessExec,
    Vfs,
    Render3d,
    CompatGfx,
    CompatAudio,
    CompatInput,
    CompatLoader,
    CompatAbi,
    CompatForeign,
    Bus,
    Network,
    NetworkHardware,
    NetworkHardwareInterface,
    NetworkHardwareRx,
    NetworkHardwareUdpRx,
    NetworkHardwareTx,
    NetworkHardwareUdpTx,
    DeviceRuntime,
    StorageCommit,
    StorageRecover,
    StorageCorrupt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceSmokeKind {
    Smoke,
    Shell,
    DeviceRuntime,
    Bus,
    Vfs,
    Wasm,
    CompatGfx,
    CompatAudio,
    CompatInput,
    CompatLoader,
    CompatAbi,
    Network,
}

impl BootProofKind {
    pub fn parse_env(value: Option<&str>) -> Option<Self> {
        match value? {
            "vm" => Some(Self::Vm),
            "wasm" => Some(Self::Wasm),
            "shell" => Some(Self::Shell),
            "scheduler" => Some(Self::Scheduler),
            "process-exec" => Some(Self::ProcessExec),
            "vfs" => Some(Self::Vfs),
            "render3d" => Some(Self::Render3d),
            "compat-gfx" => Some(Self::CompatGfx),
            "compat-audio" => Some(Self::CompatAudio),
            "compat-input" => Some(Self::CompatInput),
            "compat-loader" => Some(Self::CompatLoader),
            "compat-abi" => Some(Self::CompatAbi),
            "compat-foreign" => Some(Self::CompatForeign),
            "bus" => Some(Self::Bus),
            "network" => Some(Self::Network),
            "network-hardware" => Some(Self::NetworkHardware),
            "network-hardware-interface" => Some(Self::NetworkHardwareInterface),
            "network-hardware-rx" => Some(Self::NetworkHardwareRx),
            "network-hardware-udp-rx" => Some(Self::NetworkHardwareUdpRx),
            "network-hardware-tx" => Some(Self::NetworkHardwareTx),
            "network-hardware-udp-tx" => Some(Self::NetworkHardwareUdpTx),
            "device-runtime" => Some(Self::DeviceRuntime),
            "storage-commit" => Some(Self::StorageCommit),
            "storage-recover" => Some(Self::StorageRecover),
            "storage-corrupt" => Some(Self::StorageCorrupt),
            _ => None,
        }
    }

    pub fn marker(self) -> &'static str {
        match self {
            Self::Vm => "boot.proof=vm",
            Self::Wasm => "boot.proof=wasm",
            Self::Shell => "boot.proof=shell",
            Self::Scheduler => "boot.proof=scheduler",
            Self::ProcessExec => "boot.proof=process-exec",
            Self::Vfs => "boot.proof=vfs",
            Self::Render3d => "boot.proof=render3d",
            Self::CompatGfx => "boot.proof=compat-gfx",
            Self::CompatAudio => "boot.proof=compat-audio",
            Self::CompatInput => "boot.proof=compat-input",
            Self::CompatLoader => "boot.proof=compat-loader",
            Self::CompatAbi => "boot.proof=compat-abi",
            Self::CompatForeign => "boot.proof=compat-foreign",
            Self::Bus => "boot.proof=bus",
            Self::Network => "boot.proof=network",
            Self::NetworkHardware => "boot.proof=network-hardware",
            Self::NetworkHardwareInterface => "boot.proof=network-hardware-interface",
            Self::NetworkHardwareRx => "boot.proof=network-hardware-rx",
            Self::NetworkHardwareUdpRx => "boot.proof=network-hardware-udp-rx",
            Self::NetworkHardwareTx => "boot.proof=network-hardware-tx",
            Self::NetworkHardwareUdpTx => "boot.proof=network-hardware-udp-tx",
            Self::DeviceRuntime => "boot.proof=device-runtime",
            Self::StorageCommit => "boot.proof=storage-commit",
            Self::StorageRecover => "boot.proof=storage-recover",
            Self::StorageCorrupt => "boot.proof=storage-corrupt",
        }
    }
}

impl SurfaceSmokeKind {
    pub fn parse_command(line: &str) -> Option<Self> {
        match line {
            "smoke" => Some(Self::Smoke),
            "shell-smoke" => Some(Self::Shell),
            "device-runtime-smoke" => Some(Self::DeviceRuntime),
            "bus-smoke" => Some(Self::Bus),
            "vfs-smoke" => Some(Self::Vfs),
            "wasm-smoke" => Some(Self::Wasm),
            "compat-gfx-smoke" => Some(Self::CompatGfx),
            "compat-audio-smoke" => Some(Self::CompatAudio),
            "compat-input-smoke" => Some(Self::CompatInput),
            "compat-loader-smoke" => Some(Self::CompatLoader),
            "compat-abi-smoke" => Some(Self::CompatAbi),
            "network-smoke" => Some(Self::Network),
            _ => None,
        }
    }

    pub fn marker(self) -> &'static str {
        match self {
            Self::Smoke => "smoke-ok",
            Self::Shell => "shell-smoke-ok",
            Self::DeviceRuntime => "device-runtime-smoke-ok",
            Self::Bus => "bus-smoke-ok",
            Self::Vfs => "vfs-smoke-ok",
            Self::Wasm => "wasm-smoke-ok",
            Self::CompatGfx => "compat-gfx-smoke-ok",
            Self::CompatAudio => "compat-audio-smoke-ok",
            Self::CompatInput => "compat-input-smoke-ok",
            Self::CompatLoader => "compat-loader-smoke-ok",
            Self::CompatAbi => "compat-abi-smoke-ok",
            Self::Network => "network-smoke-ok",
        }
    }
}

pub trait BootProofDispatcher {
    type Context;
    type Output;

    fn run_vm(&mut self) -> Self::Output;
    fn run_wasm(&mut self) -> Self::Output;
    fn run_shell(&mut self, context: &Self::Context) -> Self::Output;
    fn run_scheduler(&mut self) -> Self::Output;
    fn run_process_exec(&mut self) -> Self::Output;
    fn run_vfs(&mut self) -> Self::Output;
    fn run_render3d(&mut self) -> Self::Output;
    fn run_compat_gfx(&mut self) -> Self::Output;
    fn run_compat_audio(&mut self) -> Self::Output;
    fn run_compat_input(&mut self) -> Self::Output;
    fn run_compat_loader(&mut self) -> Self::Output;
    fn run_compat_abi(&mut self) -> Self::Output;
    fn run_compat_foreign(&mut self) -> Self::Output;
    fn run_bus(&mut self) -> Self::Output;
    fn run_network(&mut self) -> Self::Output;
    fn run_network_hardware(&mut self) -> Self::Output;
    fn run_network_hardware_interface(&mut self) -> Self::Output;
    fn run_network_hardware_rx(&mut self) -> Self::Output;
    fn run_network_hardware_udp_rx(&mut self) -> Self::Output;
    fn run_network_hardware_tx(&mut self) -> Self::Output;
    fn run_network_hardware_udp_tx(&mut self) -> Self::Output;
    fn run_device_runtime(&mut self) -> Self::Output;
    fn run_storage_commit(&mut self) -> Self::Output;
    fn run_storage_recover(&mut self) -> Self::Output;
    fn run_storage_corrupt(&mut self) -> Self::Output;
}

pub trait SurfaceSmokeDispatcher {
    type Output;

    fn run_smoke(&mut self) -> Self::Output;
    fn run_shell(&mut self) -> Self::Output;
    fn run_device_runtime(&mut self) -> Self::Output;
    fn run_bus(&mut self) -> Self::Output;
    fn run_vfs(&mut self) -> Self::Output;
    fn run_wasm(&mut self) -> Self::Output;
    fn run_compat_gfx(&mut self) -> Self::Output;
    fn run_compat_audio(&mut self) -> Self::Output;
    fn run_compat_input(&mut self) -> Self::Output;
    fn run_compat_loader(&mut self) -> Self::Output;
    fn run_compat_abi(&mut self) -> Self::Output;
    fn run_network(&mut self) -> Self::Output;
}

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

fn render_device_runtime_line(
    path: &str,
    device: NativeDeviceRecord,
    driver: NativeDriverRecord,
) -> String {
    alloc::format!(
        "device.runtime.smoke.{} device={} class={} state={} submitted={} completed={} driver-state={} driver-completed={} outcome=ok",
        device_class_name(device.class),
        path,
        device_class_name(device.class),
        device.state,
        device.submitted_requests,
        device.completed_requests,
        driver.state,
        driver.completed_requests
    )
}

fn render_storage_runtime_line(
    path: &str,
    storage: NativeStorageVolumeRecord,
    driver: NativeDriverRecord,
) -> String {
    alloc::format!(
        "device.runtime.smoke.storage device={} valid={} generation={} files={} dirs={} symlinks={} driver-state={} driver-completed={} outcome=ok",
        path,
        storage.valid,
        storage.generation,
        storage.mapped_file_count,
        storage.mapped_directory_count,
        storage.mapped_symlink_count,
        driver.state,
        driver.completed_requests
    )
}

fn render_ipv4(bytes: [u8; 4]) -> String {
    alloc::format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
}

fn render_network_interface_runtime_line(
    path: &str,
    interface: NativeNetworkInterfaceRecord,
    driver: NativeDriverRecord,
) -> String {
    alloc::format!(
        "device.runtime.smoke.network.interface device={} admin={} link={} mtu={} tx={} rx={} sockets={} driver-state={} driver-completed={} outcome=ok",
        path,
        interface.admin_up,
        interface.link_up,
        interface.mtu,
        interface.tx_packets,
        interface.rx_packets,
        interface.attached_socket_count,
        driver.state,
        driver.completed_requests
    )
}

fn render_network_socket_runtime_line(
    path: &str,
    socket: NativeNetworkSocketRecord,
    driver: NativeDriverRecord,
) -> String {
    alloc::format!(
        "device.runtime.smoke.network.socket socket={} connected={} local={}:{} remote={}:{} rx-depth={} tx={} rx={} dropped={} driver-state={} driver-completed={} outcome=ok",
        path,
        socket.connected,
        render_ipv4(socket.local_ipv4),
        socket.local_port,
        render_ipv4(socket.remote_ipv4),
        socket.remote_port,
        socket.rx_depth,
        socket.tx_packets,
        socket.rx_packets,
        socket.dropped_packets,
        driver.state,
        driver.completed_requests
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceRuntimeSmokeReport {
    pub graphics_line: String,
    pub audio_line: String,
    pub input_line: String,
    pub network_interface_line: String,
    pub network_socket_line: String,
    pub storage_line: String,
    pub final_marker: &'static str,
}

pub fn build_device_runtime_smoke_report<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> Result<DeviceRuntimeSmokeReport, ExitCode> {
    let gpu = runtime.inspect_device("/dev/gpu0").map_err(|_| 171)?;
    let gpu_driver = runtime.inspect_driver("/drv/gpu0").map_err(|_| 172)?;

    let audio = runtime.inspect_device("/dev/audio0").map_err(|_| 174)?;
    let audio_driver = runtime.inspect_driver("/drv/audio0").map_err(|_| 175)?;

    let input = runtime.inspect_device("/dev/input0").map_err(|_| 177)?;
    let input_driver = runtime.inspect_driver("/drv/input0").map_err(|_| 178)?;

    let network_interface = runtime
        .inspect_network_interface("/dev/net1")
        .map_err(|_| 179)?;
    let network_interface_driver = runtime.inspect_driver("/drv/net1").map_err(|_| 180)?;

    let network_socket = runtime
        .inspect_network_socket("/run/net1.sock")
        .map_err(|_| 181)?;
    let network_socket_driver = runtime.inspect_driver("/drv/net1").map_err(|_| 182)?;

    let storage = runtime
        .inspect_storage_volume("/dev/storage0")
        .map_err(|_| 183)?;
    let storage_driver = runtime.inspect_driver("/drv/storage0").map_err(|_| 184)?;

    Ok(DeviceRuntimeSmokeReport {
        graphics_line: render_device_runtime_line("/dev/gpu0", gpu, gpu_driver),
        audio_line: render_device_runtime_line("/dev/audio0", audio, audio_driver),
        input_line: render_device_runtime_line("/dev/input0", input, input_driver),
        network_interface_line: render_network_interface_runtime_line(
            "/dev/net1",
            network_interface,
            network_interface_driver,
        ),
        network_socket_line: render_network_socket_runtime_line(
            "/run/net1.sock",
            network_socket,
            network_socket_driver,
        ),
        storage_line: render_storage_runtime_line("/dev/storage0", storage, storage_driver),
        final_marker: "device-runtime-smoke-ok",
    })
}

pub fn dispatch_boot_proof<D: BootProofDispatcher>(
    dispatcher: &mut D,
    context: &D::Context,
    proof_kind: BootProofKind,
) -> D::Output {
    match proof_kind {
        BootProofKind::Vm => dispatcher.run_vm(),
        BootProofKind::Wasm => dispatcher.run_wasm(),
        BootProofKind::Shell => dispatcher.run_shell(context),
        BootProofKind::Scheduler => dispatcher.run_scheduler(),
        BootProofKind::ProcessExec => dispatcher.run_process_exec(),
        BootProofKind::Vfs => dispatcher.run_vfs(),
        BootProofKind::Render3d => dispatcher.run_render3d(),
        BootProofKind::CompatGfx => dispatcher.run_compat_gfx(),
        BootProofKind::CompatAudio => dispatcher.run_compat_audio(),
        BootProofKind::CompatInput => dispatcher.run_compat_input(),
        BootProofKind::CompatLoader => dispatcher.run_compat_loader(),
        BootProofKind::CompatAbi => dispatcher.run_compat_abi(),
        BootProofKind::CompatForeign => dispatcher.run_compat_foreign(),
        BootProofKind::Bus => dispatcher.run_bus(),
        BootProofKind::Network => dispatcher.run_network(),
        BootProofKind::NetworkHardware => dispatcher.run_network_hardware(),
        BootProofKind::NetworkHardwareInterface => dispatcher.run_network_hardware_interface(),
        BootProofKind::NetworkHardwareRx => dispatcher.run_network_hardware_rx(),
        BootProofKind::NetworkHardwareUdpRx => dispatcher.run_network_hardware_udp_rx(),
        BootProofKind::NetworkHardwareTx => dispatcher.run_network_hardware_tx(),
        BootProofKind::NetworkHardwareUdpTx => dispatcher.run_network_hardware_udp_tx(),
        BootProofKind::DeviceRuntime => dispatcher.run_device_runtime(),
        BootProofKind::StorageCommit => dispatcher.run_storage_commit(),
        BootProofKind::StorageRecover => dispatcher.run_storage_recover(),
        BootProofKind::StorageCorrupt => dispatcher.run_storage_corrupt(),
    }
}

pub fn dispatch_surface_smoke<D: SurfaceSmokeDispatcher>(
    dispatcher: &mut D,
    smoke_kind: SurfaceSmokeKind,
) -> D::Output {
    match smoke_kind {
        SurfaceSmokeKind::Smoke => dispatcher.run_smoke(),
        SurfaceSmokeKind::Shell => dispatcher.run_shell(),
        SurfaceSmokeKind::DeviceRuntime => dispatcher.run_device_runtime(),
        SurfaceSmokeKind::Bus => dispatcher.run_bus(),
        SurfaceSmokeKind::Vfs => dispatcher.run_vfs(),
        SurfaceSmokeKind::Wasm => dispatcher.run_wasm(),
        SurfaceSmokeKind::CompatGfx => dispatcher.run_compat_gfx(),
        SurfaceSmokeKind::CompatAudio => dispatcher.run_compat_audio(),
        SurfaceSmokeKind::CompatInput => dispatcher.run_compat_input(),
        SurfaceSmokeKind::CompatLoader => dispatcher.run_compat_loader(),
        SurfaceSmokeKind::CompatAbi => dispatcher.run_compat_abi(),
        SurfaceSmokeKind::Network => dispatcher.run_network(),
    }
}

pub fn parse_boot_proof_env(value: Option<&str>) -> Option<BootProofKind> {
    BootProofKind::parse_env(value)
}

pub fn bytes_contain_all_markers(bytes: &[u8], markers: &[&str]) -> bool {
    markers.iter().all(|marker| {
        let marker = marker.as_bytes();
        bytes
            .windows(marker.len())
            .any(|candidate| candidate == marker)
    })
}

pub fn parse_exit_code(token: Option<&str>) -> ExitCode {
    token
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(0)
}

pub fn path_contains_all_markers<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    markers: &[&str],
) -> Result<bool, ExitCode> {
    const MAX_MARKERS: usize = 8;
    if markers.len() > MAX_MARKERS {
        return Err(241);
    }
    let mut marker_bytes = [&[][..]; MAX_MARKERS];
    let mut max_marker_len = 0usize;
    for (index, marker) in markers.iter().enumerate() {
        let bytes = marker.as_bytes();
        marker_bytes[index] = bytes;
        max_marker_len = max_marker_len.max(bytes.len());
    }
    if max_marker_len > 128 {
        return Err(241);
    }
    let tail_len = max_marker_len.saturating_sub(1);
    let mut seen = [false; MAX_MARKERS];
    let mut tail = [0u8; 128];
    let mut tail_count = 0usize;
    let mut buffer = [0u8; 256];
    let mut window = [0u8; 384];
    let is_procfs = path.starts_with("/proc/");
    if is_procfs {
        const INITIAL_PROCFS_CAPACITY: usize = 4096;
        const MAX_PROCFS_SNAPSHOT_BYTES: usize = 4 * 1024 * 1024;
        let mut capacity = INITIAL_PROCFS_CAPACITY;
        while capacity <= MAX_PROCFS_SNAPSHOT_BYTES {
            let mut snapshot = alloc::vec::Vec::with_capacity(capacity);
            snapshot.resize(capacity, 0);
            let count = runtime.read_procfs(path, &mut snapshot).map_err(|_| 238)?;
            let haystack = &snapshot[..count];
            for (index, marker) in marker_bytes[..markers.len()].iter().enumerate() {
                if !seen[index]
                    && haystack
                        .windows(marker.len())
                        .any(|candidate| candidate == *marker)
                {
                    seen[index] = true;
                }
            }
            if seen[..markers.len()].iter().all(|matched| *matched) {
                return Ok(true);
            }
            if count < capacity {
                return Ok(false);
            }
            if capacity > (MAX_PROCFS_SNAPSHOT_BYTES / 2) {
                break;
            }
            capacity *= 2;
        }
        return Ok(false);
    }
    let fd = runtime.open_path(path).map_err(|_| 237)?;
    loop {
        let count = runtime.read(fd, &mut buffer).map_err(|_| 238)?;
        if count == 0 {
            break;
        }
        let window_len = tail_count + count;
        window[..tail_count].copy_from_slice(&tail[..tail_count]);
        window[tail_count..window_len].copy_from_slice(&buffer[..count]);
        let haystack = &window[..window_len];
        for (index, marker) in marker_bytes[..markers.len()].iter().enumerate() {
            if !seen[index]
                && haystack
                    .windows(marker.len())
                    .any(|candidate| candidate == *marker)
            {
                seen[index] = true;
            }
        }
        if seen[..markers.len()].iter().all(|matched| *matched) {
            break;
        }
        if tail_len != 0 {
            let keep = window_len.min(tail_len);
            tail[..keep].copy_from_slice(&window[window_len - keep..window_len]);
            tail_count = keep;
        }
    }
    runtime.close(fd).map_err(|_| 240)?;
    Ok(seen[..markers.len()].iter().all(|matched| *matched))
}

#[cfg(test)]
mod tests {
    use super::{
        BootProofDispatcher, BootProofKind, DeviceRuntimeSmokeReport, SurfaceSmokeDispatcher,
        SurfaceSmokeKind, bytes_contain_all_markers, device_class_name, dispatch_boot_proof,
        dispatch_surface_smoke, parse_boot_proof_env, parse_exit_code, render_device_runtime_line,
        render_storage_runtime_line,
    };
    use alloc::string::String;
    use ngos_user_abi::{NativeDeviceRecord, NativeDriverRecord, NativeStorageVolumeRecord};

    #[test]
    fn boot_proof_kind_parses_vm() {
        assert_eq!(
            BootProofKind::parse_env(Some("vm")),
            Some(BootProofKind::Vm)
        );
    }

    #[test]
    fn boot_proof_kind_emits_vm_marker() {
        assert_eq!(BootProofKind::Vm.marker(), "boot.proof=vm");
    }

    #[test]
    fn boot_proof_kind_parses_network_hardware() {
        assert_eq!(
            BootProofKind::parse_env(Some("network-hardware")),
            Some(BootProofKind::NetworkHardware)
        );
    }

    #[test]
    fn boot_proof_kind_emits_network_hardware_marker() {
        assert_eq!(
            BootProofKind::NetworkHardware.marker(),
            "boot.proof=network-hardware"
        );
    }

    #[test]
    fn boot_proof_kind_parses_network_hardware_interface() {
        assert_eq!(
            BootProofKind::parse_env(Some("network-hardware-interface")),
            Some(BootProofKind::NetworkHardwareInterface)
        );
    }

    #[test]
    fn boot_proof_kind_emits_network_hardware_interface_marker() {
        assert_eq!(
            BootProofKind::NetworkHardwareInterface.marker(),
            "boot.proof=network-hardware-interface"
        );
    }

    #[test]
    fn boot_proof_kind_parses_network_hardware_udp_tx() {
        assert_eq!(
            BootProofKind::parse_env(Some("network-hardware-udp-tx")),
            Some(BootProofKind::NetworkHardwareUdpTx)
        );
    }

    #[test]
    fn boot_proof_kind_emits_network_hardware_udp_tx_marker() {
        assert_eq!(
            BootProofKind::NetworkHardwareUdpTx.marker(),
            "boot.proof=network-hardware-udp-tx"
        );
    }

    #[test]
    fn boot_proof_kind_parses_network_hardware_rx() {
        assert_eq!(
            BootProofKind::parse_env(Some("network-hardware-rx")),
            Some(BootProofKind::NetworkHardwareRx)
        );
    }

    #[test]
    fn boot_proof_kind_emits_network_hardware_rx_marker() {
        assert_eq!(
            BootProofKind::NetworkHardwareRx.marker(),
            "boot.proof=network-hardware-rx"
        );
    }

    #[test]
    fn boot_proof_kind_parses_network_hardware_udp_rx() {
        assert_eq!(
            BootProofKind::parse_env(Some("network-hardware-udp-rx")),
            Some(BootProofKind::NetworkHardwareUdpRx)
        );
    }

    #[test]
    fn boot_proof_kind_emits_network_hardware_udp_rx_marker() {
        assert_eq!(
            BootProofKind::NetworkHardwareUdpRx.marker(),
            "boot.proof=network-hardware-udp-rx"
        );
    }

    #[test]
    fn boot_proof_kind_parses_network_hardware_tx() {
        assert_eq!(
            BootProofKind::parse_env(Some("network-hardware-tx")),
            Some(BootProofKind::NetworkHardwareTx)
        );
    }

    #[test]
    fn boot_proof_kind_emits_network_hardware_tx_marker() {
        assert_eq!(
            BootProofKind::NetworkHardwareTx.marker(),
            "boot.proof=network-hardware-tx"
        );
    }

    #[test]
    fn surface_smoke_kind_parses_network_smoke_command() {
        assert_eq!(
            SurfaceSmokeKind::parse_command("network-smoke"),
            Some(SurfaceSmokeKind::Network)
        );
    }

    #[test]
    fn surface_smoke_kind_emits_shell_marker() {
        assert_eq!(SurfaceSmokeKind::Shell.marker(), "shell-smoke-ok");
    }

    #[test]
    fn parse_boot_proof_env_delegates_to_kind_parser() {
        assert_eq!(
            parse_boot_proof_env(Some("compat-abi")),
            Some(BootProofKind::CompatAbi)
        );
    }

    #[test]
    fn bytes_contain_all_markers_checks_all_markers() {
        assert!(bytes_contain_all_markers(
            b"alpha beta gamma",
            &["alpha", "gamma"]
        ));
        assert!(!bytes_contain_all_markers(
            b"alpha beta gamma",
            &["alpha", "delta"]
        ));
    }

    #[test]
    fn parse_exit_code_defaults_to_zero_for_invalid_input() {
        assert_eq!(parse_exit_code(Some("17")), 17);
        assert_eq!(parse_exit_code(Some("bad")), 0);
        assert_eq!(parse_exit_code(None), 0);
    }

    #[derive(Default)]
    struct Recorder;

    impl BootProofDispatcher for Recorder {
        type Context = ();
        type Output = &'static str;

        fn run_vm(&mut self) -> Self::Output {
            "vm"
        }
        fn run_wasm(&mut self) -> Self::Output {
            "wasm"
        }
        fn run_shell(&mut self, _context: &Self::Context) -> Self::Output {
            "shell"
        }
        fn run_scheduler(&mut self) -> Self::Output {
            "scheduler"
        }
        fn run_process_exec(&mut self) -> Self::Output {
            "process-exec"
        }
        fn run_vfs(&mut self) -> Self::Output {
            "vfs"
        }
        fn run_render3d(&mut self) -> Self::Output {
            "render3d"
        }
        fn run_compat_gfx(&mut self) -> Self::Output {
            "compat-gfx"
        }
        fn run_compat_audio(&mut self) -> Self::Output {
            "compat-audio"
        }
        fn run_compat_input(&mut self) -> Self::Output {
            "compat-input"
        }
        fn run_compat_loader(&mut self) -> Self::Output {
            "compat-loader"
        }
        fn run_compat_abi(&mut self) -> Self::Output {
            "compat-abi"
        }
        fn run_compat_foreign(&mut self) -> Self::Output {
            "compat-foreign"
        }
        fn run_bus(&mut self) -> Self::Output {
            "bus"
        }
        fn run_network(&mut self) -> Self::Output {
            "network"
        }
        fn run_network_hardware(&mut self) -> Self::Output {
            "network-hardware"
        }
        fn run_network_hardware_interface(&mut self) -> Self::Output {
            "network-hardware-interface"
        }
        fn run_network_hardware_rx(&mut self) -> Self::Output {
            "network-hardware-rx"
        }
        fn run_network_hardware_udp_rx(&mut self) -> Self::Output {
            "network-hardware-udp-rx"
        }
        fn run_network_hardware_tx(&mut self) -> Self::Output {
            "network-hardware-tx"
        }
        fn run_network_hardware_udp_tx(&mut self) -> Self::Output {
            "network-hardware-udp-tx"
        }
        fn run_device_runtime(&mut self) -> Self::Output {
            "device-runtime"
        }
        fn run_storage_commit(&mut self) -> Self::Output {
            "storage-commit"
        }
        fn run_storage_recover(&mut self) -> Self::Output {
            "storage-recover"
        }
        fn run_storage_corrupt(&mut self) -> Self::Output {
            "storage-corrupt"
        }
    }

    impl SurfaceSmokeDispatcher for Recorder {
        type Output = &'static str;

        fn run_smoke(&mut self) -> Self::Output {
            "smoke"
        }
        fn run_shell(&mut self) -> Self::Output {
            "shell-smoke"
        }
        fn run_device_runtime(&mut self) -> Self::Output {
            "device-runtime-smoke"
        }
        fn run_bus(&mut self) -> Self::Output {
            "bus-smoke"
        }
        fn run_vfs(&mut self) -> Self::Output {
            "vfs-smoke"
        }
        fn run_wasm(&mut self) -> Self::Output {
            "wasm-smoke"
        }
        fn run_compat_gfx(&mut self) -> Self::Output {
            "compat-gfx-smoke"
        }
        fn run_compat_audio(&mut self) -> Self::Output {
            "compat-audio-smoke"
        }
        fn run_compat_input(&mut self) -> Self::Output {
            "compat-input-smoke"
        }
        fn run_compat_loader(&mut self) -> Self::Output {
            "compat-loader-smoke"
        }
        fn run_compat_abi(&mut self) -> Self::Output {
            "compat-abi-smoke"
        }
        fn run_network(&mut self) -> Self::Output {
            "network-smoke"
        }
    }

    #[test]
    fn dispatch_boot_proof_routes_to_network_hardware_udp_rx() {
        let mut recorder = Recorder::default();
        let result = dispatch_boot_proof(&mut recorder, &(), BootProofKind::NetworkHardwareUdpRx);
        assert_eq!(result, "network-hardware-udp-rx");
    }

    #[test]
    fn dispatch_surface_smoke_routes_to_network_smoke() {
        let mut recorder = Recorder;
        let result = dispatch_surface_smoke(&mut recorder, SurfaceSmokeKind::Network);
        assert_eq!(result, "network-smoke");
    }

    #[test]
    fn device_runtime_renderers_produce_expected_markers() {
        let device = NativeDeviceRecord {
            class: 3,
            state: 2,
            reserved0: 0,
            queue_depth: 0,
            queue_capacity: 8,
            submitted_requests: 7,
            completed_requests: 6,
            total_latency_ticks: 0,
            max_latency_ticks: 0,
            total_queue_wait_ticks: 0,
            max_queue_wait_ticks: 0,
            link_up: 1,
            reserved1: 0,
            block_size: 0,
            reserved2: 0,
            capacity_bytes: 0,
            last_completed_request_id: 0,
            last_completed_frame_tag: [0; 64],
            last_completed_source_api_name: [0; 24],
            last_completed_translation_label: [0; 32],
            last_terminal_request_id: 0,
            last_terminal_state: 0,
            reserved3: 0,
            last_terminal_frame_tag: [0; 64],
            last_terminal_source_api_name: [0; 24],
            last_terminal_translation_label: [0; 32],
        };
        let driver = NativeDriverRecord {
            state: 4,
            reserved: 0,
            bound_device_count: 1,
            queued_requests: 0,
            in_flight_requests: 1,
            completed_requests: 9,
            last_completed_request_id: 0,
            last_completed_frame_tag: [0; 64],
            last_completed_source_api_name: [0; 24],
            last_completed_translation_label: [0; 32],
            last_terminal_request_id: 0,
            last_terminal_state: 0,
            reserved1: 0,
            last_terminal_frame_tag: [0; 64],
            last_terminal_source_api_name: [0; 24],
            last_terminal_translation_label: [0; 32],
        };
        let storage = NativeStorageVolumeRecord {
            valid: 1,
            dirty: 0,
            payload_len: 0,
            generation: 42,
            parent_generation: 0,
            replay_generation: 0,
            payload_checksum: 0,
            superblock_sector: 0,
            journal_sector: 0,
            data_sector: 0,
            index_sector: 0,
            alloc_sector: 0,
            data_start_sector: 0,
            prepared_commit_count: 0,
            recovered_commit_count: 0,
            repaired_snapshot_count: 0,
            allocation_total_blocks: 0,
            allocation_used_blocks: 0,
            mapped_file_count: 3,
            mapped_extent_count: 0,
            mapped_directory_count: 2,
            mapped_symlink_count: 1,
            volume_id: [0; 32],
            state_label: [0; 32],
            last_commit_tag: [0; 32],
            payload_preview: [0; 32],
        };

        assert_eq!(device_class_name(3), "graphics");
        assert!(
            render_device_runtime_line("/dev/gpu0", device, driver)
                .contains("device.runtime.smoke.graphics")
        );
        assert!(
            render_storage_runtime_line("/dev/storage0", storage, driver)
                .contains("device.runtime.smoke.storage")
        );
    }

    #[test]
    fn device_runtime_report_struct_keeps_expected_marker() {
        let report = DeviceRuntimeSmokeReport {
            graphics_line: String::from("gfx"),
            audio_line: String::from("audio"),
            input_line: String::from("input"),
            network_interface_line: String::from("netif"),
            network_socket_line: String::from("netsock"),
            storage_line: String::from("storage"),
            final_marker: "device-runtime-smoke-ok",
        };

        assert_eq!(report.final_marker, "device-runtime-smoke-ok");
    }
}
