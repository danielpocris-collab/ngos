# Capability Closure Status

`Subsystem capability model is closed.`

## Scope

Acest document fixează statusul subsistemului `capability model` pe truth path-ul cerut de repo:

- `kernel-core`
- syscall surface
- observabilitate

Conform [docs/ngos-subsystem-closure-matrix.md](/C:/Users/pocri/OneDrive/Desktop/experiment/docs/ngos-subsystem-closure-matrix.md), pentru acest subsistem nu este cerut `QEMU` ca minim de closure.

## Familii închise

- identitate reală de capability prin:
  - `CapabilityId`
  - `owner`
  - `target`
  - `label`
- rights reale prin:
  - `READ`
  - `WRITE`
  - `EXECUTE`
  - `DUPLICATE`
  - `TRANSFER`
  - `ADMIN`
- grant și duplicate restrâns pe syscall surface:
  - [kernel-core/src/process_vm_dispatch.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/process_vm_dispatch.rs)
  - [kernel-core/src/syscall_surface.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/syscall_surface.rs)
- refusal la autoritate invalidă:
  - duplicate fără `DUPLICATE` este refuzat cu `RightDenied`
  - duplicate peste rights-ul disponibil este refuzat de aceeași masca de rights
- revocare reală:
  - capability-ul este eliminat din tabelul viu și nu mai rămâne observabil la owner
- observabilitate reală:
  - `/proc/<pid>/caps` expune `capability-id`, `target-id`, `rights`, `label`
  - snapshot-ul de sistem și `verify_core()` raportează `capability_model_verified`
- verified-core invariants:
  - capability table este verificat în raportul nucleului și propagat în snapshot

## Familii rămase deschise

- none pe scope-ul activ al `capability model`

## Ce a fost închis acum

- proof dedicat de closure pentru success, refusal, recovery, revocation și final observable state:
  - [kernel-core/src/tests/capability_model.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/tests/capability_model.rs)

Fluxul închis acum dovedește cap-coadă:

- grant root capability
- duplicate restrâns cu rights mai mici
- refusal când holder-ul fără `DUPLICATE` încearcă delegare
- recovery prin duplicate valid din owner-ul root
- revocare
- stare finală observabilă goală în `/proc/<pid>/caps`
- verified-core și snapshot consistente după flow

## Verificare

- `cargo test -p ngos-kernel-core capability_model_closes_identity_rights_refusal_recovery_and_observability -- --nocapture`
- `cargo test -p ngos-kernel-core syscall_surface_dispatches_runtime_operations -- --nocapture`

## Dovezi

- [kernel-core/src/core_objects.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/core_objects.rs)
- [kernel-core/src/observability.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/observability.rs)
- [kernel-core/src/tests/capability_model.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/tests/capability_model.rs)
