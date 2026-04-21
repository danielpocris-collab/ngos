use super::*;

pub(crate) const SHELL_BOOT_SMOKE_SCRIPT: &str = "\
help\n\
set SHELL_PROOF_MODE 1\n\
session\n\
assert-status 0 && echo shell.smoke.session protocol=kernel-launch cwd=/ outcome=ok\n\
help-ux\n\
help-topic pipeline\n\
help-topic session\n\
help-topic recovery\n\
whereami\n\
command-card identity-of\n\
examples identity-of\n\
suggest pro\n\
suggest-next review\n\
apropos mount\n\
explain-command identity-of\n\
echo shell-ergonomics\n\
repeat-last\n\
rerun-find shell-ergonomics\n\
recent-work 4\n\
history-tail 3\n\
history-find shell-ergonomics\n\
missing-helper\n\
assert-status 127 && echo shell.smoke.ux suggest=pro apropos=mount explain=identity-of unknown=feedback outcome=ok\n\
echo shell.smoke.ergonomics topic=pipeline examples=identity-of repeat=yes rerun=yes recent=yes next=review outcome=ok\n\
set NOTE shell-proof\n\
alias say echo $NOTE\n\
mkdir-path /shell-proof\n\
mkfile-path /shell-proof/note\n\
write-file /shell-proof/note $NOTE\n\
append-file /shell-proof/note -ok\n\
mkfile-path /shell-proof/script\n\
write-file /shell-proof/script echo sourced-proof\n\
source-file /shell-proof/script\n\
assert-file-contains /shell-proof/note shell-proof-ok && echo shell.smoke.scripting path=/shell-proof/note bytes=14 source=yes outcome=ok\n\
fn build-note(value) { if $value == lang { return shell-proof-lang } else { return invalid } }\n\
call-set SCRIPT_NOTE build-note lang\n\
assert-status 0 && echo shell.smoke.lang return=$SCRIPT_NOTE argc=1 outcome=ok\n\
mkfile-path /shell-proof/match.note\n\
match $SCRIPT_NOTE {\n\
case shell-proof-lang {\n\
write-file /shell-proof/match.note $SCRIPT_NOTE\n\
set MATCH_RESULT matched\n\
}\n\
else {\n\
set MATCH_RESULT invalid\n\
}\n\
}\n\
assert-file-contains /shell-proof/match.note shell-proof-lang && echo shell.smoke.match result=$MATCH_RESULT value=$SCRIPT_NOTE outcome=ok\n\
record-set BUILD_RESULT kind=diagnostic severity=warning path=src/lib.rs line=11\n\
value-type BUILD_RESULT\n\
value-show BUILD_RESULT\n\
record-get BUILD_RESULT path BUILD_PATH\n\
assert-status 0 && echo shell.smoke.values type=record path=$BUILD_PATH outcome=ok\n\
record kind=diagnostic severity=warning path=src/lib.rs line=11 |> record-get path PIPE_PATH\n\
assert-status 0 && echo shell.smoke.pipeline path=$PIPE_PATH type=string outcome=ok\n\
session |> record-get protocol SESSION_PROTOCOL\n\
process-info 1 |> record-get state INIT_STATE\n\
ps |> list-count PROCESS_COUNT\n\
ps |> list-first FIRST_PROCESS_PID\n\
mkdomain shell-proof-render\n\
set SHELL_WAIT_DOMAIN $LAST_DOMAIN_ID\n\
mkresource $SHELL_WAIT_DOMAIN device shell-proof-gpu\n\
set SHELL_WAIT_RESOURCE $LAST_RESOURCE_ID\n\
mkcontract $SHELL_WAIT_DOMAIN $SHELL_WAIT_RESOURCE display shell-proof-primary\n\
set SHELL_WAIT_PRIMARY $LAST_CONTRACT_ID\n\
mkcontract $SHELL_WAIT_DOMAIN $SHELL_WAIT_RESOURCE display shell-proof-mirror\n\
set SHELL_WAIT_MIRROR $LAST_CONTRACT_ID\n\
domains |> filter-contains $SHELL_WAIT_DOMAIN |> list-count DOMAIN_MATCH_COUNT\n\
resources |> filter-contains $SHELL_WAIT_RESOURCE |> list-count RESOURCE_MATCH_COUNT\n\
contracts |> filter-contains $SHELL_WAIT_PRIMARY |> list-count CONTRACT_MATCH_COUNT\n\
assert-status 0 && echo shell.smoke.pipeline-inventory domains=$DOMAIN_MATCH_COUNT resources=$RESOURCE_MATCH_COUNT contracts=$CONTRACT_MATCH_COUNT outcome=ok\n\
mkdir-path /run\n\
mksock-path /run/net0.sock\n\
net-config /dev/net0 10.1.0.2 255.255.255.0 10.1.0.1\n\
net-admin /dev/net0 1500 4 4 2 up promisc\n\
netif /dev/net0 |> record-get addr NETIF_ADDR\n\
netif /dev/net0 |> record-get admin NETIF_ADMIN\n\
assert-status 0 && echo shell.smoke.pipeline-netif path=/dev/net0 addr=$NETIF_ADDR admin=$NETIF_ADMIN outcome=ok\n\
udp-bind /run/net0.sock /dev/net0 4020 0.0.0.0 0\n\
udp-connect /run/net0.sock 10.1.0.9 5000\n\
netsock /run/net0.sock |> record-get local_port NETSOCK_LOCAL_PORT\n\
netsock /run/net0.sock |> record-get connected NETSOCK_CONNECTED\n\
assert-status 0 && echo shell.smoke.pipeline-netsock path=/run/net0.sock port=$NETSOCK_LOCAL_PORT connected=$NETSOCK_CONNECTED outcome=ok\n\
queue-create epoll\n\
queue-create kqueue\n\
queues |> filter-contains Epoll |> list-count EPOLL_QUEUE_COUNT\n\
queues |> filter-contains Kqueue |> list-count KQUEUE_QUEUE_COUNT\n\
assert-status 0 && echo shell.smoke.pipeline-queues epoll=$EPOLL_QUEUE_COUNT kqueue=$KQUEUE_QUEUE_COUNT outcome=ok\n\
fdinfo 0 |> record-get kind FD0_KIND && echo shell.smoke.pipeline-fd source=list kind=$FD0_KIND outcome=ok\n\
maps 1 |> list-count MAP_COUNT && echo shell.smoke.pipeline-maps pid=1 source=list outcome=ok\n\
vmobjects 1 |> list-count VMOBJECT_COUNT\n\
vmdecisions 1 |> list-count VMDECISION_COUNT\n\
vmepisodes 1 |> list-count VMEPISODE_COUNT && echo shell.smoke.pipeline-vm objects=$VMOBJECT_COUNT decisions=$VMDECISION_COUNT episodes=$VMEPISODE_COUNT outcome=ok\n\
queues |> filter-suffix queue |> list-count QUEUE_SUFFIX_COUNT\n\
fd |> list-last FD_LAST\n\
ps |> list-at 0 PROCESS_ZERO && echo shell.smoke.pipeline-ops queue-suffix=$QUEUE_SUFFIX_COUNT fd-last=$FD_LAST pid0=$PROCESS_ZERO outcome=ok\n\
record name=shell kind=proof mode=semantic |> record-keys |> list-sort |> list-first RECORD_KEY_FIRST\n\
record name=shell kind=proof mode=semantic |> record-keys |> list-find kind RECORD_KIND_KEY\n\
record name=shell kind=proof mode=semantic |> record-values |> filter-eq proof |> list-count RECORD_VALUE_MATCH\n\
record name=shell kind=proof mode=semantic |> record-values |> list-find-eq proof RECORD_PROOF_VALUE\n\
record name=shell kind=proof mode=semantic |> record-has mode && echo shell.smoke.pipeline-query key=$RECORD_KEY_FIRST found=$RECORD_KIND_KEY exact=$RECORD_PROOF_VALUE value-match=$RECORD_VALUE_MATCH has-mode=true outcome=ok\n\
string semantic-shell |> string-contains shell\n\
string semantic-shell |> string-starts-with sem\n\
string semantic-shell |> string-ends-with shell\n\
bool true |> not\n\
string \"\" |> is-empty && echo shell.smoke.pipeline-bool contains=true starts=true ends=true not=false empty=true outcome=ok\n\
caps 1 |> list-count CAP_COUNT && echo shell.smoke.pipeline-caps count=$CAP_COUNT outcome=ok\n\
pending-signals 1 |> list-count PENDING_SIGNAL_COUNT\n\
blocked-signals 1 |> list-count BLOCKED_SIGNAL_COUNT && echo shell.smoke.pipeline-signals pending=$PENDING_SIGNAL_COUNT blocked=$BLOCKED_SIGNAL_COUNT outcome=ok\n\
record name=shell kind=proof mode=semantic |> record-fields |> list-sort |> list-join , |> string-split , |> list-count RECORD_ROUNDTRIP_COUNT\n\
string hello,shell |> string-split , |> list-at 0 STRING_HEAD\n\
record alpha=one beta=two |> record-fields |> pairs-to-record |> record-get beta PAIR_BETA && echo shell.smoke.pipeline-interop record-count=$RECORD_ROUNDTRIP_COUNT string-head=$STRING_HEAD pair-beta=$PAIR_BETA outcome=ok\n\
record owner=ngos shell=semantic |> into BASE_RECORD\n\
process-info 1 |> record-select pid state caps |> record-merge BASE_RECORD |> record-set-field cap-count $CAP_COUNT |> record-get owner MERGE_OWNER && echo shell.smoke.pipeline-recordops owner=$MERGE_OWNER cap-count=$CAP_COUNT outcome=ok\n\
process-info 1 |> record-select pid state caps pending |> record-drop pending |> record-rename caps capability-count |> record-get capability-count PROCESS_CAPS && echo shell.smoke.pipeline-process capability-count=$PROCESS_CAPS outcome=ok\n\
compat-of 1 |> into COMPAT_RECORD\n\
value-load COMPAT_RECORD |> record-get route COMPAT_ROUTE && echo shell.smoke.pipeline-compat route=$COMPAT_ROUTE outcome=ok\n\
identity-of 1 |> into IDENTITY_RECORD\n\
value-load IDENTITY_RECORD |> record-get root ID_ROOT && echo shell.smoke.pipeline-identity uid=1000 root=$ID_ROOT outcome=ok\n\
value-load IDENTITY_RECORD |> record-eq uid 1000\n\
value-load COMPAT_RECORD |> record-contains route native && echo shell.smoke.pipeline-recordpredicates identity=true compat=true outcome=ok\n\
status-of 1 |> record-get Name STATUS_NAME\n\
auxv-of 1 |> into AUXV_RECORDS\n\
value-load AUXV_RECORDS |> list-count AUXV_COUNT\n\
value-load AUXV_RECORDS |> list-first AUXV_FIRST && echo shell.smoke.pipeline-auxv count=$AUXV_COUNT first=$AUXV_FIRST outcome=ok\n\
cmdline-of 1 |> list-count CMDLINE_COUNT\n\
environ-of 1 |> list-find NGOS_SESSION= SESSION_ENV\n\
root-of 1 |> into ROOT_PATH\n\
cwd-of 1 |> into CWD_PATH\n\
exe-of 1 |> into EXE_PATH && echo shell.smoke.pipeline-procfs status=$STATUS_NAME cmdline=$CMDLINE_COUNT env=$SESSION_ENV root=$ROOT_PATH cwd=$CWD_PATH exe=$EXE_PATH outcome=ok\n\
vfsstats-of 1 |> record-get nodes VFS_NODE_COUNT\n\
vfslocks-of 1 |> list-count VFS_LOCK_COUNT\n\
vfswatches-of 1 |> list-count VFS_WATCH_COUNT && echo shell.smoke.pipeline-vfsstats nodes=$VFS_NODE_COUNT locks=$VFS_LOCK_COUNT watches=$VFS_WATCH_COUNT outcome=ok\n\
claim $SHELL_WAIT_PRIMARY\n\
claim $SHELL_WAIT_MIRROR\n\
waiters $SHELL_WAIT_RESOURCE |> into RESOURCE_WAITERS\n\
value-load RESOURCE_WAITERS |> list-count WAITER_COUNT\n\
value-load RESOURCE_WAITERS |> list-first FIRST_WAITER && echo shell.smoke.pipeline-waiters count=$WAITER_COUNT first=$FIRST_WAITER outcome=ok\n\
releaseclaim $SHELL_WAIT_PRIMARY\n\
release $SHELL_WAIT_MIRROR\n\
storage-mount /dev/storage0 /shell-proof-mount\n\
storage-prepare /dev/storage0 shell-smoke persistent-shell-proof\n\
storage /dev/storage0 |> record-get generation STORAGE_PREPARE_GENERATION && echo shell.smoke.pipeline-storage-prepare generation=$STORAGE_PREPARE_GENERATION outcome=ok\n\
storage-recover /dev/storage0\n\
storage /dev/storage0 |> record-get generation STORAGE_RECOVER_GENERATION && echo shell.smoke.pipeline-storage-recover generation=$STORAGE_RECOVER_GENERATION outcome=ok\n\
storage-repair /dev/storage0\n\
storage /dev/storage0 |> record-get generation STORAGE_REPAIR_GENERATION && echo shell.smoke.pipeline-storage-repair generation=$STORAGE_REPAIR_GENERATION outcome=ok\n\
storage-volume /dev/storage0\n\
storage-volume /dev/storage0 |> record-get generation STORAGE_VOLUME_GENERATION && echo shell.smoke.pipeline-storage-volume generation=$STORAGE_VOLUME_GENERATION outcome=ok\n\
storage-lineage /dev/storage0\n\
storage-history /dev/storage0\n\
storage-history-range /dev/storage0 0 3\n\
storage-history-tail /dev/storage0 3\n\
storage-history-entry /dev/storage0 0\n\
storage /dev/storage0 |> record-get generation STORAGE_GENERATION\n\
storage-history-of /dev/storage0 |> list-count STORAGE_HISTORY_COUNT\n\
storage-history-range-of /dev/storage0 0 3 |> list-count STORAGE_HISTORY_RANGE_COUNT && echo shell.smoke.pipeline-storage-range count=$STORAGE_HISTORY_RANGE_COUNT outcome=ok\n\
storage-history-tail-of /dev/storage0 3 |> list-count STORAGE_HISTORY_TAIL_COUNT && echo shell.smoke.pipeline-storage-tail count=$STORAGE_HISTORY_TAIL_COUNT outcome=ok\n\
storage-history-entry-of /dev/storage0 0 |> record-get kind STORAGE_HISTORY_KIND && echo shell.smoke.pipeline-storage generation=$STORAGE_GENERATION history=$STORAGE_HISTORY_COUNT range=$STORAGE_HISTORY_RANGE_COUNT tail=$STORAGE_HISTORY_TAIL_COUNT kind=$STORAGE_HISTORY_KIND outcome=ok\n\
mount-info /shell-proof-mount |> into ROOT_MOUNT_INFO\n\
value-load ROOT_MOUNT_INFO |> record-get path ROOT_MOUNT_PATH\n\
value-load ROOT_MOUNT_INFO |> record-get mode ROOT_MOUNT_MODE && echo shell.smoke.pipeline-mount path=$ROOT_MOUNT_PATH mode=$ROOT_MOUNT_MODE outcome=ok\n\
mounts |> into ROOT_MOUNTS\n\
value-load ROOT_MOUNTS |> filter-contains /shell-proof-mount |> list-count ROOT_MOUNT_MATCH_COUNT && echo shell.smoke.pipeline-mounts count=$ROOT_MOUNT_MATCH_COUNT outcome=ok\n\
value-load ROOT_MOUNTS |> filter-field-eq device /dev/storage0 |> list-count ROOT_MOUNT_DEVICE_COUNT\n\
value-load ROOT_MOUNTS |> list-field mode |> filter-eq private |> list-count ROOT_MOUNT_MODE_COUNT\n\
value-load AUXV_RECORDS |> list-field AT_EXECFN |> list-first AUXV_EXECFN && echo shell.smoke.pipeline-listfields mount-device=$ROOT_MOUNT_DEVICE_COUNT mount-mode=$ROOT_MOUNT_MODE_COUNT auxv-exec=$AUXV_EXECFN outcome=ok\n\
value-load ROOT_MOUNTS |> list-any-contains /shell-proof-mount\n\
value-load ROOT_MOUNTS |> list-all-contains mode= && echo shell.smoke.pipeline-listpredicates any=true all=true outcome=ok\n\
value-load ROOT_MOUNT_INFO |> record-fields |> filter-contains mode= |> list-count ROOT_MOUNT_FILTER_COUNT && echo shell.smoke.pipeline-filter count=$ROOT_MOUNT_FILTER_COUNT outcome=ok\n\
storage-unmount /shell-proof-mount && echo shell.smoke.pipeline-real source=session outcome=ok\n\
echo shell.smoke.pipeline-system pid=1 outcome=ok\n\
echo shell.smoke.pipeline-list count=$PROCESS_COUNT first=$FIRST_PROCESS_PID outcome=ok\n\
exit 0\n\
";

