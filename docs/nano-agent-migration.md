# Nano-Agent Migration Plan

## Purpose

Acest document definește planul de transformare incrementală a kernelului `ngos` spre o arhitectură internă mai explicită, bazată pe agenți semantici mici.

Nu descrie un rewrite.
Nu descrie un kernel nou.
Descrie modul în care kernelul existent este împărțit gradual în unități de decizie mai mici, cu autoritate mai îngustă și observabilitate mai bună.

## Active Development Rule

Migrarea spre modelul nano-agent nu este doar un plan viitor.
Ea este regula activă de dezvoltare pentru subsistemele atinse de lucru nou.

Din acest punct:

- extensiile de kernel se dezvoltă ca slice-uri nano, nu ca blocuri monolitice noi
- extinderile de userland care reflectă capabilități de kernel trebuie și ele împărțite în unități semantice mici
- orice subsistem nou trebuie să se miște spre autoritate mai îngustă, nu spre manageri mai mari
- dacă apare conflict între viteză locală și forma nano-agent, se păstrează direcția nano-agent
- regula se aplică întregului produs: kernel, userland, shell, aplicații, utilitare și tooling intern
- nu există excepție de tip "în shell putem compacta totul"; shell-ul și userland-ul trebuie să rămână compuse din agenți/moduluri semantice mici
- suprafețele mari și unitare sunt permise numai ca orchestratori semantici, nu ca blocuri cu mutație și responsabilitate nelimitată

## Execution Contract

În `ngos`, execuția validă este sistemică și end-to-end.

### 1. Interzis: micro-progres

Nu sunt livrări valide:

- "am adăugat structuri"
- "am pus hook-uri"
- "am pregătit baza"
- "urmează să implementez"
- "pot continua cu"

Orice astfel de stare este incompletă și nu este considerată progres acceptat de proiect.

### 2. Obligatoriu: front complet

Orice front început trebuie împins până la un rezultat observabil, executabil și verificabil cap-coadă.

Un front este considerat valid numai dacă include toate acestea:

- logică reală, nu stub-uri
- integrare în sistemul existent
- efect vizibil în runtime
- observability sau introspecție
- expunere în interfețe relevante ale path-ului real: CLI, syscall surface sau API intern real
- test sau demonstrație reală

Dacă lipsește una dintre aceste condiții, frontul nu este `done`.

### 3. Definiția globală de `done`

Un subsistem este considerat `done` numai dacă:

1. produce comportament real în runtime
2. este integrat în fluxurile existente
3. poate fi observat
4. poate fi explicat cauzal
5. poate fi testat sau demonstrat cap-coadă

Fără toate aceste condiții, nu este considerat implementat.

Validarea pentru `done` nu este satisfăcută de o demonstrație doar pe happy path.
Pentru orice front declarat închis, validarea trebuie să includă și:

1. path de succes
2. path de blocare, refuz sau eroare, dacă subsistemul poate respinge operația
3. reversibilitate sau recovery, dacă subsistemul permite restaurare, release sau rollback
4. expunerea observabilă a stării finale după închiderea fluxului

Dacă există doar demo pozitiv, frontul este doar parțial închis.

### 3A. Clauza de scope: fără reducere abuzivă a obiectivului

Când se cere închiderea unui subsistem sau a unei familii mari, termenul folosit este autoritar și nu poate fi redus unilateral.

Nu este permisă reformularea în:

- `frontul lucrat în acest ciclu`
- `sub-frontul curent`
- `calea urmărită aici`
- orice altă formulare care îngustează scope-ul cerut

Exemple:

- `închide VM` înseamnă închide subsistemul VM ca ansamblu, nu doar `map/unmap/quarantine`
- `închide networking` înseamnă închide networking ca subsistem, nu doar un path de socket
- `închide VFS` înseamnă închide VFS cap-coadă, nu doar `lookup/open`

Este interzisă declararea `done` dacă au rămas familii relevante ale aceluiași subsistem neînchise end-to-end.

