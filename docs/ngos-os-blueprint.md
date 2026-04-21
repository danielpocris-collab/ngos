# NGOS OS Blueprint

## Authority

Acest document este blueprint-ul canonic al sistemului de operare `ngos`.

El descrie:

- forma sistemului ca ansamblu
- straturile autorizate ale OS-ului
- subsistemele canonice
- contractele dintre straturi
- truth path-ul de produs
- nucleul tare care trebuie păstrat corect
- regulile de closure pentru subsistemele strategice

Acest document nu înlocuiește regulile din `AGENTS.md`.
Le completează printr-un model tehnic de sistem, suficient de explicit pentru oameni și pentru LLM-uri.

Pentru matrix-ul canonic subsystem-by-subsystem:

- [ngos-subsystem-closure-matrix.md](/C:/Users/pocri/OneDrive/Desktop/experiment/docs/ngos-subsystem-closure-matrix.md)
- [ngos-execution-evidence-matrix.md](/C:/Users/pocri/OneDrive/Desktop/experiment/docs/ngos-execution-evidence-matrix.md)
- [ngos-canonical-state-machines.md](/C:/Users/pocri/OneDrive/Desktop/experiment/docs/ngos-canonical-state-machines.md)
- [ngos-subsystem-ownership-dependency-map.md](/C:/Users/pocri/OneDrive/Desktop/experiment/docs/ngos-subsystem-ownership-dependency-map.md)
- [ngos-hardware-platform-matrix.md](/C:/Users/pocri/OneDrive/Desktop/experiment/docs/ngos-hardware-platform-matrix.md)
- [ngos-subsystem-maturity-matrix.md](/C:/Users/pocri/OneDrive/Desktop/experiment/docs/ngos-subsystem-maturity-matrix.md)
- [ngos-interface-contract-catalog.md](/C:/Users/pocri/OneDrive/Desktop/experiment/docs/ngos-interface-contract-catalog.md)
- [ngos-blueprint-gap-report.md](/C:/Users/pocri/OneDrive/Desktop/experiment/docs/ngos-blueprint-gap-report.md)

## Blueprint Goal

`ngos` trebuie să existe ca:

- sistem de operare original
- OS object-centric și capability-aware
- OS nano-semantic, construit ca swarm de agenți semantici mici
- OS cu observabilitate cauzală
- OS refusal-first și recovery-aware
- OS al cărui adevăr final este pe path-ul real, nu pe host-runtime

Blueprint-ul există pentru a împiedica repo-ul să derive în:

- colecție de crate-uri fără model global
- monoliți de orchestrare care absorb autoritatea semantică
- closure declarată pe path-uri auxiliare
- dezvoltare fără hartă a contractelor dintre subsisteme

## Product Truth Path

Path-ul canonic al produsului este:

1. `boot-x86_64`
2. `platform-x86_64`
3. `kernel-core`
4. `user-runtime`
5. `userland-native`
6. `QEMU`
7. hardware fizic

Regulă:

- `host-runtime`
- `platform-host-runtime`
- synthetic validation
- model-only execution

nu sunt truth surface de produs.

Ele sunt instrumente auxiliare.

Un subsistem strategic nu este considerat închis global dacă funcționează numai pe path-uri auxiliare și nu există dovadă reală pe truth path.

## System Layers

### Layer 0: Boot and Bring-Up

Responsabilitate:

- intrare hardware
- CPU feature bring-up
- memorie timpurie
- ACPI / platform facts
- APIC / interrupt bootstrap
- încărcare kernel + module userland
- diagnostics de boot

Crate-uri:

- `boot-x86_64`
- `platform-x86_64`

Contract:

- produce mediu executabil pentru `kernel-core`
- produce facts, diagnostics și handoff-uri reale
- nu definește modelul semantic final al OS-ului

### Layer 1: Kernel Semantic Core

Responsabilitate:

- modelul intern al OS-ului
- procese, threaduri, scheduler
- capability/domain/resource/contract model
- VFS
- VM
- eventing / waits / signals
- device runtime și networking core
- observability și syscall surface

Crate principal:

- `kernel-core`

Contract:

- este centrul semantic al OS-ului
- definește obiectele reale, stările și tranzițiile lor
- nu trebuie contaminat de shape-uri străine de OS

### Layer 2: User ABI and Runtime

Responsabilitate:

- ABI stabil între kernel și user mode
- syscall wrappers
- user-mode bootstrap contract
- runtime suport comun pentru procese user

Crate-uri:

- `user-abi`
- `user-runtime`

Contract:

- transportă adevărul kernelului către userland
- nu inventează alt model de sistem
- nu ascunde starea critică a sistemului

### Layer 3: Native Control Surface

Responsabilitate:

- shell
- control plane
- tooling intern de operare
- proof flows
- observabilitate și explainability pentru operator

Crate principal:

- `userland-native`

Contract:

- este suprafața principală de control a sistemului
- poate fi unificată extern
- intern trebuie să crească nano-semantic, nu ca monolit tot mai mare

### Layer 4: Auxiliary and Validation Surfaces

Responsabilitate:

- accelerare de dezvoltare
- validare auxiliară
- benchmark
- tooling de build/proof

Crate-uri și zone:

- `host-runtime`
- `platform-host-runtime`
- `tooling/*`

Contract:

- accelerează execuția
- nu definesc closure finală pentru subsistemele strategice

## Canonical Subsystems

Subsistemele canonice ale `ngos` sunt:

- boot and diagnostics
- CPU/runtime bring-up
- process model
- scheduler
- capability model
- domain/resource/contract model
- VFS
- VM
- eventing and waits
- signal runtime
- device runtime
- networking
- syscall surface
- observability / procfs-style inspection
- user ABI
- user runtime
- native control/userland

