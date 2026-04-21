# Eventing And Waits Closure Status

`Subsystem Eventing and waits is closed.`

## Scope

Conform [docs/ngos-subsystem-closure-matrix.md](/C:/Users/pocri/OneDrive/Desktop/experiment/docs/ngos-subsystem-closure-matrix.md), în scope intră:

- event queues
- sleep queues
- waiters
- wakeup semantics
- refusal
- queue observability

Minimul de closure pentru acest subsistem este:

- `kernel-core`
- syscall surface
- `user-runtime`

`QEMU` nu este condiție minimă de closure pentru `Eventing and waits`.

## Familii Închise

- event queues reale în owner-ul `kernel-core`:
  - create
  - watch
  - wait
  - descriptor binding
  - timer/process/resource/bus/network/signal/memory watchers
- sleep queues reale:
  - create
  - block
  - timeout
  - wake
  - requeue
  - last sleep result
- memory waits reale, integrate în aceeași familie de waits:
  - wait
  - wake
  - requeue
  - compare-and-requeue
  - wake-op
- refusal și gating reale:
  - procfs queue/wait inspection este refuzat fără contract `Observe`
  - wait path-ul blochează real când nu există eveniment sau wake disponibil
  - wake pe canalul greșit după requeue nu produce trezire falsă
- observabilitate reală:
  - `/proc/system/queues`
  - `/proc/<pid>/queues`
  - `/proc/system/waits`
  - `/proc/<pid>/waits`
  - inspect pe syscall surface pentru event queue și sleep queue descriptors
- wrappers reale în `user-runtime` pentru:
  - `create_event_queue(...)`
  - `wait_event_queue(...)`
  - `watch_process_events(...)`
  - `watch_resource_events(...)`
  - `watch_network_events(...)`
  - `watch_graphics_events(...)`
  - `watch_vfs_events(...)`
  - `watch_bus_events(...)`

## Familii Rămase Deschise

- none pe scope-ul activ al `Eventing and waits`

## Fluxul Închis

Fluxul închis acum dovedește cap-coadă:

- queue descriptorul este creat și inspectabil
- waiterul se blochează real când queue-ul este gol
- wake-ul real îl readuce în `Ready` când evenimentul este enqueued
- sleep queue-ul suportă block, wake direct, timeout și requeue
- refusal-ul de observare fără contract este real pe procfs
- recovery-ul după requeue este real:
  - wake pe sursa veche nu mai trezește waiterul mutat
  - wake pe sursa nouă finalizează flow-ul
- starea finală este observabilă în procfs și în inspect surfaces prin contoare de `waiters`, `pending` și tipurile de queue active

## Dovezi

- owner-ul principal:
  - [kernel-core/src/eventing_model.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/eventing_model.rs)
  - [kernel-core/src/event_queue_runtime.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/event_queue_runtime.rs)
  - [kernel-core/src/memory_wait_runtime.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/memory_wait_runtime.rs)
  - [kernel-core/src/observability.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/observability.rs)
- proof owner:
  - [kernel-core/src/tests/eventing_waits.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/tests/eventing_waits.rs)
- wrappers consumatoare:
  - [user-runtime/src/lib.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/user-runtime/src/lib.rs)

## Verificare

- `cargo test -p ngos-kernel-core runtime_event_queue_waiters_wake_when_event_arrives -- --nocapture`
- `cargo test -p ngos-kernel-core runtime_can_requeue_sleep_waiters_between_channels -- --nocapture`
- `cargo test -p ngos-kernel-core runtime_supports_memory_word_wait_wake_and_requeue -- --nocapture`
- `cargo test -p ngos-kernel-core observe_contract_gates_system_queues_procfs_reads -- --nocapture`
- `cargo test -p ngos-kernel-core observe_contract_gates_system_waits_procfs_reads -- --nocapture`
- `cargo test -p ngos-kernel-core syscall_surface_supports_sleep_queue_operations -- --nocapture`
- `cargo test -p ngos-kernel-core syscall_surface_can_requeue_sleep_waiters -- --nocapture`
- `cargo test -p ngos-user-runtime native_model_wrappers_encode_arguments_as_abi_contract -- --nocapture`
