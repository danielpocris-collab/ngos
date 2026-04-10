//! Local contracts for shell proc commands.

pub const JOB_INFO_USAGE: &str = "usage: job-info <pid>";
pub const FG_USAGE: &str = "usage: fg <pid>";
pub const KILL_USAGE: &str = "usage: kill <pid> <signal>";
pub const PENDING_SIGNALS_USAGE: &str = "usage: pending-signals <pid>";
pub const BLOCKED_SIGNALS_USAGE: &str = "usage: blocked-signals <pid>";
pub const SPAWN_PATH_USAGE: &str = "usage: spawn-path <name> <path>";
pub const REAP_USAGE: &str = "usage: reap <pid>";
pub const PROCESS_INFO_USAGE: &str = "usage: process-info <pid>";
pub const PROCESS_COMPAT_STATUS_USAGE: &str = "usage: process-compat-status <pid>";

pub fn is_self_procfs_section(section: &str) -> bool {
    matches!(
        section,
        "status"
            | "stat"
            | "cmdline"
            | "cwd"
            | "environ"
            | "exe"
            | "auxv"
            | "maps"
            | "vmobjects"
            | "vmdecisions"
            | "vmepisodes"
            | "fd"
            | "caps"
            | "queues"
    )
}

#[cfg(test)]
mod tests {
    use super::{
        is_self_procfs_section, BLOCKED_SIGNALS_USAGE, FG_USAGE, JOB_INFO_USAGE, KILL_USAGE,
        PENDING_SIGNALS_USAGE, PROCESS_COMPAT_STATUS_USAGE, PROCESS_INFO_USAGE, REAP_USAGE,
        SPAWN_PATH_USAGE,
    };

    #[test]
    fn self_procfs_sections_cover_vm_and_queue_views() {
        assert!(is_self_procfs_section("status"));
        assert!(is_self_procfs_section("vmobjects"));
        assert!(is_self_procfs_section("vmdecisions"));
        assert!(is_self_procfs_section("vmepisodes"));
        assert!(is_self_procfs_section("queues"));
        assert!(!is_self_procfs_section("mounts"));
        assert!(!is_self_procfs_section("unknown"));
    }

    #[test]
    fn proc_command_usage_contracts_are_stable() {
        assert_eq!(JOB_INFO_USAGE, "usage: job-info <pid>");
        assert_eq!(FG_USAGE, "usage: fg <pid>");
        assert_eq!(KILL_USAGE, "usage: kill <pid> <signal>");
        assert_eq!(PENDING_SIGNALS_USAGE, "usage: pending-signals <pid>");
        assert_eq!(BLOCKED_SIGNALS_USAGE, "usage: blocked-signals <pid>");
        assert_eq!(SPAWN_PATH_USAGE, "usage: spawn-path <name> <path>");
        assert_eq!(REAP_USAGE, "usage: reap <pid>");
        assert_eq!(PROCESS_INFO_USAGE, "usage: process-info <pid>");
        assert_eq!(
            PROCESS_COMPAT_STATUS_USAGE,
            "usage: process-compat-status <pid>"
        );
    }
}
