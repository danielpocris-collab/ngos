use super::*;

pub(crate) fn image_path_matches_program(image_path: &str) -> bool {
    matches!(
        image_path,
        PROGRAM_NAME | LEGACY_PROGRAM_NAME | "/bin/ngos-userland-native" | "/bin/userland-native"
    ) || image_path
        .rsplit('/')
        .next()
        .is_some_and(|name| matches!(name, PROGRAM_NAME | LEGACY_PROGRAM_NAME))
}

pub(crate) use ngos_shell_proof::{
    bytes_contain_all_markers, parse_exit_code, path_contains_all_markers,
};

pub(crate) fn shell_render_system_queues<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> Result<(), ExitCode> {
    shell_render_procfs_path(runtime, "/proc/queues")
}
