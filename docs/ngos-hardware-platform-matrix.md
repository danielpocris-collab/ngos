# NGOS Hardware And Platform Matrix

## Authority

Acest document fixează:

- platformele și suprafețele de execuție recunoscute
- rolul fiecărei suprafețe
- nivelul de adevăr acceptat
- ordinea de maturizare hardware/platform

## Platform Truth Levels

### Level P0: Synthetic / Auxiliary

Exemple:

- `host-runtime`
- `platform-host-runtime`

Rol:

- dezvoltare locală
- accelerare
- izolare semantică

Nu sunt truth surface de produs.

### Level P1: Real-System Emulated

Exemplu:

- `QEMU` pe path-ul:
  - `boot-x86_64`
  - `platform-x86_64`
  - `kernel-core`
  - `user-runtime`
  - `userland-native`

Rol:

- primul truth surface acceptat pentru closure

### Level P2: Physical Hardware

Exemplu:

- x86_64 hardware fizic

Rol:

- confirmare finală pentru fronturile relevante
- validare hardware-specifică

## Recognized Platform Matrix

| Platform Surface | Status In Product Model | Allowed Use | Not Allowed As |
| --- | --- | --- | --- |
| `host-runtime` | auxiliary | acceleration, tests, local proofs | final closure target |
| `platform-host-runtime` | auxiliary | host backend | product truth |
| `QEMU + boot-x86_64 + platform-x86_64` | canonical truth surface | real subsystem closure | mere smoke-only curiosity |
| physical x86_64 hardware | final physical truth | final confirmation and platform maturity | optional replacement for missing QEMU closure |

## Hardware Scope

În forma actuală a produsului, hardware scope-ul canonic este:

- `x86_64`

Active path:

- `boot-x86_64`
- `platform-x86_64`

## Non-Active Hardware Scope

Următoarele nu sunt scope activ de produs în acest moment:

- ARM / AArch64
- Apple-specific hardware paths
- non-x86 product closure

Ele pot intra ulterior doar prin decizie explicită și nu sunt implicite în closure-ul curent.

## Platform Responsibility Matrix

| Concern | Boot | Platform | Kernel | User Runtime | Userland |
| --- | --- | --- | --- | --- | --- |
| CPU feature bring-up | primary | secondary | consume and enforce | consume | observe |
| paging/address-space mechanism | no final ownership | primary | consume and model | no | no |
| interrupt/APIC mechanism | bootstrap | primary | consume and orchestrate | no | observe |
| device transport mechanics | no final ownership | primary | consume and model | consume | operate/inspect |
| object model | no | no | primary | consume | consume |
| diagnostics | primary | secondary | secondary | consume | consume |
| subsystem observability | limited | limited | primary | secondary | secondary |

## Closure Matrix By Platform Level

### Boot and Diagnostics

- P0: useful but not sufficient
- P1: mandatory
- P2: optional unless hardware is explicitly in scope

### CPU / Runtime Bring-Up

- P0: useful
- P1: mandatory
- P2: strongly preferred for mature hardware claims

### Scheduler

- P0: useful
- P1: mandatory
- P2: optional unless hardware-specific scheduling claims are made

### VFS / VM / Syscall / Observability

- P0: useful
- P1: mandatory
- P2: optional unless hardware behavior materially changes the subsystem

### Device Runtime / Networking

- P0: useful
- P1: mandatory
- P2: strongly preferred because hardware behavior matters directly

## QEMU Rule

`QEMU` nu este doar un emulator de conveniență.

În modelul `ngos`, el este:

- primul full-system truth surface acceptat
- prima destinație reală de closure pentru subsistemele strategice

Regulă:

dacă un subsistem strategic nu are dovadă pe `QEMU`, nu este închis global.

## Hardware Rule

Hardware fizic nu înlocuiește `QEMU`.

Ordinea corectă este:

1. path real implementat
2. `QEMU`
3. hardware fizic

Fără pasul 2, pasul 3 nu este o bază bună de closure pentru repo.

## Hardware Claims Rule

Nu este permis să se afirme:

- support complet pentru generații actuale
- support pentru generații viitoare
- readiness complet hardware

fără:

- detection
- policy
- activation/refusal
- observabilitate
- dovadă pe path-ul real

## Current Canonical Hardware Priorities

Prioritățile canonice actuale sunt:

1. x86_64 bring-up and stability
2. CPU feature policy and lifecycle
3. scheduler/device/networking closure pe `QEMU`
4. physical x86_64 confirmation pentru fronturile care au ajuns suficient de mature

## Use Rule

Acest document trebuie consultat când:

- se decide unde se implementează prima dată un front strategic
- se evaluează dacă un subsistem este „real” sau doar auxiliar
- se face o afirmație despre readiness hardware/platform
