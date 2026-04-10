# NGOS Canonical State Machines

## Authority

Acest document fixează state machines canonice pentru subsistemele critice.

El nu descrie toate detaliile interne ale fiecărui tip, dar fixează forma de adevăr pentru:

- stări
- tranziții valide
- tranziții interzise
- observabilitate minimă

## 1. Process State Machine

### Canonical States

- `Created`
- `Ready`
- `Running`
- `Blocked`
- `Exited`

### Valid Transitions

- `Created -> Ready`
- `Created -> Blocked` only if explicit runtime rule allows pre-ready blocking
- `Ready -> Running`
- `Running -> Ready`
- `Running -> Blocked`
- `Running -> Exited`
- `Blocked -> Ready`
- `Ready -> Exited` only if explicit forced termination path exists

### Invalid Transitions

- `Exited -> *`
- `Blocked -> Running` without scheduler mediation
- `Created -> Running` without scheduler-ready path

### Required Observability

- `inspect_process`
- procfs status/stat
- refusal on invalid transition

## 2. Scheduler State Machine

### Canonical State Families

- queued membership
- running slot
- urgent status
- lag/debt
- dispatch accounting
- runtime accounting

### Valid Transitions

- queued -> running
- running -> queued
- running -> blocked
- blocked -> queued via wake/resume
- queued class move via rebind
- urgent queue insertion on wake

### Invalid Conditions

- same thread duplicated in scheduler queues
- wait ticks on empty class
- lag debt on empty class
- runtime ticks without dispatch
- runtime ticks exceeding dispatch budget envelope

### Required Observability

- `/proc/system/scheduler`
- `/proc/system/schedulerepisodes`
- verified-core scheduler family

## 3. VFS Node State Machine

### Canonical Node Families

- directory
- file
- symlink
- mount-visible node

### Valid Transitions

- absent -> created
- created -> linked/visible
- visible -> renamed
- visible -> unlinked
- visible -> opened by descriptor
- opened -> descriptor close
- symlink -> readlink resolution

### Invalid Conditions

- duplicate path
- duplicate inode
- invalid normalized path
- missing parent directory
- invalid symlink target

### Required Observability

- VFS inspection
- descriptor inspection
- refusal markers for invalid path / exists / not found / directory not empty / cross-mount rename

## 4. VM State Machine

### Canonical VM Families

- mapping lifecycle
- permission lifecycle
- fault lifecycle
- quarantine lifecycle
- COW lifecycle
- file-backed lifecycle

### Valid Transitions

- unmapped -> mapped
- mapped -> protected
- mapped -> unmapped
- mapped -> faulted -> resolved/refused
- object -> shadow/COW
- quarantined -> released/reclaimed

### Invalid Conditions

- protect/unmap on invalid region
- write through read-only without refusal/fault model
- hidden COW transitions without evidence

### Required Observability

- maps
- vmobjects
- vmdecisions
- vmepisodes

## 5. CPU Extended State Lifecycle

### Canonical Families

- detection
- policy
- activation
- probe
- handoff
- per-thread ownership
- save
- restore
- release

### Valid Transitions

- feature absent -> refusal
- feature present -> activation
- activation -> probe
- probe -> runtime handoff
- thread unowned -> owned
- owned -> saved
- saved -> restored
- owned -> released

### Invalid Conditions

- runtime use without activation
- save area metadata without backing state
- restored state without prior ownership

### Required Observability

- CPU runtime status
- procfs CPU views
- verified-core CPU family

## 6. Event Queue State Machine

### Canonical States

- created
- empty
- pending
- waited
- drained
- removed

### Valid Transitions

- created -> pending
- pending -> drained
- drained -> pending
- created/pending/drained -> removed

### Invalid Conditions

- wait on invalid queue
- read after removal without refusal

### Required Observability

- queue inspection
- refusal path
- final queue state

## 7. Device Request State Machine

### Canonical States

- registered
- queued
- in-flight
- completed
- failed
- canceled

### Valid Transitions

- registered -> queued
- queued -> in-flight
- in-flight -> completed
- in-flight -> failed
- queued/in-flight -> canceled

### Invalid Conditions

- completed -> in-flight
- failed -> completed without explicit retry object

### Required Observability

- device inspect
- driver inspect
- request lifecycle evidence

## 8. Network Socket State Machine

### Canonical States

- created
- bound
- connected or session-active
- rx-ready
- tx-drained
- closed

### Valid Transitions

- created -> bound
- bound -> active
- active -> rx-ready
- active -> tx-drained
- active -> closed

### Invalid Conditions

- I/O on unbound invalid path without refusal
- hidden drop/backpressure without counters

### Required Observability

- socket inspection
- interface inspection
- readiness/watch evidence
- drop counters

## Use Rule

Când un subsistem este declarat închis, dovada trebuie să fie compatibilă cu state machine-ul lui canonic.

Dacă implementarea nu poate fi descrisă coerent prin aceste state și tranziții, subsistemul este incomplet sau modelul trebuie extins explicit.