### 3B. Regula de completitudine pe subsistem

Un subsistem este considerat închis numai dacă toate familiile lui relevante au fost fie:

1. implementate și validate cap-coadă, fie
2. enumerate explicit ca `out-of-scope` de utilizator înainte de execuție

Dacă utilizatorul nu a exclus explicit ceva, acel lucru rămâne în scope.

### 3C. Interzis: `done` local prezentat ca `done` global

Nu este permisă prezentarea formulărilor:

- `frontul lucrat acum este închis`
- `nu mai există gap în acest flux`
- `calea aceasta este completă`

ca substitut pentru cerința globală de a închide subsistemul întreg.

Dacă subsistemul mare nu este încă complet, formularea corectă este numai:

- `Subsistemul <nume> NU este încă închis.`
- `Am închis sub-frontul <x> din subsistemul <y>.`

Orice altă formulare este invalidă.

### 3D. Obligație de continuare

Dacă după o livrare mai există familii relevante din același subsistem:

- nu se oprește execuția
- nu se reclasifică acele familii ca `alte fronturi` doar pentru a închide conversația
- se continuă până la închiderea subsistemului cerut

Oprirea este validă numai dacă:

- subsistemul este închis real cap-coadă, sau
- există un blocker concret și demonstrat care face imposibilă continuarea în acel moment

`Am închis ce am urmărit aici` nu este motiv valid de oprire.

### 3E. Format obligatoriu când subsistemul nu este încă închis

Dacă subsistemul cerut nu este complet, răspunsul trebuie să înceapă explicit cu:

`Subsistemul <nume> NU este încă închis.`

Apoi trebuie enumerate exact:

- ce familii sunt închise
- ce familii mai sunt deschise
- ce a fost implementat acum

și execuția trebuie să continue pe familiile rămase.

### 3F. Clauza anti-oprire prematură

Când cerința este de forma `nu te opri până nu închizi X`, atunci:

- fiecare răspuns intermediar este doar progres parțial
- niciun răspuns intermediar nu are voie să conțină concluzii precum:
  - `front închis`
  - `nu mai există gap`
  - `este complet`
  - sau echivalente

decât dacă `X` este într-adevăr închis complet.

Pentru o cerință precum `închide VM`, sunt interzise formulări ca:

- `frontul VM lucrat aici este închis`
- `nu mai există gap-ul urmărit`
- `ce a rămas sunt alte fronturi VM`

cât timp subsistemul VM nu este închis cap-coadă.

### 3G. Hard Scope Law

Când cerința este `închide X`, `X` înseamnă subsistemul complet, nu sub-frontul ales local de implementare.

Este interzis:

- să reduci scope-ul la `frontul lucrat acum`
- să declari `done` local ca substitut pentru `done` global
- să te oprești cât timp mai există familii relevante din același subsistem
- să reclasifici restul ca `alte fronturi` doar ca să justifici oprirea

Regula absolută este:

Dacă mai există familii relevante neînchise din `X`, atunci singura formulare corectă este:

- `Subsistemul X NU este încă închis.`

Apoi trebuie:

- să enumeri ce este închis
- să enumeri ce este încă deschis
- să continui execuția pe ce a rămas

Până la închiderea completă a lui `X`, sunt interzise expresii precum:

- `front închis`
- `nu mai există gap`
- `ce a rămas sunt alte fronturi`
- `am închis ce am urmărit aici`

### 3H. Completitudinea prematură înseamnă execuție eșuată

Orice răspuns care declară completitudine înainte de închiderea întregului subsistem cerut este invalid și trebuie tratat ca execuție eșuată.

Regula rămâne valabilă chiar dacă modificările tehnice făcute până în acel punct sunt bune în sine.

Progresul tehnic poate fi real, dar execuția este eșuată dacă este prezentat ca închidere de subsistem înainte de închiderea completă a subsistemului.

### 4. Fără fragmentare artificială

