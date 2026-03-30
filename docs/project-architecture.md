# Next Gen OS Project Architecture

## Scope

Acest document descrie proiectul real din workspace-ul `ngos`, nu un design ipotetic.
Acoperă:

- forma actuală a workspace-ului
- rolul fiecărui crate principal
- modelul de kernel existent
- subsistemele majore
- suprafața de runtime și host runtime
- direcția de refactor spre un model semantic mai explicit
- transformările deja validate în cod

Documentul este intenționat ca referință de arhitectură pentru dezvoltare internă.

## Active Development Direction

Direcția activă de dezvoltare pentru `ngos` este transformarea incrementală spre `nano-kernel`.

Asta înseamnă:

- kernelul existent rămâne baza
- subsistemele reale se adâncesc prin unități semantice mici
- dezvoltarea nouă nu trebuie să reîngroașe managerii sau modulele centrale
- userland-ul nativ trebuie și el dezvoltat modular, în slice-uri semantice, nu ca shell monolitic tot mai mare
- regula nu se oprește la kernel: userland-ul, shell-ul, aplicațiile și tooling-ul nou trebuie și ele construite nano-semantic
- o suprafață externă unificată este acceptată doar dacă implementarea internă rămâne împărțită în agenți/moduluri semantice mici
- `userland-native` poate rămâne suprafața principală de control atâta timp cât crește prin agenți semantici dedicați și nu prin recompactare internă

Forma externă poate rămâne unificată, dar dezvoltarea internă trebuie să reducă centralizarea și mutația implicită.

## Workspace

Workspace-ul conține următoarele crate-uri:

- `boot-x86_64`
- `host-runtime`
- `kernel-core`
- `ngos-core-util`
- `user-abi`
- `user-runtime`
- `userland-native`
- `platform-hal`
- `semantic-runtime`
- `platform-x86_64`
- `platform-host-runtime`
- `tooling/command-runner`

Metadate workspace:

- produs: `Next Gen OS`
- codename: `ngos`
- workspace name: `next-gen-os`
- edition: `2024`

## Crates principale

### `kernel-core`

`kernel-core` este centrul semantic al proiectului.
Conține:

- process model
- scheduler
- capability/object model
- native resource/contract/domain model
- VFS și descriptor lifecycle
- IO runtime
- event queues
- sleep queues
- signal runtime
- memory wait runtime
- syscall surface
- observability și introspection
- user launch și user syscall runtime

Acesta este locul unde identitatea internă `ngos` este definită cel mai clar.

### `platform-hal`

`platform-hal` definește contractele de platformă:

- address spaces
- page mappings
- arhitectură
- cache policy
- memory permissions

HAL-ul ține interfața de platformă separată de modelul de kernel.

### `platform-host-runtime`

Backend HAL pentru rularea pe host runtime.
Este infrastructura care permite validarea semnificației kernelului fără a depinde de boot hardware real.

### `platform-x86_64`

Backend de platformă pentru x86_64.
Ține părțile concrete dependente de arhitectură și platformă.

### `boot-x86_64`

Componenta de boot și diagnostics pentru x86_64.
Aici trăiesc și suprafețele Chronoscope folosite pentru diagnostic cauzal și postmortem.

### `host-runtime`

Punctul principal de rulare în workspace-ul activ.
Rulează kernelul și userland-ul prin backend-ul host runtime și produce rapoarte de sesiune.

### `user-abi`

Definește ABI-ul user/kernel și structurile transportate peste syscall surface.

### `user-runtime`

Runtime-ul comun pentru codul din user mode.

Acesta este și locul aprobat pentru execuție WebAssembly în `ngos`, împreună
cu suprafețele de extensie și aplicație aflate deasupra lui.

Politica normativă este definită în
[wasm-execution-policy.md](C:/Users/pocri/OneDrive/Desktop/experiment/docs/wasm-execution-policy.md).

### `userland-native`

Userland-ul principal din workspace.
Include shell, comenzi, control plane, procfs-style introspection și fluxuri reale de interacțiune cu kernelul.

