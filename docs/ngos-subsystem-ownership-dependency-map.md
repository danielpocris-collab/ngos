# NGOS Subsystem Ownership And Dependency Map

## Authority

Acest document fixează:

- ownership-ul canonic pe straturi și subsisteme
- granițele de autoritate
- dependențele permise dintre subsisteme
- direcția corectă de integrare

Scopul lui este să împiedice:

- dependințe inverse greșite
- scurgeri de autoritate între straturi
- centralizare accidentală în orchestratori mari
- mutații cross-domain fără proprietar clar

## Ownership Rule

Fiecare subsistem strategic trebuie să aibă:

- un `owner layer`
- un `semantic owner`
- o frontieră clară între:
  - cine definește modelul
  - cine transportă modelul
  - cine îl observă
  - cine îl operează

## Ownership Map

| Subsystem | Owner Layer | Semantic Owner | Operational Consumer | Notes |
| --- | --- | --- | --- | --- |
| Boot and diagnostics | Layer 0 | `boot-x86_64` + `platform-x86_64` | `kernel-core`, diagnostics, proofs | nu definește modelul final de kernel |
| CPU/runtime bring-up | Layer 0 + 1 | `boot-x86_64` + `kernel-core` | `user-runtime`, `userland-native` | boot activează, kernel consumă și menține |
| Process model | Layer 1 | `kernel-core` | `user-runtime`, `userland-native` | model intern canonic |
| Scheduler | Layer 1 | `kernel-core` | `user-runtime`, `userland-native`, QEMU proofs | shell-ul nu definește policy, doar o consumă |
| Capability model | Layer 1 | `kernel-core` | syscall surface, observability | authority truth rămâne în kernel |
| Domain/resource/contract | Layer 1 | `kernel-core` | `user-runtime`, `userland-native` | object authority rămâne în kernel |
| VFS | Layer 1 | `kernel-core` | `user-runtime`, `userland-native` | path și descriptor truth în kernel |
| VM | Layer 1 | `kernel-core` | `user-runtime`, `userland-native` | policy și object lineage în kernel |
| Eventing and waits | Layer 1 | `kernel-core` | `user-runtime`, `userland-native` | wake/refusal truth în kernel |
| Signal runtime | Layer 1 | `kernel-core` | `user-runtime`, `userland-native` | model canonic în kernel |
| Device runtime | Layer 1 + 0 | `kernel-core` + `platform-x86_64` | `user-runtime`, `userland-native` | platforma mediază hardware, kernelul păstrează modelul |
| Networking | Layer 1 + 0 | `kernel-core` + `platform-x86_64` | `user-runtime`, `userland-native` | counters și readiness truth în kernel |
| Syscall surface | Layer 1 + 2 | `kernel-core` + `user-abi` | `user-runtime` | ABI transportă, nu reinventează |
| Observability / procfs | Layer 1 | `kernel-core` | `user-runtime`, `userland-native`, proofs | procfs este kernel truth surface |
| User ABI | Layer 2 | `user-abi` | `user-runtime`, `userland-native` | contract de transport, nu logică semantică primară |
| User runtime | Layer 2 | `user-runtime` | `userland-native` | extrage și clasifică, nu redefinește |
| Native control/userland | Layer 3 | `userland-native` | operator, shell, proofs | control surface, nu kernel truth |
| Host runtime / synthetic validation | Layer 4 | `host-runtime`, `platform-host-runtime` | development only | nu este owner de produs |

## Dependency Direction Rule

Direcția canonică a dependențelor este:

1. `boot-x86_64` / `platform-x86_64`
2. `kernel-core`
3. `user-abi`
4. `user-runtime`
5. `userland-native`

Dependențele inverse sunt suspecte și trebuie justificate explicit.

## Allowed Dependency Classes

### Class A: Structural

- un layer inferior oferă contract sau adevăr unui layer superior

Exemple:

- `kernel-core -> user-abi`
- `user-abi -> user-runtime`
- `user-runtime -> userland-native`

### Class B: Platform Mediation

- platforma oferă mecanism hardware unui model deja definit semantic în kernel

Exemple:

- `platform-x86_64` pentru paging, APIC, interrupts, DMA

### Class C: Proof / Validation

- tooling sau userland proof consumă suprafețe reale pentru demonstrație

Exemple:

- `tooling/x86_64/*`
- smoke proofs în `userland-native`

## Forbidden Dependency Shapes

Următoarele forme sunt interzise:

- `userland-native` care devine owner semantic pentru process model, scheduler, VFS, VM sau networking
- `user-runtime` care inventează stări ce nu există în kernel truth
- `host-runtime` care devine destinație finală de closure
- `boot-x86_64` care devine centru semantic pentru subsisteme ce aparțin kernelului
- monoliți care absorb familii din mai multe domenii fără ownership explicit

## Integration Rule By Layer

### Layer 0 -> Layer 1

Permis:

- facts
- handoff
- hardware activation results
- diagnostics

Interzis:

- policy finală de userland
- truth semantic pentru object model-ul kernelului

### Layer 1 -> Layer 2

Permis:

- ABI records
- syscall results
- introspection records
- snapshot records

Interzis:

- scurgere de detalii instabile fără record canonic

### Layer 2 -> Layer 3

Permis:

- semantic classification
- shell/runtime helpers
- session/bootstrap transport

Interzis:

- redefinirea kernel truth

## Dependency Map By Subsystem

### Scheduler

Allowed:

- `kernel-core -> user-abi -> user-runtime -> userland-native`
- `kernel-core -> procfs -> userland-native`
- `boot-x86_64 -> QEMU proof -> userland-native`

Forbidden:

- scheduler policy inventată în shell
- fairness truth definită numai în `nextmind`

### VFS

Allowed:

- `kernel-core -> user-abi -> user-runtime -> userland-native`
- `kernel-core -> procfs fd/fdinfo -> userland-native`

Forbidden:

- shadow VFS model în userland ca adevăr final

### VM

Allowed:

- `kernel-core -> vm decisions/episodes -> userland-native`

Forbidden:

- VM policy duplicată în control plane ca sursă de adevăr

### Networking

Allowed:

- `platform-x86_64 -> kernel-core`
- `kernel-core -> user-runtime -> userland-native`

Forbidden:

- networking truth derivat exclusiv din shell smoke

## Ownership Escalation Rule

Când un subsistem crește:

- owner-ul semantic rămâne același
- surface-urile auxiliare pot crește numai ca observatori sau operatori
- dacă o suprafață superioară începe să dețină autoritatea semantică reală, arhitectura este în abatere

## Use Rule

Acest document trebuie consultat când:

- se adaugă un subsistem nou
- se face refactor cross-crate
- apare tentația de a muta logică într-un orchestrator mare
- nu este clar cine este owner-ul adevărului pentru o familie de comportament
