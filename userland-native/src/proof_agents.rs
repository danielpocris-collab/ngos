use super::*;

pub use ngos_shell_proof::BootProofKind;
use ngos_shell_proof::{BootProofDispatcher, dispatch_boot_proof, parse_boot_proof_env};

pub(crate) fn parse_boot_proof(bootstrap: &BootstrapArgs<'_>) -> Option<BootProofKind> {
    parse_boot_proof_env(bootstrap.env_value(BOOT_ENV_PROOF_PREFIX))
}

pub(crate) fn run_boot_proof<B: SyscallBackend>(
    runtime: &Runtime<B>,
    context: &ngos_user_abi::bootstrap::BootContext,
    proof_kind: BootProofKind,
) -> ExitCode {
    let _ = write_line(runtime, proof_kind.marker());
    let mut dispatcher = NativeBootProofDispatcher { runtime };
    dispatch_boot_proof(&mut dispatcher, context, proof_kind)
}

struct NativeBootProofDispatcher<'a, B: SyscallBackend> {
    runtime: &'a Runtime<B>,
}

impl<B: SyscallBackend> BootProofDispatcher for NativeBootProofDispatcher<'_, B> {
    type Context = ngos_user_abi::bootstrap::BootContext;
    type Output = ExitCode;

    fn run_vm(&mut self) -> Self::Output {
        run_native_vm_boot_smoke(self.runtime)
    }
    fn run_wasm(&mut self) -> Self::Output {
        run_native_wasm_boot_smoke(self.runtime)
    }
    fn run_shell(&mut self, context: &Self::Context) -> Self::Output {
        run_native_shell_boot_smoke(self.runtime, context)
    }
    fn run_scheduler(&mut self) -> Self::Output {
        run_native_scheduler_boot_smoke(self.runtime)
    }
    fn run_process_exec(&mut self) -> Self::Output {
        run_native_process_exec_boot_smoke(self.runtime)
    }
    fn run_vfs(&mut self) -> Self::Output {
        run_native_vfs_boot_smoke(self.runtime)
    }
    fn run_render3d(&mut self) -> Self::Output {
        run_native_render3d_smoke(self.runtime)
    }
    fn run_compat_gfx(&mut self) -> Self::Output {
        run_native_compat_graphics_boot_smoke(self.runtime)
    }
    fn run_compat_audio(&mut self) -> Self::Output {
        run_native_compat_audio_boot_smoke(self.runtime)
    }
    fn run_compat_input(&mut self) -> Self::Output {
        run_native_compat_input_boot_smoke(self.runtime)
    }
    fn run_compat_loader(&mut self) -> Self::Output {
        compat_loader_smoke_agents::run_native_compat_loader_boot_smoke(self.runtime)
    }
    fn run_compat_abi(&mut self) -> Self::Output {
        compat_foreign_smoke_agents::run_native_compat_abi_boot_smoke(self.runtime)
    }
    fn run_compat_foreign(&mut self) -> Self::Output {
        compat_foreign_smoke_agents::run_native_compat_foreign_boot_smoke(self.runtime)
    }
    fn run_bus(&mut self) -> Self::Output {
        run_native_bus_boot_smoke(self.runtime)
    }
    fn run_network(&mut self) -> Self::Output {
        run_native_network_boot_smoke(self.runtime)
    }
    fn run_network_hardware(&mut self) -> Self::Output {
        run_native_network_hardware_boot_smoke(self.runtime)
    }
    fn run_network_hardware_interface(&mut self) -> Self::Output {
        run_native_network_hardware_interface_boot_smoke(self.runtime)
    }
    fn run_network_hardware_rx(&mut self) -> Self::Output {
        run_native_network_hardware_rx_boot_smoke(self.runtime)
    }
    fn run_network_hardware_udp_rx(&mut self) -> Self::Output {
        run_native_network_hardware_udp_rx_boot_smoke(self.runtime)
    }
    fn run_network_hardware_tx(&mut self) -> Self::Output {
        run_native_network_hardware_tx_boot_smoke(self.runtime)
    }
    fn run_network_hardware_udp_tx(&mut self) -> Self::Output {
        run_native_network_hardware_udp_tx_boot_smoke(self.runtime)
    }
    fn run_device_runtime(&mut self) -> Self::Output {
        run_native_device_runtime_boot_smoke(self.runtime)
    }
    fn run_resource(&mut self) -> Self::Output {
        run_native_resource_boot_smoke(self.runtime)
    }
    fn run_storage_commit(&mut self) -> Self::Output {
        run_native_storage_commit_boot_smoke(self.runtime)
    }
    fn run_storage_recover(&mut self) -> Self::Output {
        run_native_storage_recover_boot_smoke(self.runtime)
    }
    fn run_storage_corrupt(&mut self) -> Self::Output {
        run_native_storage_corrupt_boot_smoke(self.runtime)
    }
}
