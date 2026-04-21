use alloc::format;
use alloc::vec::Vec;

use ngos_game_compat_runtime::GameSessionPlan;

pub fn runtime_bootstrap_env_text(
    plan: &GameSessionPlan,
    channel_path: &str,
) -> alloc::string::String {
    plan.env_shims
        .iter()
        .map(|shim| format!("{}={}", shim.key, shim.value))
        .chain(core::iter::once(format!(
            "NGOS_GAME_CHANNEL={channel_path}"
        )))
        .collect::<Vec<_>>()
        .join("\n")
}
