# NGOS Unified Compat Layer - Status

Acest document urmărește starea curentă a stratului unificat de compatibilitate al NGOS și ce mai rămâne de implementat pentru a depăși un model de tip Wine/Proton.

## Obiectiv

Scopul nu este să clonăm Wine și Proton ca produse separate, ci să avem un singur strat NGOS unificat care:

- rulează aplicații și jocuri străine prin contracte clare
- traduce API-uri grafice majore către backend-ul intern NGOS
- expune observabilitate completă pentru launch, session, graphics, audio și input
- rămâne compatibil cu execuția reală NGOS, nu doar cu host-side validation

## Ce este deja închis

### Modelul de compatibilitate

- `game-compat-runtime` are un model explicit pentru sursa grafică:
  - `Direct3D9`
  - `Direct3D10`
  - `DirectX11`
  - `DirectX12`
  - `OpenGL`
  - `OpenGLES`
  - `Metal`
  - `Vulkan`
  - `WebGPU`
  - `Wgpu`
  - `Other`
- `GameCompatManifest` are `target` explicit:
  - `game`
  - `app`
  - `tool`
  - `other`
- `GraphicsTranslationPlan` există și este raportat prin runtime

### Orchestrare de sesiune

- manifestul se parsează și se validează
- se construiește un `GameSessionPlan`
- sesiunea expune:
  - target
  - domain
  - process
  - env shims
  - lanes pentru graphics / audio / input
- `userland-native` raportează acum explicit:
  - API-ul sursă
  - backend-ul intern
  - traducerea folosită

### Teste și observabilitate

- testele pentru `ngos-game-compat-runtime` trec
- testele pentru `ngos-userland-native` trec
- lifecycle-urile pentru launch / stop / watch / payload queue sunt validate
- input, audio și graphics au fluxuri de raportare reale

### Lane-uri compat pe calea reală QEMU

- `graphics` este închis pe:
  - `boot-x86_64`
  - `platform-x86_64`
  - `kernel-core`
  - `user-runtime`
  - `userland-native`
  - `QEMU`
- `audio` este închis pe aceeași cale reală până la `QEMU`
- `input` este închis pe aceeași cale reală până la `QEMU`
- există probe executabile și verificatori dedicați pentru:
  - `compat-gfx-smoke`
  - `compat-audio-smoke`
  - `compat-input-smoke`
  - `compat-loader-smoke`
  - `compat-abi-smoke`
- există acum și o suită unificată de validare QEMU:
  - `tooling/x86_64/prove-qemu-unified-compat-suite.ps1`
  - aceasta rulează:
    - `graphics`
    - `audio`
    - `input`
    - `foreign`
  - iar `foreign` rulează:
    - `abi smoke`
    - `loader smoke`

### Loader / launcher smoke pe calea reală QEMU

- `loader / launcher` are acum smoke executabil pe:
  - `boot-x86_64`
  - `kernel-core`
  - `user-runtime`
  - `userland-native`
  - `QEMU`
- smoke-ul acoperă:
  - success path
  - refusal path la `manifest-load-failed`
  - relaunch / recovery path
  - cleanup final observabil
- loader-ul are acum și `routing plan` automat, derivat din:
  - `target`
  - `gfx.api`
  - `translation`
  - familiile de override:
    - `shim.preload`
    - `shim.dll`
    - `env.override`
- routing-ul este observabil în:
  - `game.plan.loader`
  - `game.session.profile.loader`
  - `game.loader.status`
  - `game.session.loader`
  - fișierul bootstrap:
    - `session.loader`
- loader smoke validează acum și schimbarea reală de routing:
  - success path:
    - `compat-game-runtime`
    - `compat-shim`
    - `dx-to-vulkan-entry`
    - `shim-heavy`
  - recovery path:
    - `native-app-runtime`
    - `native-direct`
    - `native-vulkan-entry`
    - `env-overlay`
  - matrix path:
    - `compat-tool-runtime`
    - `compat-shim`
    - `webgpu-to-vulkan-entry`
    - `bootstrap-light`
  - native-with-shims path:
    - `native-other-runtime`
    - `native-direct`
    - `native-vulkan-entry`
    - `shim-heavy`
- dovada este în:
  - `tooling/x86_64/prove-qemu-compat-loader-smoke.ps1`
  - `tooling/x86_64/verify-qemu-compat-loader-log.ps1`

### Izolare QEMU pentru toate proof-urile compat