Nu se sparge munca în pași mici doar pentru raportare.

Gruparea trebuie făcută:

- pe subsisteme complete
- pe fluxuri complete
- pe capabilități reale

Se preferă o singură livrare mare și coerentă în locul unui șir de livrări mici fără închidere de front.

### 5. Decizii autonome

Implementarea nu se blochează pe confirmări de detaliu.

Agentul sau contributorul responsabil trebuie să:

- aleagă ordinea implementării
- facă refactor dacă este necesar
- conecteze subsistemele între ele
- rezolve inconsistențele locale

Blocarea este acceptată numai în caz de conflict logic major, nu pentru pași de execuție obișnuiți.

### 6. Fără stub-uri dacă există cale reală

Nu se introduc variante false, simbolice sau placeholder dacă există o cale rezonabilă spre implementare reală.

Dacă se alege o simplificare, aceasta trebuie:

- să fie funcțională
- să producă efect real

### 7. Raportare strictă

Raportarea trebuie să descrie numai:

- frontul închis
- modificările concrete
- comportamentul nou
- execuția end-to-end
- verificarea reală
- gap-urile reale

Nu este acceptată raportarea bazată pe promisiuni, schițe sau "next steps".

## Definition

În contextul `ngos`, un nano-agent este:

- o unitate semantică mică
- cu o singură responsabilitate
- cu trigger explicit
- cu autoritate minimă
- cu ieșire explicită
- cu observabilitate explicită

Un nano-agent nu este:

- un proces nou
- un thread nou
- un serviciu distribuit
- un nou runtime paralel

## Ground Rules

Migrarea este validă numai dacă respectă toate regulile de mai jos:

- kernelul existent rămâne baza
- nu se schimbă ABI-ul fără motiv clar
- nu se introduce un nou model global de scheduling per agent
- fiecare pas trebuie să păstreze sau să întărească comportamentul curent
- fiecare pas trebuie să fie testabil în crate-ul real afectat
- fiecare pas trebuie să fie observabil prin introspecție sau raport operator
- niciun pas nu este acceptat dacă rupe `cargo test --workspace`
- niciun pas nu este acceptat dacă rupe `cargo clippy --workspace --all-targets -- -D warnings`

## Minimal Substrate

Există componente care trebuie să rămână substrate comun sub agenții semantici:

- object/capability substrate
- handle space
- process și thread identity
- scheduler runtime real
- event routing substrate
- VFS object storage
- VM object storage
- descriptor namespace storage
- `TaskQueue`, `BufRing`, `SleepQueue`
- tick și epoch/timestamp infrastructure
- `SystemIntrospection`

Acestea nu trebuie rescrise ca swarm de agenți.
Ele rămân infrastructura stabilă pe care agenții operează.

## Target Shape

Forma țintă a kernelului nu este “totul devine agent”.
Forma țintă este:

- substrate stabil dedesubt
- decizii semantice mici deasupra
- mutații controlate după decizie
- diagnostic cauzal mai clar

Textual:

```text
syscall / runtime request
  -> semantic decision unit
  -> explicit transition / action plan
  -> bounded mutation on substrate
  -> explicit observability record
  -> downstream wake / event / report
```

## Migration Pattern

Modelul standard de migrare folosit în `ngos` este:

1. identifică path-ul mare și amestecat
2. extrage decizia în helper semantic explicit
3. păstrează mutația în path-ul existent
4. adaugă jurnal intern de decizie
5. exportă jurnalul prin `inspect_system()`
6. unde e util, proiectează jurnalul în suprafețe auxiliare fără a schimba truth surface-ul real
7. extinde testele reale, nu doar teste sintetice

Acesta este modelul deja validat în cod.

## Validated Slices

### 1. Resource lifecycle

Validat în:

