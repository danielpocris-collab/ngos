# NGOS Subsystem Closure Matrix

## Authority

Acest document este matricea canonică de closure pentru subsistemele strategice din `ngos`.

El completează:

- [ngos-os-blueprint.md](/C:/Users/pocri/OneDrive/Desktop/experiment/docs/ngos-os-blueprint.md)
- `AGENTS.md`

Scopul lui este să elimine ambiguitatea despre:

- ce intră în scope pentru fiecare subsistem
- ce nu intră în scope implicit
- care este layer-ul autoritativ
- care este truth path-ul obligatoriu
- când este permisă oprirea

## How To Read This Matrix

Pentru fiecare subsistem, documentul fixează:

- `owner layer`
- `in-scope`
- `out-of-scope`
- `minimum truth path`
- `stop condition`

Regulă:

dacă un element din `in-scope` nu este închis end-to-end, subsistemul nu este închis.

## 1. Boot and Diagnostics

### Owner Layer

- Layer 0

### In-Scope

- boot entry
- early memory
- platform facts
- CPU bring-up facts
- boot diagnostics
- boot locator / serial proof markers
- handoff spre `kernel-core`
- încărcare kernel și userland bootstrap

### Out-Of-Scope

- policy semantică finală de scheduler / VFS / VM
- logică de control userland
- closure globală a device runtime doar prin diagnostics

### Minimum Truth Path

- `boot-x86_64`
- `platform-x86_64`
- `QEMU`

### Stop Condition

Te poți opri numai când:

- boot-ul produce efect real
- diagnostics sunt observabile
- refusal path există unde e relevant
- handoff-ul către kernel este real
- dovada există pe `QEMU`

## 2. CPU / Runtime Bring-Up

### Owner Layer

- Layer 0 + Layer 1

### In-Scope

- feature detection
- vendor policy Intel/AMD
- enablement controlat
- extended state lifecycle
- TLB / APIC paths relevante
- CPU handoff spre runtime
- observabilitate CPU runtime

### Out-Of-Scope

- portări ARM
- promisiuni despre generații viitoare fără detection/policy reală
- simplă enumerare CPUID fără activation path

### Minimum Truth Path

- `boot-x86_64`
- `platform-x86_64`
- `kernel-core`
- `QEMU`

### Stop Condition

Te poți opri numai când:

- există detection
- există activation / refusal
- există observabilitate
- există handoff real către runtime
- există dovadă pe `QEMU`

## 3. Process Model

### Owner Layer

- Layer 1

### In-Scope

- lifecycle de proces
- lifecycle de thread
- spawn / exit / reap
- ownership
- state transitions
- process introspection
- thread introspection

### Out-Of-Scope

- shell scripting
- compat layers
- scheduling policy mai avansată decât lifecycle-ul procesului

### Minimum Truth Path

- `kernel-core`
- `user-runtime`
- `userland-native`
- `QEMU`

### Stop Condition

Te poți opri numai când:

- state machine-ul este real
- refusal path există
- release / reap există
- observabilitatea este reală
- dovada merge pe path-ul real

## 4. Scheduler

### Owner Layer

- Layer 1

### In-Scope

- queue membership
- class policy
- urgent wakeup
- anti-starvation
- lag/debt
- service accounting
- fairness summary
- scheduler observability
- scheduler verified-core invariants
- scheduler ABI/runtime/userland propagation
- scheduler proof pe `QEMU`

### Out-Of-Scope

- presupunerea că `tokens/wait-ticks` singure închid subsistemul
- closure globală fără `QEMU`
- `per-CPU / SMP / balancing` dacă nu sunt demonstrate explicit

### Minimum Truth Path

- `kernel-core`
- `user-runtime`
- `userland-native`
- `boot-x86_64`
- `QEMU`

### Stop Condition

Te poți opri numai când:

- policy-ul este real
- fairness-ul este observabil
- verified-core acoperă invariants relevante
- shell/proof-ul exercită success/refusal/recovery
- există dovadă pe `QEMU`

