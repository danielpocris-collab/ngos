# Process Model Closure Status

`Subsystem Process Model is closed.`

## Scope

Acest document fixează closure-ul pentru subsistemul `Process Model` pe truth path-ul cerut de repo:

- `kernel-core`
- `user-runtime`
- `userland-native`
- `QEMU`

## Familii închise

- lifecycle real de proces:
  - `spawn`
  - `exit`
  - `kill`
  - `reap`
- ownership și relație părinte-copil observabile prin `inspect_process` / `process-info`
- state transitions reale:
  - `Running`
  - child observabil înainte de reap
  - child absent sau reaped după flow-ul final
- introspecție de proces reală:
  - `process-info`
  - `status-of`
  - `cmdline-of`
  - `environ-of`
  - `root-of`
  - `cwd-of`
  - `exe-of`
  - procfs aferent pe path-ul real
- introspecție de thread suficientă pe scope-ul activ:
  - `main_thread`
  - `thread_count`
  - thread ownership în `kernel-core`
- refusal și release reale:
  - `metadata-only` refusal/observe pe proof-ul `process-exec`
  - `reap` final care eliberează procesul și îl scoate din starea live

## Familii rămase deschise

- none pe scope-ul activ al `Process Model`

## Ce a fost închis acum

- proof vertical dedicat în shell/runtime pentru lifecycle și introspecție:
  - [userland-native/src/tests/process_model_tests.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/userland-native/src/tests/process_model_tests.rs)

Acest proof dovedește cap-coadă:

- inspectarea procesului curent
- semnalizare și observabilitate a pending signals
- spawn de child
- introspecție a child-ului cu `parent`, `thread`, `threads`
- stare intermediară `Exited` observabilă prin `job-info`
- reap final prin `fg`
- stare finală `reaped:137`

## Dovezi existente pe path-ul real

- proof `QEMU` pentru lifecycle/refusal/recovery/reap:
  - [docs/process-exec-closure-status.md](/C:/Users/pocri/OneDrive/Desktop/experiment/docs/process-exec-closure-status.md)
  - [tooling/x86_64/prove-qemu-process-exec-smoke.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/prove-qemu-process-exec-smoke.ps1)
  - [tooling/x86_64/verify-qemu-process-exec-log.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/verify-qemu-process-exec-log.ps1)
- thread/process ownership și introspecție în owner-ul real:
  - [kernel-core/src/tests/runtime_process.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/tests/runtime_process.rs)
  - [kernel-core/src/tests/native_model.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/tests/native_model.rs)
  - [boot-x86_64/src/user_bridge.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/user_bridge.rs)

## Verificare

- `cargo test -p ngos-userland-native native_shell_closes_process_model_vertical -- --nocapture`
- `cargo test -p ngos-userland-native native_program_runs_process_exec_bootproof_and_reports_process_exec_markers -- --nocapture`
- `cargo test -p ngos-kernel-core runtime_can_block_wake_exit_and_reap_running_processes -- --nocapture`