`userland-native` poate include, în timp, aplicații sau componente Wasm
orchestrate prin `user-runtime`, dar nu mută Wasm sub granița kernelului.

### `semantic-runtime`

Strat semantic suplimentar pentru modele și fluxuri de execuție cu semnificație mai înaltă decât simple syscall-uri brute.

### `ngos-core-util`

Utilitare fundamentale reutilizabile:

- `BufRing`
- `TaskQueue`
- `SleepQueue`
- range utilities
- scatter/gather
- buffer abstractions

## Model de kernel curent

Kernelul curent nu este un clone Linux/Windows și nici un microkernel clasic.
Modelul intern este mai apropiat de o arhitectură capability-centric cu:

- obiecte tipate
- authority explicită
- resource/domain/contract model
- semnificație internă proprie

Unitățile conceptuale centrale sunt:

- `Process`
- `Thread`
- `Capability`
- `Domain`
- `Resource`
- `Contract`
- `Descriptor`
- `EventQueue`
- `SleepQueue`

## Subsisteme majore

### 1. Process și scheduler

Responsabilități:

- lifecycle de proces și thread
- planificare pe clase
- budget
- block / wake / exit / reap

Clasele de scheduler existente:

- `LatencyCritical`
- `Interactive`
- `BestEffort`
- `Background`

Scheduler-ul este un scheduler cu cozi pe clasă și budget per proces/thread principal.

### 2. Capability și object model

Responsabilități:

- handle space
- object table
- rights narrowing
- duplicate / restricted duplicate

Acesta este mecanismul de autoritate de bază.

### 3. Native model: domains, resources, contracts

Responsabilități:

- organizarea semantică a resurselor
- policy de contract
- issuer policy
- arbitration
- governance
- claim / queue / handoff / revoke / retire

Acesta este unul dintre cele mai distinctive subsisteme ale proiectului.

### 4. VFS și descriptor lifecycle

Responsabilități:

- VFS namespace
- create/open/stat/lstat/readlink/mount
- descriptor namespace
- dup / dup2-style remap
- cloexec / nonblock
- descriptor-bound queues

### 5. IO runtime

Responsabilități:

- `read_io`
- `write_io`
- vectored IO
- control ops
- readiness registration
- descriptor-bound endpoint state

### 6. Device runtime

Responsabilități:

- device/driver binding
- block devices
- sockets și networking
- driver completion flow
- endpoint IO state sync

### 7. Eventing și wait

Responsabilități:

- event queues
- timers
- process watches
- signal watches
- resource watches
- network watches
- memory wait watches
- sleep queues
- memory word wait/wake/requeue

### 8. VM și address spaces

Responsabilități:

- memory map
- VM objects
- file mappings
- copy-on-write
- region split/coalesce
- page state tracking
- shadow metadata

### 9. Signal runtime

Responsabilități:

- pending signals
- blocked masks
- wait for pending signal
- cross-effects cu memory waits și eventing

### 10. Observability

Responsabilități:

- `snapshot()`
- `inspect_process()`
- `inspect_system()`
- procfs-style rendering
- queue introspection
- IO/device metadata rendering

### 11. Syscall surface

Responsabilități:

- dispatch peste toate subsistemele
- user syscall runtime
- native model syscalls
- memory / descriptor / process / eventing / networking APIs

## Host runtime

`host-runtime` rulează fluxul real:

- construiește runtime-ul
- montează VFS-ul de bază
- creează device și driver nodes
- pornește `userland-native`
- execută shell-ul
- colectează stdout
- produce raport de sesiune

Raportul de sesiune curent include:

- sumar de proces/sesiune
- sumar Chronoscope
- sumar `resource-agents`
- stdout-ul sesiunii

## Chronoscope și diagnostics

Chronoscope trăiește în `boot-x86_64` și este folosit de `host-runtime` pentru:

- cauzalitate
- responsibility tracing
- replay status
- trust/completeness reporting
- propagation path

Chronoscope este mecanismul principal de adevăr pentru debugging cauzal.

## Direcție arhitecturală

Direcția validată în cod este refactor incremental spre o arhitectură internă mai explicită, nu rewrite complet.

