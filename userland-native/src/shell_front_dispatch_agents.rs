use super::*;

const SHELL_HELP_OVERVIEW: &str = "help session mode pwd env cd alias unalias";
const SHELL_HELP_COMMANDS: &str = "help help-ux help-topic whereami apropos suggest suggest-next explain-command command-card examples history history-tail history-find recent-work repeat-last rerun-find session mode pwd env cd alias unalias aliases set record-set record-get value-type value-load value-show string-trim string-upper string-lower string-split string-contains string-starts-with string-ends-with not is-empty record string int bool record-fields record-keys record-values record-has record-select record-drop record-rename record-merge record-set-field record-eq record-contains pairs-to-record filter-contains filter-not-contains filter-eq filter-field-eq filter-prefix filter-suffix list-count list-first list-last list-at list-find list-find-eq list-field list-any-contains list-all-contains list-sort list-reverse list-distinct list-append list-prepend list-drop list-take list-join into unset vars last-status true false repeat assert-status assert-file-contains source-file let print if match while for break continue calc fn call call-set return functions workspace-summary workspace-members workspace-topology workspace-audit crate-info crate-files crate-deps crate-audit crate-hotspots docs-list doc-show doc-search doc-links manifest-show rust-files rust-symbols crate-symbols find-symbol crate-find-symbol refs crate-refs outline crate-outline unsafe-audit todo-rust source-hotspots ps mounts jobs job-info fg kill pause resume renice affinity pending-signals blocked-signals spawn-path reap process-info compat-of identity-of status-of cmdline-of auxv-of environ-of root-of cwd-of exe-of vfsstats-of vfslocks-of vfswatches-of process-compat-status proc cat self status stat cmdline cwd environ exe auxv maps vmobjects vmdecisions vmepisodes fd fdinfo dup-fd close-fd seek-fd fcntl-getfl fcntl-getfd nonblock-fd cloexec-fd fd-watch fd-ready vm-map-anon vm-probe-map-anon vm-brk vm-probe-brk vm-quarantine vm-release vm-load-word vm-store-word vm-probe-store-word vm-sync-range vm-protect vm-unmap vm-advise vm-pressure vm-pressure-global caps queues system-queues stat-path lstat-path statfs-path open-path readlink-path cat-file head-file tail-file wc-file hex-file build-diagnostics diagnostic-files test-failures explain-test-failures diff-files patch-preview explain-diff impact-summary rollback-preview cat-numbered find-text find-tree-text replace-text replace-line insert-line delete-line append-line insert-before insert-after touch-file truncate-file move-path grep-tree copy-tree mirror-tree tree-path find-path edit-open edit-status edit-show edit-set edit-insert edit-append edit-delete edit-write edit-abort write-file append-file copy-file cmp-file grep-file mkdir-path mkfile-path mksock-path symlink-path rename-path unlink-path list-path game-manifest game-plan game-launch game-simulate game-sessions game-status game-stop game-next game-gfx-plan game-gfx-submit game-gfx-status game-gfx-driver-read game-gfx-request game-gfx-next game-audio-plan game-audio-submit game-audio-status game-audio-next game-input-plan game-input-submit game-input-status game-input-next game-watch-start game-watch-status game-watch-status-all game-watch-poll-all game-watch-wait game-watch-stop game-session-profile game-abi-status game-loader-status device gpu-evidence gpu-vbios gpu-gsp gpu-irq gpu-display gpu-power gpu-power-set gpu-media gpu-media-start gpu-neural gpu-neural-inject gpu-neural-commit gpu-tensor gpu-tensor-dispatch driver gpu-queue-capacity gpu-buffer-create gpu-buffer-write gpu-buffer gpu-scanout gpu-perf gpu-submit-buffer gpu-probe-submit-buffer gpu-request gpu-submit gpu-probe-submit gpu-present gpu-probe-present gpu-driver-read gpu-driver-bind gpu-probe-driver-bind gpu-driver-unbind gpu-probe-driver-unbind gpu-driver-reset gpu-probe-driver-reset gpu-driver-retire gpu-probe-driver-retire gpu-complete gpu-complete-request gpu-fail-request gpu-cancel-request gpu-probe-complete gpu-probe-complete-request gpu-probe-fail-request gpu-probe-cancel-request gpu-read gpu-watch gpu-unwatch gpu-lease-watch gpu-lease-unwatch gpu-lease-wait blk-read blk-write storage-volume storage-lineage storage-history storage-history-range storage-history-tail storage-history-entry storage storage-history-of storage-history-range-of storage-history-tail-of storage-history-entry-of storage-prepare storage-recover storage-repair storage-mount storage-unmount mount-info netif net-config net-admin net-link udp-bind udp-connect netsock net-send net-sendto net-recv net-recvfrom driver-read net-driver-read net-complete net-inject-udp queue-create net-watch net-unwatch mkbuspeer mkbusendpoint bus-peers bus-peer bus-endpoints bus-endpoint bus-attach bus-attach-rights bus-detach bus-send bus-recv bus-watch bus-unwatch resource-watch resource-unwatch queue-wait poll-path domains domain resources resource waiters contracts contract mkdomain mkresource mkcontract claim releaseclaim release transfer cancelclaim invoke contract-state resource-state resource-policy resource-governance resource-contract-policy resource-issuer-policy observe intent learn semantic-watch semantic-wait repair-system modernize-system repair-ai.diagnose repair-ai.repair repair-ai.memory repair-ai.save repair-ai.load nextmind.observe nextmind.optimize nextmind.auto nextmind.explain smoke shell-smoke device-runtime-smoke bus-smoke vfs-smoke wasm-smoke compat-gfx-smoke compat-audio-smoke compat-input-smoke compat-loader-smoke compat-abi-smoke network-smoke echo exit";