- toate proof-urile compat folosesc acum artefacte QEMU dedicate:
  - `target/qemu/limine-uefi-compat-gfx`
  - `target/qemu/limine-uefi-compat-gfx.img`
  - `target/qemu/limine-uefi-compat-audio`
  - `target/qemu/limine-uefi-compat-audio.img`
  - `target/qemu/limine-uefi-compat-input`
  - `target/qemu/limine-uefi-compat-input.img`
  - `target/qemu/limine-uefi-compat-loader`
  - `target/qemu/limine-uefi-compat-loader.img`
  - `target/qemu/limine-uefi-compat-abi`
  - `target/qemu/limine-uefi-compat-abi.img`
- izolarea asta elimină:
  - contaminarea de config între proof-uri
  - boot-uri care porneau pe frontul greșit
  - lock-urile pe imaginea comună

### ABI compatibility smoke pe calea reală QEMU

- `ABI compatibility` are acum smoke executabil pe:
  - `boot-x86_64`
  - `kernel-core`
  - `user-runtime`
  - `userland-native`
  - `QEMU`
- smoke-ul acoperă:
  - success path pentru:
    - handles
    - path normalization
    - scheduler mapping
    - mutex
    - eventing
    - timer semantics:
      - one-shot fire + disarm
      - periodic rearm + fire count
    - module semantics:
      - load
      - retain
      - release
    - routing ABI per target:
      - `compat-game-abi`
      - `compat-app-abi`
      - `compat-tool-abi`
      - `compat-other-abi`
  - refusal path pentru:
    - handle close invalid
    - path traversal
    - scheduler class necunoscută
    - mutex contention și unlock invalid
    - timer periodic cu interval zero
    - retain pe modul deja unloaded
  - recovery path pentru:
    - handle release complet
    - mutex unlock
    - event reset
    - path normalize valid după refuz
    - scheduler remap valid după refuz
    - timer cancel după refuz
    - module reload valid după refuz
- dovada este în:
  - `tooling/x86_64/prove-qemu-compat-abi-smoke.ps1`
  - `tooling/x86_64/verify-qemu-compat-abi-log.ps1`
- smoke-ul ABI validează acum și fișierele bootstrap ABI reale:
  - `session.abi`
  - profilele:
    - handles
    - paths
    - scheduler
    - sync
    - timer
    - module
    - event
- proof-ul ABI folosește acum artefacte QEMU dedicate:
  - `target/qemu/limine-uefi-compat-abi`
  - `target/qemu/limine-uefi-compat-abi.img`
- izolarea asta elimină contaminarea de config și lock-urile dintre `compat-abi` și celelalte proof-uri compat

## Ce rămâne deschis

Pe scope-ul `QEMU`, stratul unificat de compatibilitate este închis.

### 1. Extinderi viitoare peste stratul deja închis

Stratul este închis pe `QEMU`, dar pot exista extinderi viitoare care nu mai sunt condiții de closure pentru starea curentă:

- profile per aplicație mai bogate
- tuning per device / per joc
- acoperire suplimentară pentru edge cases care nu blochează flow-urile deja dovedite

Hardware-ul fizic rămâne o etapă separată, doar când reintră explicit în scope.

## Ce ne face mai buni decât Wine/Proton

Pentru a depăși Wine/Proton, NGOS trebuie să aibă câteva diferențe structurale:

- un singur compat layer unificat, nu două produse separate
- integrare nativă cu modelul NGOS de kernel / resource / contract
- observabilitate end-to-end în shell și runtime
- un translator grafic propriu, nu doar wrapper peste un ecosistem existent
- closure pe path-ul real `boot/platform/kernel/runtime/userland/QEMU`, nu pe host-side
- contracte explicite pentru launch, session, lanes și recovery

## Ordinea recomandată de lucru

1. translator grafic efectiv pentru lane-ul GPU
2. ABI compatibility mai adânc pentru aplicații străine
3. loader / launcher compatibil
4. audio și input mai precise
5. demonstrație pe path-ul real de boot / platform / hardware

## Plan de implementare concret

### Front 1: Translator grafic efectiv

Scop:
- să traducem efectiv API-urile grafice majore către backend-ul intern NGOS

Module implicate:
- `game-compat-runtime/src/lib.rs`
- `userland-native/src/lib.rs`
- `gfx-translate/src/lib.rs`
- `gfx-translate/src/frame_script_agent.rs`
- `gfx-translate/src/render_command_agent.rs`
- `kernel-core`
- `platform-hal`

Capabilități care trebuie închise:
- mapare API sursă -> backend intern
- validare lane graphics
- submit / present / scanout pe calea reală
- refusal path pentru API-uri sau capabilități neacceptate
- observabilitate în shell și sesiune

### Front 2: ABI compatibility mai adânc

Scop:
- să suportăm aplicații străine la nivel de ABI, nu doar de manifest

Module implicate:
- `user-runtime`
- `userland-native/src/lib.rs`
- `kernel-core`

Capabilități care trebuie închise:
- handles și object lifecycle
- thread / sync primitives
- timing și scheduling semantics
- filesystem/path normalization
- eventing și fallback