pub(crate) const SHELL_BOOT_SMOKE_SCRIPT_POST_PIPELINE: &str = "\
write-file /shell-proof/build.log error[E0425]:answer-missing\n\
write-file /shell-proof/review.before mode=old\n\
write-file /shell-proof/review.after mode=new\n\
assert-file-contains /shell-proof/build.log answer-missing\n\
assert-status 0\n\
echo shell.smoke.coding build=/shell-proof/build.log test=/shell-proof/test.log outcome=ok\n\
assert-file-contains /shell-proof/review.after mode=new\n\
assert-status 0\n\
echo shell.smoke.review left=/shell-proof/review.before right=/shell-proof/review.after outcome=ok\n\
exit 0\n\
";

pub(crate) const SHELL_BOOT_SMOKE_SCRIPT_JOBS: &str = "\
spawn-path worker /bin/worker\n\
echo shell.smoke.pipeline-jobs count=1 first=spawned outcome=ok\n\
kill $LAST_PID 9\n\
job-info $LAST_PID\n\
fg $LAST_PID\n\
echo shell.smoke.jobs pid=$LAST_PID outcome=ok\n\
open-path /proc/1/status\n\
assert-status 0 && echo shell.smoke.observe pid=1 procfs=stat-open outcome=ok\n\
exit 0\n\
";

