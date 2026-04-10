# NGOS Interface Contract Catalog

## Authority

Acest document cataloghează suprafețele canonice prin care subsistemele `ngos` comunică, expun stare sau transferă autoritate.

Nu descrie toate funcțiile individuale.
Descrie familiile de contracte care sunt autorizate și strategice.

## Contract Classes

### Class 1: Boot Contracts

Transport între:

- `boot-x86_64`
- `kernel-core`
- `user-runtime`
- `userland-native`

Exemple:

- facts de boot
- CPU handoff
- bootstrap env pentru primul proces
- boot diagnostics

### Class 2: Kernel Object Contracts

Transport între:

- `kernel-core`
- syscall surface
- inspect/procfs surfaces

Exemple:

- process/thread state
- capability/resource/contract records
- VFS object state
- VM object state

### Class 3: Runtime Snapshot Contracts

Transport între:

- `kernel-core`
- `user-abi`
- `user-runtime`
- `userland-native`

Exemple:

- `NativeSystemSnapshotRecord`
- pressure/fairness/verified-core transport

### Class 4: Inspection Contracts

Suprafețe de observabilitate:

- `inspect_system`
- `inspect_process`
- procfs routes
- queue inspection
- device/network inspection

### Class 5: Control Contracts

Suprafețe de operare:

- syscall surface
- shell process actions
- semantic control helpers

## Canonical Contract Families

## 1. Bootstrap Contract

### Purpose

- pornește primul proces user
- transferă facts și policy necesare bootstrap-ului

### Carried Truth

- boot mode / proof mode
- CPU context relevant
- bootstrap/session context

### Owners

- producer: `boot-x86_64`
- parser/transport: `user-abi`
- consumer: `user-runtime`, `userland-native`

## 2. System Snapshot Contract

### Purpose

- transportă starea canonică a sistemului către runtime și userland

### Canonical Record

- `NativeSystemSnapshotRecord`

### Carried Truth

- counts de procese și cozi
- verified core
- scheduler urgency / starvation / lag / service
- socket / event pressure
- network drop counters

### Owners

- producer: `kernel-core`
- transport: `user-abi`
- consumer: `user-runtime`, `userland-native`

## 3. Process Inspection Contract

### Purpose

- expune modelul proces/thread pentru control și introspecție

### Carried Truth

- process state
- thread state
- scheduler policy
- CPU extended state profile
- address space and descriptor visibility

### Owners

- producer: `kernel-core`
- consumer: `user-runtime`, `userland-native`

## 4. Procfs Contract

### Purpose

- expune stare cauzală și finală prin rute stabile

### Canonical Families

- `/proc/system/scheduler`
- `/proc/system/schedulerepisodes`
- `/proc/system/cpu`
- `/proc/system/verified-core`
- `/proc/system/network/*`
- `/proc/<pid>/status`
- `/proc/<pid>/stat`
- `/proc/<pid>/fd`
- `/proc/<pid>/fdinfo`
- `/proc/<pid>/maps`
- `/proc/<pid>/vmobjects`
- `/proc/<pid>/vmdecisions`
- `/proc/<pid>/vmepisodes`
- `/proc/<pid>/cpu`

### Owners

- producer: `kernel-core`
- consumer: `userland-native`, proofs, operator

## 5. Syscall Contract

### Purpose

- execuție controlată de operații reale asupra sistemului

### Canonical Families

- process control
- VFS operations
- VM operations
- queue/event operations
- device/network operations
- inspection syscalls

### Owners

- producer: `kernel-core`
- transport: `user-abi`
- consumer: `user-runtime`

## 6. Verified Core Contract

### Purpose

- face vizibil nucleul tare și violările lui

### Carried Truth

- capability model verified
- VFS invariants verified
- scheduler state machine verified
- CPU extended state lifecycle verified
- violation list

### Surfaces

- runtime report
- procfs
- system snapshot summary
- semantic runtime / nextmind consumption

### Owners

- producer: `kernel-core`
- consumer: `user-runtime`, `userland-native`

## 7. Scheduler Fairness Contract

### Purpose

- expune starea reală de fairness a schedulerului

### Carried Truth

- urgent queue counts
- starved flags
- lag debt
- dispatch counts
- runtime ticks
- fairness imbalance

### Surfaces

- procfs scheduler
- system snapshot
- semantic runtime metrics
- nextmind output

### Owners

- producer: `kernel-core`
- transport: `user-abi`
- consumer: `user-runtime`, `userland-native`

## 8. CPU Extended State Contract

### Purpose

- transferă și observă lifecycle-ul CPU extended state

### Carried Truth

- xsave managed
- save area bytes
- xcr0 mask
- boot probe state
- handoff state
- per-thread ownership

### Surfaces

- boot CPU runtime status
- runtime policy handoff
- procfs cpu
- thread inspection

### Owners

- producer: `boot-x86_64` + `kernel-core`
- transport: `user-abi` unde este relevant
- consumer: `user-runtime`, `userland-native`

## 9. Device / Driver Inspection Contract

### Purpose

- expune lifecycle-ul real al device runtime

### Carried Truth

- device state
- driver state
- request lifecycle
- evidence / counters

### Owners

- producer: `kernel-core`
- consumer: `userland-native`

## 10. Network Inspection Contract

### Purpose

- expune starea reală a networking-ului

### Carried Truth

- sockets
- interfaces
- queue depth
- drops
- readiness/watch behavior

### Owners

- producer: `kernel-core`
- consumer: `user-runtime`, `userland-native`

## Contract Integrity Rule

Un contract canonic trebuie să aibă:

- producer clar
- consumer clar
- record sau suprafață stabilă
- adevăr real, nu simbolic
- test sau proof real

## Forbidden Contract Shapes

Următoarele sunt interzise:

- transport necanonic prin string-uri ad-hoc când există record stabil
- ascunderea stării critice în `reserved` fields fără helperi canonici
- drift între producer și consumer
- redefinirea adevărului într-un layer superior

## Use Rule

Acest document trebuie folosit când:

- se adaugă câmpuri noi în ABI
- se creează o suprafață nouă de inspectare
- se mută un subsistem pe truth path
- apare întrebarea „care este contractul canonic pentru frontul ăsta?”