### Front 3: Loader / launcher compatibil

Scop:
- să pornească executabile străine cu profile și shims corecte

Module implicate:
- `userland-native/src/lib.rs`
- `game-compat-runtime/src/lib.rs`
- `host-runtime/src/main.rs`

Capabilități care trebuie închise:
- launch pe profile per aplicație
- env shims consistente
- cwd / argv / exit state
- recovery / rollback la eșecul de launch

### Front 4: Audio și input compatibile

Scop:
- să avem lane-uri complete și observabile pentru audio și input

Module implicate:
- `audio-translate`
- `input-translate`
- `userland-native/src/lib.rs`

Capabilități care trebuie închise:
- plan de traducere audio și input
- status și queue drain
- delivery / completion / refusal
- runtime payload observabil

### Front 5: Real hardware closure

Scop:
- să demonstrăm compat layer-ul pe calea reală, nu doar host-side

Module implicate:
- `boot-x86_64`
- `platform-x86_64`
- `kernel-core`
- `user-runtime`
- `userland-native`

Capabilități care trebuie închise:
- boot -> platform -> kernel -> user runtime
- observabilitate pe hardware real
- refusal/error/recovery paths pe calea reală
- demonstrație executabilă, nu doar raportată

## Concluzie

Stratul unificat de compatibilitate NGOS există deja ca model, contract și observabilitate.

Ce lipsește pentru a-l face mai bun decât Wine/Proton este traducerea reală a API-urilor grafice și închiderea pe calea reală de execuție, nu doar pe validare locală.

## Plan de execuție strict

### Regula de lucru

- un singur front activ la un moment dat
- fără blocuri demo / minimal / mock
- fiecare front trebuie să includă:
  - logică reală
  - integrare în fluxul existent
  - efect runtime observabil
  - test sau demonstrație reală
  - refusal / error path
  - recovery / rollback dacă se aplică

### Ordinea de execuție

#### Pasul 1: Translator grafic efectiv

Fișiere de atins:
- `game-compat-runtime/src/lib.rs`
- `userland-native/src/lib.rs`
- `gfx-translate/src/lib.rs`
- `gfx-translate/src/frame_script_agent.rs`
- `gfx-translate/src/render_command_agent.rs`
- `kernel-core`
- `platform-hal`

Ce trebuie obținut:
- API sursă -> backend intern -> submit/present/scanout
- raportare explicită în shell
- refusal path pentru API-uri neacoperite

Testare:
- parse manifest cu mai multe API-uri grafice
- session plan cu traducere explicită
- lifecycle launch/stop
- probe de submit/present observabile

#### Pasul 2: ABI compatibility mai adânc

Fișiere de atins:
- `user-runtime`
- `userland-native/src/lib.rs`
- `kernel-core`

Ce trebuie obținut:
- handles
- thread/sync
- path normalization
- timing/scheduling semantics
- fallback și refusal path pentru incompatibilități

Testare:
- lifecycle de obiecte
- scheduling și sync observabile
- failure/recovery paths

#### Pasul 3: Loader / launcher compatibil

Fișiere de atins:
- `userland-native/src/lib.rs`
- `game-compat-runtime/src/lib.rs`
- `host-runtime/src/main.rs`

Ce trebuie obținut:
- launch per aplicație
- profile routing
- env shims consistente
- restore/rollback după eșec de launch

Testare:
- manifest valid / invalid
- launch / stop / re-launch
- sesiunile să raporteze final state

#### Pasul 4: Audio și input compatibile

Fișiere de atins:
- `audio-translate`
- `input-translate`
- `userland-native/src/lib.rs`

Ce trebuie obținut:
- plan de traducere complet
- queue drain și delivery consistent
- report pentru success/refusal/recovery

Testare:
- input batch
- audio batch
- queue status înainte și după consum

#### Pasul 5: Real hardware closure

Fișiere de atins:
- `boot-x86_64`
- `platform-x86_64`
- `kernel-core`
- `user-runtime`
- `userland-native`

Ce trebuie obținut:
- demonstrație pe calea reală
- observabilitate completă
- fără dependență de host-only validation

Testare:
- boot path complet
- real runtime path
- refusal/recovery pe hardware real sau QEMU truth surface

### Criteriu de închidere finală

Compat layer-ul este considerat mai bun decât Wine/Proton doar când:

- translation layer-ul grafic e real
- ABI compatibility acoperă aplicații străine în practică
- launcher-ul e capabil și observabil
- audio și input sunt stabile pe toate lane-urile
- sistemul e demonstrat pe path-ul real de boot/platform/hardware

Până atunci, documentul rămâne deschis și trebuie tratat ca plan activ de execuție, nu ca rezultat final.
