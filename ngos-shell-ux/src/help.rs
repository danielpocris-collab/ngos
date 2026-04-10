//! Shell help/documentation render functions: help-ux, help-topic, command-card, examples, explain-command.

use alloc::format;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::catalog::{proof_command_summary, shell_guess_ux_topic};

fn write_line<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map_err(|_| 196)?;
    Ok(())
}

pub fn shell_render_help_ux<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    write_line(
        runtime,
        "help-ux topics=help,help-topic,whereami,apropos,suggest,suggest-next,history-find,history-tail,recent-work,repeat-last,rerun-find,examples,explain-command,command-card,unknown-command-feedback",
    )?;
    write_line(
        runtime,
        "help-ux tips=use 'suggest <prefix>' for discovery, 'apropos <needle>' for related tools, 'history-find <needle>' to recover commands, 'recent-work' to isolate actual work, and 'repeat-last' or 'rerun-find <needle>' to replay work quickly",
    )
}

pub fn shell_render_help_topic<B: SyscallBackend>(
    runtime: &Runtime<B>,
    topic: &str,
) -> Result<(), ExitCode> {
    let topic = topic.trim();
    if topic.is_empty() {
        return write_line(runtime, "usage: help-topic <topic>");
    }
    let normalized = topic.to_lowercase();
    match normalized.as_str() {
        "pipeline" => {
            write_line(
                runtime,
                "help-topic topic=pipeline summary=compose semantic values with '|>' across records lists and real subsystem surfaces",
            )?;
            write_line(
                runtime,
                "help-topic pipeline-steps=record-get,list-count,filter-field-eq,list-field,record-select,record-merge",
            )?;
            write_line(
                runtime,
                "help-topic pipeline-sources=session,process-info,mounts,storage,storage-volume,storage-history-of,storage-history-range-of,storage-history-tail-of,storage-history-entry-of,fd,maps,queues,netif,netsock,waiters",
            )
        }
        "history" => {
            write_line(
                runtime,
                "help-topic topic=history summary=recover, inspect, and replay prior shell commands",
            )?;
            write_line(
                runtime,
                "help-topic history-tools=history,history-tail,history-find,repeat-last,rerun-find,last-status",
            )
        }
        "session" => {
            write_line(
                runtime,
                "help-topic topic=session summary=orient quickly inside the current shell context and local mutable state",
            )?;
            write_line(
                runtime,
                "help-topic session-tools=session,whereami,pwd,env,aliases,vars,jobs,last-status",
            )
        }
        "recovery" => {
            write_line(
                runtime,
                "help-topic topic=recovery summary=re-enter work quickly after interruption or command mistakes",
            )?;
            write_line(
                runtime,
                "help-topic recovery-tools=recent-work,history-tail,history-find,repeat-last,rerun-find,suggest-next",
            )
        }
        "process" => {
            write_line(
                runtime,
                "help-topic topic=process summary=inspect process identity, procfs views, maps, descriptors, and signals",
            )?;
            write_line(
                runtime,
                "help-topic process-tools=ps,process-info,identity-of,status-of,cmdline-of,auxv-of,maps,fd,fdinfo,pending-signals,blocked-signals,caps",
            )
        }
        "network" => {
            write_line(
                runtime,
                "help-topic topic=network summary=inspect interfaces and sockets as semantic records instead of raw text",
            )?;
            write_line(
                runtime,
                "help-topic network-tools=netif,netsock,udp-bind,udp-connect,net-send,net-recv,network-smoke",
            )
        }
        "review" => {
            write_line(
                runtime,
                "help-topic topic=review summary=analyze code diffs, diagnostics, and test failures semantically",
            )?;
            write_line(
                runtime,
                "help-topic review-tools=build-diagnostics,diagnostic-files,test-failures,explain-test-failures,diff-files,patch-preview,explain-diff,impact-summary,rollback-preview",
            )
        }
        "vfs" => {
            write_line(
                runtime,
                "help-topic topic=vfs summary=inspect namespace, mounts, metadata, and procfs-backed views semantically",
            )?;
            write_line(
                runtime,
                "help-topic vfs-tools=mounts,mount-info,stat-path,lstat-path,statfs-path,readlink-path,fd,fdinfo,vfsstats-of,vfslocks-of,vfswatches-of,vfs-smoke",
            )
        }
        "storage" => {
            write_line(
                runtime,
                "help-topic topic=storage summary=inspect persistent storage lineage, history, and mounted semantic state",
            )?;
            write_line(
                runtime,
                "help-topic storage-tools=storage-volume,storage,storage-lineage,storage-history,storage-history-range,storage-history-tail,storage-history-entry,storage-history-of,storage-history-range-of,storage-history-tail-of,storage-history-entry-of,storage-prepare,storage-recover,storage-repair,storage-mount,storage-unmount",
            )
        }
        _ => write_line(
            runtime,
            &format!(
                "help-topic topic={} summary=unknown topic; try pipeline history session process storage network review vfs",
                topic
            ),
        ),
    }
}

