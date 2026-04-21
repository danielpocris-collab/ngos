# Syscall Surface Closure Status

`Subsystem Syscall surface is closed.`

## Scope

Conform [docs/ngos-subsystem-closure-matrix.md](/C:/Users/pocri/OneDrive/Desktop/experiment/docs/ngos-subsystem-closure-matrix.md), în scope intră:

- syscall routing
- success path
- refusal path
- dispatch trace
- inspect surfaces
- ABI transport

Minimul de closure pentru acest subsistem este:

- `kernel-core`
- `user-abi`
- `user-runtime`

`QEMU` nu este condiție minimă de closure pentru `Syscall surface`.

## Familii Închise

- routing real în `kernel-core` prin [syscall_surface.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/syscall_surface.rs):
  - dispatch pe familii reale de proces, capability, domain/resource/contract, VFS, networking, eventing, inspect și snapshot
- success path real:
  - syscall-urile produc mutații și rezultate reale în runtime, nu doar wrapper-e locale
- refusal path real:
  - authority insuficientă este refuzată explicit
  - payload invalid și cereri nepermise sunt respinse pe dispatch-ul real
- dispatch trace observabil:
  - `InspectSystem` expune `syscall_dispatches`
  - trace-ul reține familia, caller-ul, rezultatul și refusal-urile recente
- inspect surfaces reale:
  - `InspectSystem`
  - `Snapshot`
  - inspect/list pentru obiectele majore deja rutate prin syscall surface
- ABI transport real:
  - `user_syscall_runtime` citește și scrie record-uri structurate în user memory
  - `user-runtime` emite frame-uri syscall canonice peste `user-abi`
  - record-urile copiate în user memory rămân tipurile native din ABI, nu variante ad-hoc

## Familii Rămase Deschise

- none pe scope-ul activ al `Syscall surface`

## Fluxul Închis

Fluxul închis acum dovedește cap-coadă:

- `user-runtime` emite frame-uri canonice peste `user-abi`
- `kernel-core` decodează și rutează syscall-ul către owner-ul semantic real
- success path-ul produce efect runtime și rezultate inspectabile
- refusal path-ul este observabil în dispatch trace și în return path
- `InspectSystem` și `Snapshot` expun starea sistemului și istoricul recent de dispatch
- syscall-urile care copiază rezultate în user memory livrează record-uri structurate reale și listări reale, inclusiv procfs și semnale

## Dovezi

- owner-ul principal:
  - [kernel-core/src/syscall_surface.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/syscall_surface.rs)
  - [kernel-core/src/observability.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/observability.rs)
- proof owners:
  - [kernel-core/src/tests/syscall_surface.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/tests/syscall_surface.rs)
  - [kernel-core/src/user_syscall_runtime/tests/basic.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/user_syscall_runtime/tests/basic.rs)
- ABI/runtime contract:
  - [user-abi/src/lib.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/user-abi/src/lib.rs)
  - [user-runtime/src/lib.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/user-runtime/src/lib.rs)

## Verificare

- `cargo test -p ngos-kernel-core syscall_surface_dispatches_runtime_operations -- --nocapture`
- `cargo test -p ngos-kernel-core syscall_surface_records_dispatch_trace_and_refusal_paths -- --nocapture`
- `cargo test -p ngos-kernel-core process_listing_and_procfs_user_syscalls_copy_results_into_user_memory -- --nocapture`
- `cargo test -p ngos-kernel-core signal_user_syscalls_queue_and_copy_pending_signals -- --nocapture`
- `cargo test -p ngos-kernel-core stat_and_statfs_user_syscalls_copy_structured_records_into_user_memory -- --nocapture`
- `cargo test -p ngos-user-runtime native_model_wrappers_encode_arguments_as_abi_contract -- --nocapture`