Obiectivele sunt:

- separarea deciziei de mutația efectivă
- autoritate mai îngustă
- state transitions mai explicite
- observabilitate cauzală mai bună
- blast radius mai mic

Nu există un nou runtime per-agent și nu există un model de tip “thread per agent”.
Transformarea este semantică și internă, nu un schimb total de model de execuție.

## Transformări validate deja

### Resource lifecycle

În `kernel-core`, path-urile de resource lifecycle au fost refactorizate prin agenți semantici expliciți:

- `ClaimValidator`
- `CancelValidator`
- `ReleaseValidator`
- `ResourceStateTransitionAgent`
- `ContractStateTransitionAgent`

Sunt expuse și prin:

- jurnal intern de decizie
- `SystemIntrospection`
- raport operator în `host-runtime`

### Wait/wake și memory wait

Au fost introduse jurnale de decizie pentru:

- enqueue
- wake
- timeout wake
- cancel
- requeue
- memory wait block/wake/requeue

Acestea sunt expuse prin `inspect_system()`.

### Scheduler

Au fost introduse jurnale de decizie pentru:

- enqueue
- wake
- block
- tick / budget rotation
- rebind
- remove

Acestea sunt expuse prin `inspect_system()`.

### VFS/descriptor/io

Au fost introduse jurnale de decizie pentru:

- open
- duplicate
- duplicate-to
- close
- read
- write
- vectored write
- `fcntl`
- readiness registration

Acestea sunt expuse prin `inspect_system()`.

## Starea dovezii actuale

Până acum, modelul explicit a fost validat în patru zone grele:

- resource lifecycle
- wait/wake + memory wait
- scheduler
- VFS/descriptor/io

Acest lucru înseamnă că direcția nu mai este doar o ipoteză arhitecturală.
Există dovadă practică că modelul poate rămâne curat și în subsisteme cu:

- mutație internă reală
- wake/block/requeue
- scheduling decisions
- descriptor and IO lifecycle

## Avantaje ale direcției curente

- deciziile devin observabile
- mutațiile implicite se reduc
- debugging-ul cauzal devine mai bun
- responsabilitățile sunt mai clare
- testele pot verifica și decizia, nu doar rezultatul final
- blast radius-ul intern scade

## Dezavantaje și riscuri

- crește densitatea structurală a codului
- apare cost de întreținere suplimentar
- există risc de supra-instrumentare
- există risc de duplicare semantică între plan și execuție
- disciplina trebuie păstrată uniform între subsisteme

Dezavantajul principal nu este performanța brută, ci riscul de prea multă ceremonie dacă modelul este aplicat mecanic.

## Ce nu este încă dovedit complet

Direcția nu este încă validată la același nivel de adâncime pentru:

- VM / COW / page fault path
- toate path-urile device/network cele mai dense
- toate fluxurile de fault containment și quarantine

Acestea rămân fronturile următoare dacă scopul este o dovadă de arhitectură și mai puternică.

## Criteriu practic de succes

Transformarea este bună dacă:

- `cargo test --workspace` rămâne verde
- `cargo clippy --workspace --all-targets -- -D warnings` rămâne verde
- state transitions devin mai explicite
- deciziile devin observabile
- raportarea operatorului devine mai bună
- complexitatea nu explodează

Transformarea este rea dacă:

- introduce straturi fără valoare
- dublează logică fără câștig
- produce zgomot de diagnostics
- face codul mai greu de urmărit decât înainte

## Rezumat

`ngos` este deja un OS real în dezvoltare, cu kernel, ABI, runtime, userland și diagnostics proprii.
Forma lui actuală este suficient de matură încât refactorizarea arhitecturală incrementală are sens.

Direcția cea mai promițătoare validată până acum este:

- păstrarea kernelului existent
- refactor semantic incremental
- autoritate mai îngustă
- observabilitate mai puternică
- state transitions mai explicite

Practic, proiectul se prezintă acum ca un kernel capability-centric și semantics-first, aflat în tranziție controlată spre o arhitectură internă mai explicită și mai ușor de diagnosticat.