pub fn shell_render_command_explain<B: SyscallBackend>(
    runtime: &Runtime<B>,
    command: &str,
) -> Result<(), ExitCode> {
    let command = command.trim();
    if command.is_empty() {
        return write_line(runtime, "usage: explain-command <name>");
    }
    let normalized = command.to_lowercase();
    if let Some(summary) = proof_command_summary(normalized.as_str()) {
        return write_line(runtime, &format!("command={} summary={}", command, summary));
    }
    let summary = match normalized.as_str() {
        "help-topic" => "explains one shell topic such as pipeline, process, network, or review",
        "whereami" => "summarizes the current shell session context, cwd, image, and local state",
        "command-card" => "shows a compact summary, topic, and examples for one command",
        "history-find" => "searches session history for matching commands",
        "history-tail" => "shows the last N commands from session history",
        "recent-work" => {
            "shows recent non-meta commands so work is separated from shell navigation noise"
        }
        "repeat-last" => "replays the latest non-repeat command back into the session",
        "rerun-find" => "replays the latest history command containing a search needle",
        "suggest-next" => {
            "suggests a small next-step flow for a topic like session, process, or review"
        }
        "suggest" => "suggests commands or aliases by typed prefix",
        "apropos" => "finds commands related to a keyword",
        "explain-command" => "explains a shell command in one line",
        "examples" => "shows short concrete examples for a shell command or topic",
        "identity-of" => "emits semantic process identity fields like uid gid umask root",
        "value-load" => "replays a stored semantic shell variable back into the active pipeline",
        "compat-of" => "emits semantic compat and loader routing fields for a process",
        "auxv-of" => "reads process auxv as a semantic list of key=value entries",
        "mounts" => "emits semantic inventory of active mounts",
        "storage-volume" => {
            "renders the current persistent storage volume state as a semantic record"
        }
        "storage" => "emits semantic persistent storage record for one device",
        "storage-lineage" => {
            "renders persistent storage generation, parent, and continuity summary"
        }
        "storage-history" => "renders persistent storage lineage history as direct shell text",
        "storage-history-of" => "emits semantic persistent storage lineage history for one device",
        "storage-history-range-of" => {
            "emits a semantic persistent storage lineage window for one device and range"
        }
        "storage-history-tail-of" => {
            "emits a semantic persistent storage lineage tail for one device"
        }
        "storage-history-entry-of" => {
            "emits one semantic persistent storage lineage entry for one device and index"
        }
        "storage-history-entry" => {
            "renders one persistent storage lineage entry as direct shell text"
        }
        "storage-history-range" => {
            "renders a bounded persistent storage lineage window as direct shell text"
        }
        "storage-history-tail" => {
            "renders the tail of persistent storage lineage history as direct shell text"
        }
        "storage-prepare" => "creates a persistent storage commit and returns the new generation",
        "storage-recover" => "recovers a persistent storage volume and returns the new generation",
        "storage-repair" => "repairs a persistent storage snapshot and returns the new generation",
        "repair-system" => {
            "repairs scheduler, network, and memory pressure around the verified core"
        }
        "modernize-system" => {
            "applies proactive runtime modernization for memory and network profiles"
        }
        "repair-ai.diagnose" => "diagnoses system pressure and picks a learned repair strategy",
        "repair-ai.repair" => "runs learned semantic repair planning with memory and rollback",
        "repair-ai.memory" => "shows learned repair episodes for the active shell session",
        "repair-ai.save" => "persists learned repair episodes to a VFS path",
        "repair-ai.load" => "loads learned repair episodes from a VFS path",
        "storage-mount" => "mounts a storage volume onto a shell path",
        "storage-unmount" => "unmounts a storage volume from a shell path",
        "mount-info" => "emits semantic record for one mount path",
        "process-info" => "emits semantic process record for one pid",
        "queues" => "emits semantic inventory of event queues for the active process",
        "system-queues" => "renders the global procfs queue view",
        "find-symbol" => "finds symbol definitions in Rust sources",
        "build-diagnostics" => "parses build logs into semantic diagnostics",
        "test-failures" => "parses failed tests into semantic failure records",
        "impact-summary" => "summarizes semantic impact between two files",
        "rollback-preview" => "shows inverse patch preview between two files",
        _ => "command exists in the shell surface; use help, apropos, or suggest for nearby tools",
    };
    write_line(runtime, &format!("command={} summary={}", command, summary))
}