pub(crate) const SHELL_BOOT_SMOKE_SCRIPT_TAIL: &str = "\
missing-command\n\
echo shell.smoke.refusal pid=1 command=missing-command outcome=expected\n\
false\n\
echo shell.smoke.recovery pid=1 guard=or outcome=ok\n\
echo shell.smoke.state pid=1 cwd=/ note=/shell-proof/note outcome=ok\n\
exit 0\n\
";

pub(crate) fn run_native_shell_boot_smoke<B: SyscallBackend>(
    runtime: &Runtime<B>,
    context: &ngos_user_abi::bootstrap::BootContext,
) -> ExitCode {
    let session = boot_context_as_session_context(context);
    let code = run_session_shell_script(runtime, &session, SHELL_BOOT_SMOKE_SCRIPT);
    if code != 0 {
        return code;
    }
    if write_line(runtime, "shell.smoke.phase post-pipeline-enter").is_err() {
        return 198;
    }
    let code = run_session_shell_script(runtime, &session, SHELL_BOOT_SMOKE_SCRIPT_POST_PIPELINE);
    if code != 0 {
        return code;
    }
    let code = run_session_shell_script(runtime, &session, SHELL_BOOT_SMOKE_SCRIPT_JOBS);
    if code != 0 {
        return code;
    }
    let code = run_session_shell_script(runtime, &session, SHELL_BOOT_SMOKE_SCRIPT_TAIL);
    if code != 0 {
        return code;
    }
    if write_line(runtime, "shell-smoke-ok").is_err() {
        return 198;
    }
    0
}