- [`kernel-core/src/runtime_core/native_model.rs`](C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/runtime_core/native_model.rs)
- [`kernel-core/src/tests/native_model.rs`](C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/tests/native_model.rs)
- [`host-runtime/src/report.rs`](C:/Users/pocri/OneDrive/Desktop/experiment/host-runtime/src/report.rs)
- [`host-runtime/src/session.rs`](C:/Users/pocri/OneDrive/Desktop/experiment/host-runtime/src/session.rs)

Agenți introduși:

- `ClaimValidator`
- `CancelValidator`
- `ReleaseValidator`
- `ResourceStateTransitionAgent`
- `ContractStateTransitionAgent`

Suprafață observabilă:

- `resource_agent_decisions`
- `SystemIntrospection`
- proiecții auxiliare de raportare, dacă există, rămân subordinate path-ului real

### 2. Wait/wake și memory wait

Validat în:

- [`kernel-core/src/sleep_queue_runtime.rs`](C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/sleep_queue_runtime.rs)
- [`kernel-core/src/sleep_queue_runtime/wait_ops.rs`](C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/sleep_queue_runtime/wait_ops.rs)
- [`kernel-core/src/memory_wait_runtime/ops.rs`](C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/memory_wait_runtime/ops.rs)
- [`kernel-core/src/runtime_core/eventing.rs`](C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/runtime_core/eventing.rs)

Agenți semantici observați:

- `SleepEnqueueAgent`
- `SleepWakeAgent`
- `SleepCancelAgent`
- `SleepRequeueAgent`
- `MemoryWaitAgent`

Suprafață observabilă:

- `wait_agent_decisions`
- `SystemIntrospection`

### 3. Scheduler

Validat în:

- [`kernel-core/src/scheduler.rs`](C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/scheduler.rs)
- [`kernel-core/src/tests/foundation.rs`](C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/tests/foundation.rs)
- [`kernel-core/src/tests/runtime_process.rs`](C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/tests/runtime_process.rs)

Agenți semantici observați:

- `EnqueueAgent`
- `WakeAgent`
- `BlockAgent`
- `TickAgent`
- `RebindAgent`
- `RemoveAgent`

Suprafață observabilă:

- `scheduler_agent_decisions`
- `SystemIntrospection`

### 4. VFS / descriptor / io

Validat în:

- [`kernel-core/src/descriptor_runtime.rs`](C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/descriptor_runtime.rs)
- [`kernel-core/src/descriptor_io_runtime/ops.rs`](C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/descriptor_io_runtime/ops.rs)
- [`kernel-core/src/descriptor_io_runtime/readiness.rs`](C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/descriptor_io_runtime/readiness.rs)
- [`kernel-core/src/tests/vfs_io.rs`](C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/tests/vfs_io.rs)

Agenți semantici observați:

- `OpenPathAgent`
- `DuplicateDescriptorAgent`
- `CloseDescriptorAgent`
- `ReadAgent`
- `WriteAgent`
- `FcntlAgent`
- `ReadinessAgent`

Suprafață observabilă:

- `io_agent_decisions`
- `SystemIntrospection`

## What Is Already Proven

Este deja demonstrat în cod că modelul rămâne curat în:

- resource ownership și contract lifecycle
- wait/wake și requeue logic
- scheduler rotation și wake/block
- descriptor/io lifecycle și readiness

Acesta este un prag important.
Modelul nu mai este doar o teză arhitecturală.

## What Is Not Yet Proven Enough

Fronturile următoare cu risc și valoare mare sunt:

- VM / COW / fault path
- deeper device/network data paths
- quarantine / fault containment
- poate unele zone de syscall orchestration foarte late

## Recommended Migration Order

Ordinea recomandată de acum înainte:

1. `VM / COW / fault path`
2. `fault containment / quarantine`
3. `device/network semantic decision points`
4. `contract policy hardening`
5. curățare structurală și unificare de pattern-uri

Motivul:

- primele patru fronturi validate acoperă deja control, wake, ownership și IO lifecycle
- următorul prag real de complexitate este fault path

## Phase Plan

### Phase 0: Stabilize substrate and naming

