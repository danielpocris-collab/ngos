# Signal Runtime Closure Status

`Subsystem Signal runtime is closed.`

## Scope

Conform [docs/ngos-subsystem-closure-matrix.md](/C:/Users/pocri/OneDrive/Desktop/experiment/docs/ngos-subsystem-closure-matrix.md), în scope intră:

- signal send
- signal delivery
- refusal
- signal inspection

Minimul de closure pentru acest subsistem este:

- `kernel-core`
- syscall surface
- observabilitate

`QEMU` nu este condiție minimă de closure pentru `Signal runtime`.

## Familii Închise

- delivery real de semnale în `kernel-core`:
  - `send_signal(...)`
  - pending signals per process
  - pending blocked signals
  - masked signal waits
  - signal disposition și signal masks
- wake/refusal semantics reale:
  - delivery poate întrerupe memory waits când semnalul nu este blocat
  - un semnal blocat rămâne `pending` fără să anuleze wait-ul greșit
  - `wait_for_pending_signal(...)` blochează real, se trezește la delivery și poate expira controlat
- inspectability reală:
  - `/proc/system/signals`
  - `/proc/<pid>/signals`
  - expunere pentru `mask`, `pending`, `blocked`, `blocked-pending`, dispositions și `wait-mask`
- refusal real pe observare:
  - accesul la suprafețele procfs de semnale este refuzat fără contract `Observe`
- syscall surface real:
  - `SYS_SEND_SIGNAL`
  - `SYS_PENDING_SIGNALS`
  - `SYS_BLOCKED_PENDING_SIGNALS`
- wrappers reale în `user-runtime`:
  - `send_signal(...)`
  - `pending_signals(...)`
  - `blocked_pending_signals(...)`

## Familii Rămase Deschise

- none pe scope-ul activ al `Signal runtime`

## Fluxul Închis

Fluxul închis acum dovedește cap-coadă:

- signal-ul este trimis pe syscall surface și ajunge în starea `pending`
- dacă semnalul este blocat, wait-ul neasociat nu este anulat fals și semnalul rămâne inspectabil în `blocked-pending`
- `wait_for_pending_signal(...)` intră în blocare reală și se trezește la masked delivery cu delivery observabil
- același flow poate expira controlat cu rezultat observabil `TimedOut`
- procfs expune starea inițială, intermediară și starea de recovery după consumarea semnalului
- observarea procfs este refuzată fără contract și permisă după bind valid

## Dovezi

- owner-ul principal:
  - [kernel-core/src/observability.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/observability.rs)
  - [kernel-core/src/syscall_surface.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/syscall_surface.rs)
- proof owners:
  - [kernel-core/src/tests/eventing_waits.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/tests/eventing_waits.rs)
  - [kernel-core/src/tests/native_model.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/tests/native_model.rs)
  - [kernel-core/src/user_syscall_runtime/tests/basic.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/user_syscall_runtime/tests/basic.rs)
- wrappers consumatoare:
  - [user-runtime/src/lib.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/user-runtime/src/lib.rs)

## Verificare

- `cargo test -p ngos-kernel-core runtime_signal_delivery_marks_pending_and_cancels_memory_waits -- --nocapture`
- `cargo test -p ngos-kernel-core runtime_blocked_signal_stays_pending_without_canceling_wait -- --nocapture`
- `cargo test -p ngos-kernel-core runtime_wait_for_pending_signal_blocks_and_wakes_on_masked_delivery -- --nocapture`
- `cargo test -p ngos-kernel-core runtime_wait_for_pending_signal_can_timeout_immediately -- --nocapture`
- `cargo test -p ngos-kernel-core procfs_signals_renders_delivery_and_recovery_state -- --nocapture`
- `cargo test -p ngos-kernel-core observe_contract_gates_system_signals_procfs_reads -- --nocapture`
- `cargo test -p ngos-kernel-core observe_contract_gates_process_signals_procfs_reads -- --nocapture`
- `cargo test -p ngos-kernel-core signal_user_syscalls_queue_and_copy_pending_signals -- --nocapture`
- `cargo test -p ngos-user-runtime native_model_wrappers_encode_arguments_as_abi_contract -- --nocapture`
