use alloc::format;

use ngos_game_compat_runtime::{GameCompatManifest, compat_target_name};

use crate::PROGRAM_NAME;

pub fn runtime_bootstrap_abi_text(manifest: &GameCompatManifest) -> alloc::string::String {
    let abi_routing = manifest.abi_routing_plan();
    [
        format!("route-class={}", abi_routing.route_class),
        format!("handle-profile={}", abi_routing.handle_profile),
        format!("path-profile={}", abi_routing.path_profile),
        format!("scheduler-profile={}", abi_routing.scheduler_profile),
        format!("sync-profile={}", abi_routing.sync_profile),
        format!("timer-profile={}", abi_routing.timer_profile),
        format!("module-profile={}", abi_routing.module_profile),
        format!("event-profile={}", abi_routing.event_profile),
        format!(
            "requires-kernel-abi-shims={}",
            if abi_routing.requires_kernel_abi_shims {
                "1"
            } else {
                "0"
            }
        ),
        format!("target={}", compat_target_name(manifest.target)),
        format!("prefix={}", manifest.shims.prefix),
        format!("exec={}", manifest.executable_path),
        format!("cwd={}", manifest.working_dir),
        format!("producer={PROGRAM_NAME}"),
    ]
    .join("\n")
}
