# Domain / Resource / Contract Closure Status

`Subsystem Domain / Resource / Contract Model is closed.`

## Scope

Acest document fixează closure-ul pentru subsistemul `Domain / Resource / Contract Model` pe truth path-ul cerut de repo:

- `kernel-core`
- syscall surface
- `user-runtime`
- `userland-native`

Conform [docs/ngos-subsystem-closure-matrix.md](/C:/Users/pocri/OneDrive/Desktop/experiment/docs/ngos-subsystem-closure-matrix.md), acest subsistem nu cere `QEMU` ca minim de closure.

## Familii închise

- lifecycle real pentru:
  - `domain`
  - `resource`
  - `contract`
- ownership real:
  - owner de domain
  - creator de resource
  - issuer de contract
  - binding explicit `contract -> resource -> domain`
- inspectability reală pe toate straturile:
  - `inspect_domain`
  - `inspect_resource`
  - `inspect_contract`
  - listare prin `domains`, `resources`, `contracts`, `waiters`
- policy și state reale:
  - `resource arbitration`
  - `resource governance`
  - `resource contract policy`
  - `resource issuer policy`
  - `contract state`
  - `resource state`
- refusal și release reale:
  - `invoke` refuză contract suspendat cu `EACCES`
  - `claim` poate intra în `queued` și este eliberat prin `releaseclaim` / `release`
  - handoff-ul între contracte este observabil
- wrappers `user-runtime` reale pentru create / inspect / state / claim / release / transfer / invoke:
  - [user-runtime/src/lib.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/user-runtime/src/lib.rs)
- front shell real în `userland-native`:
  - `mkdomain`
  - `mkresource`
  - `mkcontract`
  - `domains`
  - `domain`
  - `resources`
  - `resource`
  - `contracts`
  - `contract`
  - `waiters`
  - `claim`
  - `releaseclaim`
  - `release`
  - `transfer`
  - `invoke`
  - `contract-state`
  - `resource-state`
  - `resource-policy`
  - `resource-governance`
  - `resource-contract-policy`
  - `resource-issuer-policy`

## Familii rămase deschise

- none pe scope-ul activ al `Domain / Resource / Contract Model`

## Ce a fost închis acum

- proof vertical dedicat în owner-ul membranei `userland-native`:
  - [userland-native/src/tests/domain_resource_contract_tests.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/userland-native/src/tests/domain_resource_contract_tests.rs)

Fluxul închis acum dovedește cap-coadă:

- create `domain -> resource -> contract`
- inspect și listare pentru toate obiectele
- `claim` succes pe contractul principal
- `claim` queued pe contractul secundar
- `releaseclaim` cu handoff observabil
- refusal pe `invoke` cât timp contractul este suspendat
- recovery prin reactivare și `invoke` reușit
- release final cu stare observabilă `holder=0 waiters=0`

## Verificare

- `cargo test -p ngos-userland-native native_shell_closes_domain_resource_contract_model_vertical -- --nocapture`
- `cargo test -p ngos-userland-native native_shell_can_transfer_and_release_resource_through_contract_commands -- --nocapture`
- `cargo test -p ngos-userland-native native_shell_reports_resource_governance_refusal_and_recovery_semantically -- --nocapture`

## Dovezi

- [kernel-core/src/runtime_core/native_model.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/runtime_core/native_model.rs)
- [kernel-core/src/syscall_surface.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/syscall_surface.rs)
- [user-runtime/src/lib.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/user-runtime/src/lib.rs)
- [userland-native/src/tests/domain_resource_contract_tests.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/userland-native/src/tests/domain_resource_contract_tests.rs)