Schedulerul nu este închis global cât timp:

- `per-CPU`
- `SMP`
- balancing
- topologie hardware

rămân deschise.

## 5. Capability Model

### Owner Layer

- Layer 1

### In-Scope

- capability identity
- owner
- target binding
- rights
- refusal la autoritate invalidă
- observabilitate capability
- invariants în verified-core

### Out-Of-Scope

- path-only authority ca adevăr final
- prezentare superficială fără real enforcement

### Minimum Truth Path

- `kernel-core`
- syscall surface
- observabilitate

### Stop Condition

Te poți opri numai când:

- authority este reală
- rights sunt reale
- refusal path există
- observabilitatea există
- verified-core verifică invariants

## 6. Domain / Resource / Contract Model

### Owner Layer

- Layer 1

### In-Scope

- domain lifecycle
- resource lifecycle
- contract binding
- ownership
- refusal
- inspectability

### Out-Of-Scope

- naming decorativ fără lifecycle real
- contracts doar ca metadata fără efect runtime

### Minimum Truth Path

- `kernel-core`
- syscall surface
- `user-runtime`
- `userland-native`

### Stop Condition

Te poți opri numai când:

- obiectele există real
- authority și lifecycle sunt reale
- inspectability există
- refusal/release există unde e relevant

## 7. VFS

### Owner Layer

- Layer 1

### In-Scope

- path resolution
- mount graph
- node lifecycle
- symlink handling
- rename / unlink / create / remove
- descriptor coherence
- permission/refusal
- VFS inspection
- `procfs` VFS surfaces
- QEMU proof

### Out-Of-Scope

- simplu `lookup/open`
- host-only validation
- simplă prezentare de path-uri fără lifecycle și refusal

### Minimum Truth Path

- `kernel-core`
- `user-runtime`
- `userland-native`
- `boot-x86_64`
- `QEMU`

### Stop Condition

Te poți opri numai când:

- namespace-ul este real
- refusal path există
- recovery/release există unde e cazul
- descriptor coherence este reală
- există dovadă pe `QEMU`

## 8. VM

### Owner Layer

- Layer 1

### In-Scope

- map / protect / unmap
- faults
- reclaim
- quarantine
- COW
- file-backed mappings
- policy block
- VM observability
- VM decisions / episodes
- QEMU proof

### Out-Of-Scope

- simplu `map/unmap`
- host-only closure

### Minimum Truth Path

- `kernel-core`
- `boot-x86_64`
- `user-runtime`
- `userland-native`
- `QEMU`

### Stop Condition

Te poți opri numai când:

- familia VM este închisă end-to-end
- observabilitatea există
- policy/refusal există
- QEMU proof există

## 9. Eventing and Waits

### Owner Layer

- Layer 1

### In-Scope

- event queues
- sleep queues
- waiters
- wakeup semantics
- refusal
- queue observability

### Out-Of-Scope

- polling indirect ca substitut pentru eventing real

### Minimum Truth Path

- `kernel-core`
- syscall surface
- `user-runtime`

### Stop Condition

Te poți opri numai când:

- wait path-ul funcționează real
- wake/refusal există
- queue state este inspectabilă

## 10. Signal Runtime

### Owner Layer

- Layer 1

### In-Scope

- signal send
- signal delivery
- refusal
- signal inspection

### Out-Of-Scope

- semnalistică doar ca compat shim fără model propriu `ngos`

### Minimum Truth Path

- `kernel-core`
- syscall surface
- observabilitate

### Stop Condition

Te poți opri numai când:

- delivery este reală
- refusal există
- inspectability există

## 11. Device Runtime

### Owner Layer

- Layer 1 + Layer 0

### In-Scope

- device model
- driver model
- request lifecycle
- queueing
- refusal
- observabilitate
- runtime behavior real
- proof real pe path-ul sistemului

### Out-Of-Scope

- descrieri de device fără request lifecycle
- smoke local fără integrare cu boot/platform

### Minimum Truth Path