Fiecare subsistem strategic trebuie tratat ca familie completă.
Nu este permisă reducerea unilaterală la un sub-front îngust și declararea lui ca închidere globală.

## Canonical Kernel Objects

Obiectele canonice ale sistemului sunt:

- process
- thread
- capability
- domain
- resource
- contract
- VFS node
- mount
- descriptor
- event queue
- sleep queue
- signal state
- VM object / mapping
- device
- driver
- socket

Regula canonică:

- obiectele au identitate
- obiectele au owner sau autoritate explicită
- obiectele au stare observabilă
- obiectele au tranziții explicabile cauzal

## Verified Core

`verified core` este nucleul tare care trebuie păstrat corect chiar când restul sistemului evoluează.

În forma actuală, blueprint-ul îl fixează astfel:

- capability model verified
- VFS invariants verified
- scheduler state machine verified
- CPU extended state lifecycle verified

Rol:

- constituție tehnică a kernelului
- gardă pentru refactorizări
- bază de adevăr pentru swarm-ul nano-semantic

Regulă:

restul sistemului trebuie construit în jurul acestui nucleu tare, nu prin ocolirea lui.

## Observability Map

Fiecare subsistem strategic trebuie să aibă:

- runtime state real
- suprafață de inspectare
- succes observabil
- refusal observabil
- recovery sau release observabil, dacă există

Suprafețe canonice:

- `inspect_system`
- `inspect_process`
- syscall inspection records
- `/proc/system/*`
- `/proc/<pid>/*`
- diagnostics de boot
- QEMU serial proof logs

Dacă un subsistem nu poate fi explicat prin aceste suprafețe, el este incomplet arhitectural.

## Closure Law For Subsystems

Un subsistem strategic este închis numai dacă:

1. are logică reală
2. este integrat în fluxurile existente
3. produce efect runtime real
4. este observabil
5. are success path
6. are refusal/error path unde e cazul
7. are recovery/release path unde e cazul
8. are dovadă pe `QEMU`
9. are path-ul real relevant implementat, nu doar host validation

Fără toate acestea, subsistemul rămâne deschis.

## Nano-Semantic Swarm Rule

`ngos` nu trebuie să evolueze ca monolit mare cu helperi.

Modelul canonic este:

- agenți semantici înguști
- autoritate locală
- mutație localizată
- orchestrare subțire
- fără centre mari care păstrează semantica reală a subsistemului

Semn de abatere:

- un `lib.rs`, `main.rs` sau fișier central mare care trebuie atins pentru aproape orice lucru nou din subsistem

Blueprint rule:

existența unui fișier mare moștenit nu autorizează dezvoltarea nouă în acel fișier dacă există o separare semantică rezonabilă.

## Runtime Flow Blueprint

Fluxul canonic de execuție este:

1. hardware / firmware
2. `boot-x86_64`
3. `platform-x86_64`
4. `kernel-core`
5. bootstrap user ABI
6. `user-runtime`
7. `userland-native`
8. operator / shell / apps / proofs

Fluxul canonic de feedback este:

1. runtime state
2. observability
3. semantic classification
4. operator control
5. system action
6. final observable state

## Scheduler Blueprint

Schedulerul canonic trebuie să includă:

- queue membership real
- class policy real
- anti-starvation
- service accounting
- fairness observabilă
- verified-core invariants
- procfs visibility
- ABI visibility
- semantic-runtime visibility
- dovadă pe `QEMU`

Familii încă deschise, prin blueprint:

- fairness mai puternică decât agregate pe clasă
- `per-CPU`
- `SMP`
- balancing
- topologie hardware reală

## VM / VFS / Networking Blueprint

Aceste subsisteme trebuie tratate ca familii complete.

Exemple:

- `VFS` nu înseamnă doar `lookup/open`, ci și mount, lifecycle, ownership, refusal, observability
- `VM` nu înseamnă doar `map/unmap`, ci și faults, reclaim, COW, object lineage, policy, observability
- `networking` nu înseamnă doar socket open/send, ci și runtime queues, readiness, pressure, drops, recovery, observability

Blueprint rule:

niciunul nu este considerat închis dacă doar o parte mică a familiei este completă.

## Control Surface Blueprint

`userland-native` este control surface-ul principal.

Trebuie să facă următoarele:

- să expună control real, nu demo
- să consume starea reală a kernelului
- să raporteze canale semantice reale
- să nu devină monolit semantic

`nextmind`, shell agents și alți agenți userland trebuie să fie subordonați adevărului kernelului și truth path-ului real.

## Blueprint Deliverables

Orice lucru strategic nou trebuie să poată fi plasat explicit în blueprint prin:

- layer
- subsystem
- object family
- authority boundary
- observability surface
- real truth path

Dacă un lucru nou nu poate fi plasat clar aici, implementarea lui este prea vagă.

## Canonical Use Of This Blueprint

Acest document trebuie folosit pentru:

- alegerea următorului subsistem
- verificarea deviațiilor arhitecturale
- ghidarea LLM-urilor
- menținerea coerenței între kernel, runtime și userland
- evaluarea dacă o implementare este locală sau globală

## Short Form

Forma scurtă a blueprint-ului este:

- `ngos` este un OS original
- nucleul semantic este `kernel-core`
- truth path-ul este `boot-x86_64 -> platform-x86_64 -> kernel-core -> user-runtime -> userland-native -> QEMU -> hardware`
- subsistemele strategice nu sunt închise fără path real, observabilitate și dovadă end-to-end
- sistemul trebuie construit ca swarm nano-semantic
- `verified core` este constituția tehnică a nucleului
