use alloc::string::String;

use ngos_game_compat_runtime::GameCompatManifest;

pub fn runtime_bootstrap_paths(
    manifest: &GameCompatManifest,
) -> (String, String, String, String, String) {
    let env_path = if manifest.shims.prefix == "/" {
        String::from("/session.env")
    } else {
        alloc::format!("{}/session.env", manifest.shims.prefix)
    };
    let argv_path = if manifest.shims.prefix == "/" {
        String::from("/session.argv")
    } else {
        alloc::format!("{}/session.argv", manifest.shims.prefix)
    };
    let channel_path = if manifest.shims.prefix == "/" {
        String::from("/session.chan")
    } else {
        alloc::format!("{}/session.chan", manifest.shims.prefix)
    };
    let loader_path = if manifest.shims.prefix == "/" {
        String::from("/session.loader")
    } else {
        alloc::format!("{}/session.loader", manifest.shims.prefix)
    };
    let abi_path = if manifest.shims.prefix == "/" {
        String::from("/session.abi")
    } else {
        alloc::format!("{}/session.abi", manifest.shims.prefix)
    };
    (env_path, argv_path, channel_path, loader_path, abi_path)
}