pub fn shell_render_examples<B: SyscallBackend>(
    runtime: &Runtime<B>,
    name: &str,
) -> Result<(), ExitCode> {
    let name = name.trim();
    if name.is_empty() {
        return write_line(runtime, "usage: examples <command-or-topic>");
    }
    let normalized = name.to_lowercase();
    let lines: &[&str] = match normalized.as_str() {
        "pipeline" => &[
            "example pipeline process-info 1 |> record-get pid PID |> value-show",
            "example pipeline mounts |> filter-field-eq mode rw |> list-count RW_MOUNTS |> value-show",
        ],
        "history" | "history-tail" => &[
            "example history history-tail 5",
            "example history history-find mount",
            "example history repeat-last",
            "example history rerun-find shell-proof",
        ],
        "session" | "whereami" => &[
            "example session whereami",
            "example session session |> record-get cwd CURRENT_CWD |> value-show",
        ],
        "identity-of" => &[
            "example identity identity-of 1 |> record-get uid UID |> value-show",
            "example identity identity-of 1 |> record-eq root / |> value-show",
        ],
        "mounts" => &[
            "example mounts mounts |> list-field device DEVICES |> value-show",
            "example mounts mounts |> filter-field-eq mode rw |> list-count RW_MOUNTS |> value-show",
        ],
        "storage-volume"
        | "storage"
        | "storage-lineage"
        | "storage-history"
        | "storage-history-range"
        | "storage-history-of"
        | "storage-history-range-of"
        | "storage-history-tail"
        | "storage-history-tail-of"
        | "storage-history-entry-of"
        | "storage-history-entry"
        | "storage-prepare"
        | "storage-recover"
        | "storage-repair"
        | "storage-mount"
        | "storage-unmount" => &[
            "example storage storage-volume /dev/storage0",
            "example storage storage /dev/storage0 |> record-get generation GEN |> value-show",
            "example storage storage-lineage /dev/storage0",
            "example storage storage-history /dev/storage0",
            "example storage storage-history-range /dev/storage0 0 3",
            "example storage storage-history-tail /dev/storage0 3",
            "example storage storage-prepare /dev/storage0 shell-smoke persistent-shell-proof",
            "example storage storage-recover /dev/storage0",
            "example storage storage-repair /dev/storage0",
            "example storage storage-mount /dev/storage0 /shell-proof-mount",
            "example storage storage-unmount /shell-proof-mount",
            "example storage storage-history-of /dev/storage0 |> list-count EVENTS |> value-show",
            "example storage storage-history-range-of /dev/storage0 0 3 |> list-count WINDOW |> value-show",
            "example storage storage-history-tail-of /dev/storage0 3 |> list-count WINDOW |> value-show",
            "example storage storage-history-entry-of /dev/storage0 0 |> record-get kind KIND |> value-show",
            "example storage storage-history-entry /dev/storage0 0",
        ],
        "netsock" => &[
            "example netsock netsock /run/net0.sock |> record-get local_port PORT |> value-show",
            "example netsock netsock /run/net0.sock |> record-get connected CONNECTED |> value-show",
        ],
        "review" | "impact-summary" => &[
            "example review impact-summary left.rs right.rs",
            "example review rollback-preview left.rs right.rs",
        ],
        "recovery" | "recent-work" => &[
            "example recovery recent-work 5",
            "example recovery rerun-find shell-proof",
            "example recovery suggest-next review",
        ],
        _ => &[
            "example generic suggest pro",
            "example generic help-topic pipeline",
        ],
    };
    for line in lines {
        write_line(runtime, line)?;
    }
    write_line(
        runtime,
        &format!("examples topic={} count={}", name, lines.len()),
    )
}

pub fn shell_render_command_card<B: SyscallBackend>(
    runtime: &Runtime<B>,
    command: &str,
) -> Result<(), ExitCode> {
    let command = command.trim();
    if command.is_empty() {
        return write_line(runtime, "usage: command-card <command>");
    }
    let normalized = command.to_lowercase();
    shell_render_command_explain(runtime, &normalized)?;
    write_line(
        runtime,
        &format!(
            "command-card command={} topic={}",
            normalized,
            shell_guess_ux_topic(&normalized)
        ),
    )?;
    shell_render_examples(runtime, &normalized)
}
