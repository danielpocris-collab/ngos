# User Runtime Closure Status

`Subsystem User runtime is closed.`

## Scope

Conform [docs/ngos-subsystem-closure-matrix.md](/C:/Users/pocri/OneDrive/Desktop/experiment/docs/ngos-subsystem-closure-matrix.md), în scope intră:

- syscall wrappers
- pressure observation
- semantic state extraction
- bootstrap/session context
- runtime helpers

Minimul de closure pentru acest subsistem este:

- `user-runtime`
- `user-abi`
- `kernel-core`

`QEMU` nu este condiție minimă de closure pentru `User runtime`.

## Familii Închise

- wrappers syscall canonice peste `user-abi` pentru:
  - process
  - resource / contract / domain
  - bus
  - networking
  - event queues
  - VFS
  - boot report
  - snapshot / inspect
- bootstrap/session runtime helpers:
  - `Runtime::start`
  - `_start`
  - integrare cu `BootstrapArgs`
  - construcție coerentă a stack-ului inițial prin contractul ABI
- semantic state extraction în [system_control.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/user-runtime/src/system_control.rs):
  - `observe_pressure`
  - `classify_pressure`
  - `observe_semantic_state`
  - `observe_topology`
  - `plan_pressure_response`
  - `semantic_diagnostics`
- event-stream helpers reale:
  - watch pentru process / resource / network / bus
  - `wait_event_queue`
  - `event_source_name(...)`
- consum corect al adevărului kernelului:
  - `user-runtime` nu reinventează modelul kernelului
  - record-urile rămân tipurile din `user-abi`
  - testele de `kernel-core` confirmă că datele livrate de syscall surface sunt exact cele pe care wrappers și controllerul le consumă

## Familii Rămase Deschise

- none pe scope-ul activ al `User runtime`

## Fluxul Închis

Fluxul închis acum dovedește cap-coadă:

- `user-abi` definește contractul
- `user-runtime` emite frame-uri syscall exacte și decodează rezultatele în tipuri canonice
- `kernel-core` livrează record-uri structurate în user memory
- `SystemController` extrage semantic starea din snapshot și procfs fără să rescrie modelul de kernel
- bootstrap/session helpers rămân compatibili cu contractul first-user și session-launch

## Dovezi

- owner-ul principal:
  - [user-runtime/src/lib.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/user-runtime/src/lib.rs)
  - [user-runtime/src/system_control.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/user-runtime/src/system_control.rs)
  - [user-runtime/src/bootstrap.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/user-runtime/src/bootstrap.rs)
- contractul consumat:
  - [user-abi/src/lib.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/user-abi/src/lib.rs)
  - [user-abi/src/bootstrap.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/user-abi/src/bootstrap.rs)
- truth surface-ul scris de kernel:
  - [kernel-core/src/user_syscall_runtime/tests/basic.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/user_syscall_runtime/tests/basic.rs)

## Verificare

- `cargo test -p ngos-user-runtime native_model_wrappers_encode_arguments_as_abi_contract -- --nocapture`
- `cargo test -p ngos-user-runtime observe_semantic_state_escalates_to_kernel_channel_when_verified_core_is_broken -- --nocapture`
- `cargo test -p ngos-user-runtime observe_topology_reads_real_scheduler_cpu_entries_from_procfs -- --nocapture`
- `cargo test -p ngos-user-runtime event_source_name_reports_bus_for_bus_event_records -- --nocapture`
- `cargo test -p ngos-kernel-core networking_user_syscalls_expose_link_control_and_event_queue_delivery -- --nocapture`
- `cargo test -p ngos-kernel-core stat_and_statfs_user_syscalls_copy_structured_records_into_user_memory -- --nocapture`

## Comportament Observabil

- wrappers emit frame-uri syscall stabile pentru modele native și queue/event surfaces
- semantic controllerul produce canale și presiuni distincte:
  - `proc::steady`
  - `kernel::verified-core`
  - `proc::scheduler`
- topology observation citește date reale din `/proc/system/scheduler` și cade controlat pe snapshot când procfs lipsește
- event records sunt clasificate semantic, inclusiv `bus`, prin `event_source_name(...)`
