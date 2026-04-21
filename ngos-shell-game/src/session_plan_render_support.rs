use alloc::format;

use ngos_game_compat_runtime::{GameSessionPlan, compat_target_name, lane_name};
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::write_line;

pub fn game_render_plan<B: SyscallBackend>(
    runtime: &Runtime<B>,
    plan: &GameSessionPlan,
) -> Result<(), ExitCode> {
    write_line(
        runtime,
        &format!(
            "game.plan domain={} target={} process={} cwd={} exec={}",
            plan.domain_name,
            compat_target_name(plan.target),
            plan.process_name,
            plan.working_dir,
            plan.executable_path
        ),
    )?;
    for lane in &plan.lanes {
        write_line(
            runtime,
            &format!(
                "game.plan.lane kind={} resource={} contract={}",
                lane_name(lane.kind),
                lane.resource_name,
                lane.contract_label
            ),
        )?;
    }
    write_line(
        runtime,
        &format!(
            "game.plan.loader route={} mode={} entry={} bootstrap={} entrypoint={} requires-shims={}",
            plan.loader_routing.route_class,
            plan.loader_routing.launch_mode,
            plan.loader_routing.entry_profile,
            plan.loader_routing.bootstrap_profile,
            plan.loader_routing.entrypoint,
            if plan.loader_routing.requires_compat_shims {
                1
            } else {
                0
            },
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "game.plan.abi route={} handles={} paths={} scheduler={} sync={} timer={} module={} event={} requires-shims={}",
            plan.abi_routing.route_class,
            plan.abi_routing.handle_profile,
            plan.abi_routing.path_profile,
            plan.abi_routing.scheduler_profile,
            plan.abi_routing.sync_profile,
            plan.abi_routing.timer_profile,
            plan.abi_routing.module_profile,
            plan.abi_routing.event_profile,
            if plan.abi_routing.requires_kernel_abi_shims {
                1
            } else {
                0
            },
        ),
    )?;
    for env in &plan.env_shims {
        write_line(runtime, &format!("game.plan.env {}={}", env.key, env.value))?;
    }
    Ok(())
}
