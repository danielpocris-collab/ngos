# NGOS Execution And Evidence Matrix

## Authority

Acest document definește, pentru subsistemele strategice din `ngos`, unde trebuie să existe execuție, ce dovadă este acceptată și ce artefact trebuie să rămână observabil.

El nu descrie intenții.
Descrie forma acceptată de dovadă.

## Evidence Classes

### Class A: Kernel Truth

Dovadă internă în:

- runtime state
- verified core
- procfs
- inspect surfaces

### Class B: User Truth

Dovadă în:

- `user-runtime`
- `userland-native`
- shell/control output

### Class C: Full-System Truth

Dovadă în:

- `QEMU`
- boot diagnostics
- serial log
- proof scripts

### Class D: Hardware Truth

Dovadă în:

- execuție pe hardware fizic
- logs sau capturi observabile

## Acceptance Levels

### Level 0

- cod existent
- fără dovadă reală

### Level 1

- dovadă locală în crate/test

### Level 2

- dovadă pe path intern de sistem
- observabilitate reală

### Level 3

- dovadă pe `QEMU`

### Level 4

- dovadă pe hardware fizic

Regulă:

pentru subsistemele strategice, closure normală minimă este Level 3.

## Matrix

| Subsystem | Kernel Truth | User Truth | QEMU Truth | Hardware Truth | Required Artifact |
| --- | --- | --- | --- | --- | --- |
| Boot and diagnostics | mandatory | optional | mandatory | optional unless explicitly in scope | boot locator + serial markers |
| CPU/runtime bring-up | mandatory | optional | mandatory | optional unless explicitly in scope | CPU diagnostics + runtime status |
| Process model | mandatory | mandatory | mandatory | optional unless explicitly in scope | inspect/process + procfs + proof |
| Scheduler | mandatory | mandatory | mandatory | optional unless explicitly in scope | `/proc/system/scheduler` + proof log |
| Capability model | mandatory | optional | preferred | optional | verified core + inspect surfaces |
| Domain/resource/contract | mandatory | mandatory | preferred | optional | inspect surfaces + proof |
| VFS | mandatory | mandatory | mandatory | optional unless explicitly in scope | procfs/fd + vfs proof |
| VM | mandatory | mandatory | mandatory | preferred when strategically requested | vmepisodes + proof log |
| Eventing and waits | mandatory | mandatory | preferred | optional | queue inspection + wake proofs |
| Signal runtime | mandatory | preferred | preferred | optional | signal inspection |
| Device runtime | mandatory | mandatory | mandatory | preferred | device/driver inspect + proof |
| Networking | mandatory | mandatory | mandatory | preferred | socket/interface inspection + proof |
| Syscall surface | mandatory | mandatory | indirect through system proofs | optional | dispatch trace + ABI records |
| Observability / procfs | mandatory | mandatory | mandatory when subsystem is strategic | optional | procfs routes |
| User ABI | mandatory | mandatory | indirect | optional | ABI tests + runtime transport |
| User runtime | optional as source, mandatory as consumer | mandatory | indirect | optional | semantic extraction |
| Native control/userland | optional as source, mandatory as surface | mandatory | mandatory for proofs | optional | shell proof output |

## Required Artifacts By Subsystem

### Boot and Diagnostics

- boot locator markers
- serial markers
- diagnostics report

### CPU / Runtime Bring-Up

- CPU runtime status
- boot diagnostics
- observable feature activation/refusal

### Scheduler

- `/proc/system/scheduler`
- `/proc/system/schedulerepisodes`
- scheduler proof log on `QEMU`
- verified-core scheduler invariants

### VFS

- `/proc/<pid>/fd`
- `/proc/<pid>/fdinfo`
- VFS proof log
- refusal markers

### VM

- `/proc/<pid>/maps`
- `/proc/<pid>/vmobjects`
- `/proc/<pid>/vmdecisions`
- `/proc/<pid>/vmepisodes`
- VM proof log

### Networking

- `/proc/system/network/interfaces`
- `/proc/system/network/sockets`
- event/watch proofs
- drop/pressure evidence

### Device Runtime

- inspect device
- inspect driver
- request lifecycle evidence

## Non-Accepted Evidence

Următoarele nu sunt suficiente ca dovadă finală pentru subsistemele strategice:

- `cargo check` singur
- unit tests singure fără integrare
- host-runtime only behavior
- synthetic log fără execuție reală
- debug print fără suprafață de observabilitate stabilă

## Stop Rule

Un subsistem strategic nu poate fi declarat închis dacă nu există:

- evidență Class A
- evidență Class B unde subsistemul atinge user/control
- evidență Class C pe `QEMU`

Class D este obligatorie numai când hardware fizic este explicit în scope sau când documentele de subsistem cer deja asta.