- `platform-x86_64`
- `kernel-core`
- `user-runtime`
- `userland-native`
- `QEMU`

### Stop Condition

Te poți opri numai când:

- request lifecycle există
- refusal/recovery există
- inspectability există
- există dovadă pe path-ul real

## 12. Networking

### Owner Layer

- Layer 1 + Layer 0

### In-Scope

- sockets
- readiness
- RX/TX queues
- drops
- pressure
- watch events
- networking observability
- refusal/recovery
- QEMU proof

### Out-Of-Scope

- simplă deschidere de socket
- host-only networking ca truth final

### Minimum Truth Path

- `platform-x86_64`
- `kernel-core`
- `user-runtime`
- `userland-native`
- `QEMU`

### Stop Condition

Te poți opri numai când:

- networking-ul este observabil
- pressure și drops sunt reale
- refusal/recovery există
- există dovadă pe path-ul real

## 13. Syscall Surface

### Owner Layer

- Layer 1 + Layer 2

### In-Scope

- syscall routing
- success path
- refusal path
- dispatch trace
- inspect surfaces
- ABI transport

### Out-Of-Scope

- wrappers locale fără dispatch real
- ABI fictiv fără integrare în kernel

### Minimum Truth Path

- `kernel-core`
- `user-abi`
- `user-runtime`

### Stop Condition

Te poți opri numai când:

- syscall-urile sunt reale
- refusal există
- dispatch-ul este observabil
- ABI-ul este real și stabil

## 14. Observability / Procfs

### Owner Layer

- Layer 1

### In-Scope

- `/proc/system/*`
- `/proc/<pid>/*`
- causal inspection
- final state visibility
- contract-based access gating

### Out-Of-Scope

- debug prints fără model stabil
- observabilitate doar pentru success path

### Minimum Truth Path

- `kernel-core`
- `user-runtime`
- `userland-native`
- `QEMU`

### Stop Condition

Te poți opri numai când:

- procfs este real
- gating există
- success/refusal/final state sunt vizibile

## 15. User ABI

### Owner Layer

- Layer 2

### In-Scope

- records native
- bootstrap contract
- snapshot contract
- typed transport

### Out-Of-Scope

- record fields nefolosite și fără semnificație
- drift între ABI și kernel truth

### Minimum Truth Path

- `kernel-core`
- `user-abi`
- `user-runtime`

### Stop Condition

Te poți opri numai când:

- transportă adevărul kernelului
- are helperi canonici
- este testat

## 16. User Runtime

### Owner Layer

- Layer 2

### In-Scope

- syscall wrappers
- pressure observation
- semantic state extraction
- bootstrap/session context
- runtime helpers

### Out-Of-Scope

- reinvenția modelului de kernel
- control surface mare care aparține `userland-native`

### Minimum Truth Path

- `user-runtime`
- `user-abi`
- `kernel-core`

### Stop Condition

Te poți opri numai când:

- consumă adevărul kernelului corect
- clasifică semantic corect
- este testat cap-coadă

## 17. Native Control / Userland

### Owner Layer

- Layer 3

### In-Scope

- shell
- operator control
- proof flows
- explanation surfaces
- semantic control plane

### Out-Of-Scope

- centru monolitic care înghite toată logica OS-ului
- path-uri demo-only

### Minimum Truth Path

- `userland-native`
- `user-runtime`
- `kernel-core`
- `QEMU`

### Stop Condition

Te poți opri numai când:

- controlul este real
- output-ul este observabil
- proof flow-ul e real
- success/refusal/recovery există unde e relevant

## 18. Host Runtime and Synthetic Validation

### Owner Layer

- Layer 4

### In-Scope

- accelerare
- testare auxiliară
- izolare de probleme

### Out-Of-Scope

- closure finală
- adevăr de produs
- substitut pentru `QEMU`

### Minimum Truth Path

- none as final destination

### Stop Condition

Te poți opri numai când helperul auxiliar sprijină path-ul real.
Nu te poți opri aici pentru a declara închiderea unui subsistem strategic.
