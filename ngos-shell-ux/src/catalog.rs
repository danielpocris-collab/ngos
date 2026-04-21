//! UX catalog: command lists, proof summaries, topic guessing, meta-history detection.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use ngos_shell_types::ShellAlias;

pub const SHELL_UX_COMMANDS: &[&str] = &[
    "help",
    "help-ux",
    "help-topic",
    "whereami",
    "apropos",
    "suggest",
    "explain-command",
    "command-card",
    "examples",
    "history",
    "history-tail",
    "history-find",
    "recent-work",
    "repeat-last",
    "rerun-find",
    "suggest-next",
    "session",
    "pwd",
    "env",
    "cd",
    "alias",
    "unalias",
    "aliases",
    "set",
    "unset",
    "vars",
    "record-set",
    "record-get",
    "value-type",
    "value-load",
    "value-show",
    "source-file",
    "ps",
    "mounts",
    "storage",
    "storage-lineage",
    "storage-history",
    "storage-history-range",
    "storage-history-of",
    "storage-history-range-of",
    "storage-history-tail",
    "storage-history-tail-of",
    "storage-history-entry-of",
    "storage-history-entry",
    "storage-repair",
    "jobs",
    "job-info",
    "spawn-path",
    "fg",
    "kill",
    "process-info",
    "compat-of",
    "identity-of",
    "status-of",
    "cmdline-of",
    "auxv-of",
    "environ-of",
    "root-of",
    "cwd-of",
    "exe-of",
    "vfsstats-of",
    "vfslocks-of",
    "vfswatches-of",
    "fd",
    "fdinfo",
    "maps",
    "vmobjects",
    "vmdecisions",
    "vmepisodes",
    "caps",
    "queues",
    "system-queues",
    "mount-info",
    "netif",
    "netsock",
    "find-symbol",
    "refs",
    "outline",
    "build-diagnostics",
    "test-failures",
    "diff-files",
    "patch-preview",
    "impact-summary",
    "rollback-preview",
    "echo",
    "exit",
];

pub const UX_PROOF_COMMANDS: &[&str] = &[
    "shell-smoke",
    "vfs-smoke",
    "device-runtime-smoke",
    "bus-smoke",
    "network-smoke",
    "wasm-smoke",
];

pub fn proof_command_summary(command: &str) -> Option<&'static str> {
    match command {
        "shell-smoke" => Some("runs the shell proof front end-to-end"),
        "vfs-smoke" => Some("runs the VFS proof front end-to-end"),
        "device-runtime-smoke" => Some("runs the device runtime proof front end-to-end"),
        "bus-smoke" => Some("runs the bus proof front end-to-end"),
        "network-smoke" => Some("runs the networking proof front end-to-end"),
        "wasm-smoke" => Some("runs the WASM proof front end-to-end"),
        "compat-gfx-smoke" => Some("runs the compat graphics proof front end-to-end"),
        "compat-audio-smoke" => Some("runs the compat audio proof front end-to-end"),
        "compat-input-smoke" => Some("runs the compat input proof front end-to-end"),
        "compat-loader-smoke" => Some("runs the compat loader proof front end-to-end"),
        "compat-abi-smoke" => Some("runs the compat ABI proof front end-to-end"),
        _ => None,
    }
}

pub fn shell_is_meta_history_command(command: &str) -> bool {
    let command = command.trim();
    command == "help"
        || command == "help-ux"
        || command == "whereami"
        || command == "repeat-last"
        || command.starts_with("help-topic ")
        || command.starts_with("command-card ")
        || command.starts_with("examples ")
        || command.starts_with("history")
        || command.starts_with("suggest ")
        || command.starts_with("suggest-next")
        || command.starts_with("apropos ")
        || command.starts_with("explain-command ")
        || command.starts_with("rerun-find ")
}

pub fn shell_guess_ux_topic(command: &str) -> &'static str {
    if command.contains("storage")
        || command.contains("commit")
        || command.contains("recover")
        || command.contains("repair")
        || command.contains("snapshot")
    {
        "storage"
    } else if command.contains("mount")
        || command.contains("path")
        || command.contains("fd")
        || command.contains("vfs")
    {
        "vfs"
    } else if command.contains("net") || command.contains("udp") || command.contains("sock") {
        "network"
    } else if command.contains("process")
        || command.contains("signal")
        || command.contains("identity")
        || command.contains("compat")
        || command.contains("auxv")
        || command.contains("environ")
        || command.contains("root-of")
        || command.contains("cwd-of")
        || command.contains("exe-of")
        || command.contains("cmdline")
        || command.contains("maps")
        || command.contains("caps")
    {
        "process"
    } else if command.contains("history")
        || command.contains("repeat")
        || command.contains("rerun")
        || command.contains("suggest")
        || command.contains("apropos")
    {
        "history"
    } else if command.contains("diff")
        || command.contains("patch")
        || command.contains("diagnostic")
        || command.contains("test")
        || command.contains("review")
    {
        "review"
    } else if command.contains("session") || command.contains("where") || command.contains("pwd") {
        "session"
    } else {
        "pipeline"
    }
}

pub fn shell_ux_catalog(aliases: &[ShellAlias], history: &[String]) -> Vec<String> {
    let mut entries = SHELL_UX_COMMANDS
        .iter()
        .map(|entry| (*entry).to_string())
        .collect::<Vec<_>>();
    for proof_command in UX_PROOF_COMMANDS {
        if !entries.iter().any(|entry| entry == proof_command) {
            entries.push((*proof_command).to_string());
        }
    }
    for alias in aliases {
        if !entries.iter().any(|entry| entry == &alias.name) {
            entries.push(alias.name.clone());
        }
    }
    for command in history.iter().rev() {
        let name = command.split_whitespace().next().unwrap_or_default().trim();
        if name.is_empty() {
            continue;
        }
        if !entries.iter().any(|entry| entry == name) {
            entries.push(name.to_string());
        }
        if entries.len() >= 192 {
            break;
        }
    }
    entries
}
