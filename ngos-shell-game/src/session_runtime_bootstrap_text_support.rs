use alloc::vec::Vec;

use ngos_game_compat_runtime::{GameCompatManifest, GameSessionPlan};

pub fn runtime_bootstrap_texts(
    plan: &GameSessionPlan,
    manifest: &GameCompatManifest,
    channel_path: &str,
) -> (
    alloc::string::String,
    alloc::string::String,
    alloc::string::String,
    alloc::string::String,
) {
    let env_text = crate::session_runtime_bootstrap_env_text_support::runtime_bootstrap_env_text(
        plan,
        channel_path,
    );
    let argv_text = core::iter::once(plan.executable_path.clone())
        .chain(plan.argv.iter().cloned())
        .collect::<Vec<_>>()
        .join("\n");
    let loader_text =
        crate::session_runtime_bootstrap_loader_text_support::runtime_bootstrap_loader_text(
            manifest,
        );
    let abi_text =
        crate::session_runtime_bootstrap_abi_text_support::runtime_bootstrap_abi_text(manifest);
    (env_text, argv_text, loader_text, abi_text)
}