pub(crate) enum ShellFrontDispatchOutcome {
    Continue,
    Unhandled,
}

pub(crate) struct ShellFrontDispatchState<'a> {
    pub(crate) variables: &'a mut Vec<ShellVariable>,
    pub(crate) shell_functions: &'a mut Vec<ShellFunction>,
    pub(crate) shell_call_stack: &'a mut Vec<ShellCallFrame>,
    pub(crate) pending_lines: &'a mut Vec<String>,
    pub(crate) line_index: usize,
    pub(crate) last_status: &'a mut i32,
}

pub(crate) fn try_dispatch_shell_front_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    lang_candidate: &str,
    line: &str,
    state: ShellFrontDispatchState<'_>,
) -> Result<ShellFrontDispatchOutcome, ExitCode> {
    if lang_candidate.starts_with("fn ") || lang_candidate.starts_with("match ") {
        *state.last_status = 0;
        if let Some(result) = try_handle_shell_lang_command(
            runtime,
            lang_candidate,
            state.variables,
            state.shell_functions,
            state.shell_call_stack,
            state.pending_lines,
            state.line_index,
        ) {
            return map_front_dispatch_result(result);
        }
    }
    if line == "help" {
        write_line(runtime, SHELL_HELP_OVERVIEW).map_err(|_| 195)?;
        write_line(runtime, SHELL_HELP_COMMANDS).map_err(|_| 195)?;
        return Ok(ShellFrontDispatchOutcome::Continue);
    }
    if let Some(result) = try_handle_shell_lang_command(
        runtime,
        line,
        state.variables,
        state.shell_functions,
        state.shell_call_stack,
        state.pending_lines,
        state.line_index,
    ) {
        return map_front_dispatch_result(result);
    }
    Ok(ShellFrontDispatchOutcome::Unhandled)
}

fn map_front_dispatch_result(
    result: Result<(), ExitCode>,
) -> Result<ShellFrontDispatchOutcome, ExitCode> {
    match result {
        Ok(()) => Ok(ShellFrontDispatchOutcome::Continue),
        Err(code) if code == 2 => Err(199),
        Err(_) => Err(205),
    }
}