Scop:

- zero regresii de integrare
- naming uniform
- structuri de observabilitate coerente

Condiții:

- workspace verde
- instrumentarea existentă curată

### Phase 1: Resource lifecycle semantic split

Status:

- realizat și validat

### Phase 2: Wait/wake semantic split

Status:

- realizat și validat

### Phase 3: Scheduler semantic split

Status:

- realizat și validat

### Phase 4: VFS/descriptor/io semantic split

Status:

- realizat și validat

### Phase 5: VM / COW / fault semantic split

Scop:

- separarea clară între fault detection, fault classification, shadow reuse, split/coalesce, dirty/access accounting și commit final

Țintă:

- agenți semantici mici pe path-ul de fault
- observabilitate explicită pentru:
  - fault classify
  - COW decision
  - shadow reuse
  - region split
  - page state update

### Phase 6: Fault containment / quarantine

Scop:

- izolarea deciziilor de containment
- observabilitate mai bună la first-bad-state

### Phase 7: Structural simplification

Scop:

- eliminarea helperilor vechi rămași doar ca datorie
- reducerea punctelor în care planul și mutația dublează aceeași logică

## Anti-Complexity Rules

Aceste reguli sunt obligatorii pentru orice continuare a migrației:

- un agent semantic trebuie să aibă o singură responsabilitate
- niciun agent nu are voie să valideze și să mute mai multe domenii simultan
- fiecare activare trebuie să aibă trigger explicit
- fiecare decizie critică trebuie să fie observabilă
- niciun jurnal nu trebuie să devină flood de low-value noise
- dacă două structuri exprimă aceeași logică, una trebuie eliminată
- nu se introduc queue-uri noi fără motiv demonstrabil
- nu se introduc thread-uri noi pentru acești agenți
- niciun pas nu e acceptat dacă face codul mai greu de urmărit decât înainte

## Authority Narrowing Model

Direcția corectă este:

- helperul semantic decide cât mai puțin și cât mai precis
- substrate-ul execută mutația concretă
- autoritatea nu se propagă implicit

Exemple deja validate:

- `ClaimValidator` nu mută direct tot subsistemul, doar produce planul semantic
- `SleepRequeueAgent` nu rescrie scheduler-ul, doar face mutația strictă de requeue
- `FcntlAgent` este limitat la flags și nu poate muta arbitrar descriptor namespace-ul

## Observability Contract

Fiecare subsistem migrat trebuie să respecte contractul minim de observabilitate:

- jurnal bounded în runtime
- export prin `SystemIntrospection`
- test care verifică jurnalul
- dacă frontul e operator-relevant, proiectare auxiliară permisă doar fără a înlocui path-ul real

Acesta este pragul minim.
Fără el nu există migrare acceptată.

## Success Criteria

Migrarea este considerată reușită dacă:

- subsistemul rămâne verde la teste
- `clippy -D warnings` rămâne verde
- deciziile sunt verificabile prin test
- `inspect_system()` poate explica ce s-a decis
- debugging-ul devine mai scurt și mai local
- nu apare un nou strat greu de menținut

## Failure Criteria

Migrarea trebuie oprită sau redusă dacă:

- începe să dubleze logică fără câștig
- introduce structuri fără testare
- produce zgomot diagnostic disproporționat
- cere queue-uri și scheduling noi fără beneficii măsurabile
- scade lizibilitatea subsistemului

## Current Verdict

În forma actuală a proiectului, direcția este validată.

Nu este validată ca dogmă pentru tot kernelul.
Este validată ca metodă de refactor incremental, aplicabilă în subsisteme grele.

Judecata practică actuală este:

- continuați
- continuați disciplinat
- continuați pe subsisteme reale
- nu transformați modelul într-un slogan

## Next Front

Următorul front recomandat este:

- `VM / COW / fault path`

Acesta este locul unde se decide dacă modelul rămâne bun și în cea mai dificilă parte a kernelului semantic.
