use super::*;
use ngos_shell_proof::{
    SurfaceSmokeDispatcher, SurfaceSmokeKind, dispatch_surface_smoke, dispatch_wasm_run,
};
use ngos_user_runtime::wasm::{
    self, WasmCapability, WasmVerdict, execute_wasm_file,
};

struct NativeSurfaceSmokeDispatcher<'a, B: SyscallBackend> {
    runtime: &'a Runtime<B>,
    context: &'a SessionContext,
}

impl<B: SyscallBackend> SurfaceSmokeDispatcher for NativeSurfaceSmokeDispatcher<'_, B> {
    type Output = Result<(), ExitCode>;

    fn run_smoke(&mut self) -> Self::Output {
        let code = run_native_surface_smoke(self.runtime, false);
        if code == 0 { Ok(()) } else { Err(code) }
    }

    fn run_shell(&mut self) -> Self::Output {
        let code = run_session_shell_script(self.runtime, self.context, SHELL_BOOT_SMOKE_SCRIPT);
        if code != 0 {
            return Err(code);
        }
        let code = run_session_shell_script(
            self.runtime,
            self.context,
            SHELL_BOOT_SMOKE_SCRIPT_POST_PIPELINE,
        );
        if code != 0 {
            return Err(code);
        }
        let code =
            run_session_shell_script(self.runtime, self.context, SHELL_BOOT_SMOKE_SCRIPT_JOBS);
        if code != 0 {
            return Err(code);
        }
        let code =
            run_session_shell_script(self.runtime, self.context, SHELL_BOOT_SMOKE_SCRIPT_TAIL);
        if code == 0 { Ok(()) } else { Err(code) }
    }

    fn run_device_runtime(&mut self) -> Self::Output {
        let code = run_native_device_runtime_boot_smoke(self.runtime);
        if code == 0 { Ok(()) } else { Err(code) }
    }

    fn run_bus(&mut self) -> Self::Output {
        let code = run_native_bus_boot_smoke(self.runtime);
        if code == 0 { Ok(()) } else { Err(code) }
    }

    fn run_vfs(&mut self) -> Self::Output {
        let code = run_native_vfs_boot_smoke(self.runtime);
        if code == 0 { Ok(()) } else { Err(code) }
    }

    fn run_wasm(&mut self) -> Self::Output {
        let code = run_native_wasm_boot_smoke(self.runtime);
        if code == 0 { Ok(()) } else { Err(code) }
    }

    fn run_wasm_file(&mut self, path: &str) -> Self::Output {
        let pid = 1;
        let capabilities = vec![
            WasmCapability::ObserveProcessCapabilityCount,
            WasmCapability::ObserveSystemProcessCount,
            WasmCapability::ObserveProcessStatusBytes,
            WasmCapability::ObserveProcessCwdRoot,
        ];
        match execute_wasm_file(self.runtime, path, pid, &capabilities) {
            Ok(report) => {
                let _ = write_line(
                    self.runtime,
                    &format!(
                        "wasm.run path={} verdict={} capabilities={}",
                        path,
                        report.verdict.marker_name(),
                        report.granted_capabilities.len()
                    ),
                );
                if report.verdict == WasmVerdict::Ready {
                    Ok(())
                } else {
                    Err(250)
                }
            }
            Err(err) => {
                let _ = write_line(self.runtime, &format!("wasm.run error={err:?}"));
                Err(251)
            }
        }
    }

    fn run_compat_gfx(&mut self) -> Self::Output {
        let code = run_native_compat_graphics_boot_smoke(self.runtime);
        if code == 0 { Ok(()) } else { Err(code) }
    }

    fn run_compat_audio(&mut self) -> Self::Output {
        let code = run_native_compat_audio_boot_smoke(self.runtime);
        if code == 0 { Ok(()) } else { Err(code) }
    }

    fn run_compat_input(&mut self) -> Self::Output {
        let code = run_native_compat_input_boot_smoke(self.runtime);
        if code == 0 { Ok(()) } else { Err(code) }
    }

    fn run_compat_loader(&mut self) -> Self::Output {
        let code = compat_loader_smoke_agents::run_native_compat_loader_boot_smoke(self.runtime);
        if code == 0 { Ok(()) } else { Err(code) }
    }

    fn run_compat_abi(&mut self) -> Self::Output {
        let code = compat_foreign_smoke_agents::run_native_compat_abi_boot_smoke(self.runtime);
        if code == 0 { Ok(()) } else { Err(code) }
    }

    fn run_network(&mut self) -> Self::Output {
        let code = run_native_network_boot_smoke(self.runtime);
        if code == 0 { Ok(()) } else { Err(code) }
    }
}

pub(super) fn try_handle_surface_smoke_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    context: &SessionContext,
    line: &str,
) -> Option<Result<(), ExitCode>> {
    if let Some(rest) = line.strip_prefix("wasm-run ") {
        let path = rest.trim();
        if path.is_empty() {
            let _ = write_line(runtime, "usage: wasm-run <path>");
            return Some(Err(2));
        }
        let mut dispatcher = NativeSurfaceSmokeDispatcher { runtime, context };
        return Some(dispatch_wasm_run(&mut dispatcher, path));
    }
    let smoke_kind = SurfaceSmokeKind::parse_command(line)?;
    let mut dispatcher = NativeSurfaceSmokeDispatcher { runtime, context };
    let result = dispatch_surface_smoke(&mut dispatcher, smoke_kind);
    Some(match result {
        Ok(()) => write_line(runtime, smoke_kind.marker()).map_err(|_| 198),
        Err(code) => Err(code),
    })
}