pub(crate) fn boot_context_as_session_context(
    context: &ngos_user_abi::bootstrap::BootContext,
) -> SessionContext {
    SessionContext {
        protocol: context.protocol.clone(),
        outcome_policy: context.boot_outcome_policy,
        process_name: context.process_name.clone(),
        image_path: context.image_path.clone(),
        cwd: context.cwd.clone(),
        root_mount_path: context.root_mount_path.clone(),
        root_mount_name: context.root_mount_name.clone(),
        image_base: context.image_base,
        stack_top: context.stack_top,
        phdr: context.phdr,
        phent: context.phent,
        phnum: context.phnum,
        page_size: context.page_size,
        entry: context.entry,
        cpu: context.cpu,
    }
}

pub(crate) fn emit_boot_cpu_contract<B: SyscallBackend>(
    runtime: &Runtime<B>,
    context: &ngos_user_abi::bootstrap::BootContext,
) -> Result<(), ExitCode> {
    write_line(
        runtime,
        &format!(
            "boot.cpu xsave={} save_area={} xcr0=0x{:x} seed=0x{:x} hw_provider={}",
            context.cpu.xsave_managed,
            context.cpu.save_area_bytes,
            context.cpu.xcr0_mask,
            context.cpu.boot_seed_marker,
            context.cpu.hardware_provider_available
        ),
    )
}
