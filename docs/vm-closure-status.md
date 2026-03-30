# VM Closure Status

## Scope

Acest document fixează exact unde a rămas subsistemul `VM`, ce este deja închis, ce rămâne deschis și care este ordinea corectă de continuare.

Nu este un roadmap general al OS-ului.
Este un document de execuție pentru frontul `VM`.

Conform regulilor repo-ului:

- `VM` nu este considerat închis cât timp familii relevante rămân deschise
- validarea `host-runtime` nu este suficientă
- truth path-ul obligatoriu este:
  - `boot-x86_64`
  - `platform-x86_64`
  - `kernel-core`
  - `user-runtime`
  - `userland-native`
  - `QEMU`
  - hardware real

## Stare Curentă

`Subsystem VM is closed.`

Frontul `VM` este închis pe:

- `kernel-core`
- `boot-x86_64`
- `user-runtime`
- `userland-native`

și are acum dovadă reală și pe:

- `QEMU`
- hardware fizic

Frontul rămas deschis anterior, dovada hardware reală pentru smoke-ul `VM`,
este acum acoperit de execuție capturată vizual pe hardware fizic, cu markerii:

- `vm.smoke.map`
- `vm.smoke.protect`
- `vm.smoke.heap`
- `vm.smoke.region`
- `vm.smoke.cow.observe`
- `vm.smoke.cow`
- `vm.smoke.unmap`

și cu finalizare observabilă:

- `first user process report`
- `outcome=success`
- `exit_code=0`
- `boot_status=0`
- `boot_stage=2`

## Ce Este Închis

Familii închise în `VM` până acum:

- `quarantine / release / reclaim-pressure`
- `memory contract / policy block`
- `global reclaim-pressure`
- `protect / unmap`
- `file-backed mappings`
- observabilitate `procfs` pentru:
  - `maps`
  - `vmobjects`
  - `vmdecisions`
  - `vmepisodes`
- path de descriptor `procfs` pe `boot/QEMU`
- smoke `VM` pe `QEMU` pentru:
  - map anonim
  - protect
  - refusal la write pe pagină read-only
  - sync
  - unmap
  - map file-backed
  - protect file-backed
  - sync file-backed
- contoare reale de fault:
  - `read fault`
  - `write fault`
  - `cow fault`
- decizii observabile:
  - `fault-classifier`
  - `page-touch`
  - `map`
  - `brk`
  - `protect`
  - `unmap`
  - `map-file`
  - `sync`
  - `quarantine-block`
- episoade `kind=map`, `kind=heap`, `kind=region`, `kind=fault` și `kind=quarantine` în `vmepisodes`
- episoade `kind=policy` în `vmepisodes`
- `copy_vm -> COW shadow`
- lanțuri `shadow depth > 1`
- `shadow reuse`
- `shadow reuse reverse`
- `shadow bridge`
- offset-uri non-zero în lanțuri `shadow`
- observabilitate `bridged=yes` în `vmepisodes`
- syscall surface pentru `spawn_process_copy_vm`
- reconfirmare live pe `QEMU` pentru:
  - `vmepisodes kind=heap grew=yes shrank=yes`
  - `vmepisodes kind=region protected=yes unmapped=yes`
  - `spawn_process_copy_vm`
  - `store_memory_word` în copil
  - observabilitate serială pentru:
    - `vm.smoke.heap`
    - `vm.smoke.region`
    - `vm.smoke.cow`
    - `object=[cow]`
    - `depth=1`
    - `kind=fault`
    - `cow=yes`
- proof runner dedicat pentru `VM` pe `QEMU`
- hardening `QEMU` pentru:
  - stres repetat `map/protect/unmap`
  - pressure reclaim cu restore observabil
  - global pressure reclaim cross-process
  - workload summary serial pentru:
    - `anon`
    - `cow`
    - `file`
    - `heap`
    - `region`

## Ce A Fost Împins Cap-Coada

Suprafețe deja legate end-to-end:

- `user-abi`
- `user-runtime`
- `kernel-core`
- `boot-x86_64`
- `userland-native`
- proof `QEMU` pentru `heap + region + cow + unmap`
- proof `QEMU` pentru hardening:
  - `vm.smoke.stress`
  - `vm.smoke.pressure`
  - `vm.smoke.pressure.global`
  - `vm.smoke.production`
- activare nativă reală a page tables-urilor materializate pe path-ul:
  - `boot-x86_64`
  - `platform-x86_64`
  - `kernel-core`
  - `user-runtime`
  - `userland-native`
  - `QEMU`

Fișiere cheie deja atinse pe acest front:

- [boot-x86_64/src/user_syscall.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/user_syscall.rs)
- [userland-native/src/lib.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/userland-native/src/lib.rs)
- [user-runtime/src/lib.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/user-runtime/src/lib.rs)
- [user-abi/src/lib.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/user-abi/src/lib.rs)
- [kernel-core/src/user_syscall_runtime/dispatch_basic.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/user_syscall_runtime/dispatch_basic.rs)
- [tooling/x86_64/verify-qemu-vm-log.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/verify-qemu-vm-log.ps1)
- [tooling/x86_64/prove-qemu-vm-smoke.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/prove-qemu-vm-smoke.ps1)

## Ce A Fost Ultimul Front Deschis

Ultimul front deschis în `VM` a fost:

1. Hardware real closure
   - `QEMU` era deja închis pentru smoke-ul `VM`
   - artefactul hardware `limine-uefi-hardware.img` se construiește
   - repo-ul are harness-uri pentru:
     - montare ESP
     - deploy Limine UEFI
     - captură serială `COM1`
     - verificare markerilor de boot
   - dovada hardware fizică există acum și acoperă markerii `VM` relevanți

## Dovada Finală

Dovada finală care a închis `VM` ca subsistem este acum compusă din:

- proof `QEMU` pentru markerii `VM`
- dovadă hardware reală pentru aceiași markeri `VM`
- finalizare observabilă a procesului user principal cu:
  - `outcome=success`
  - `exit_code=0`
  - `boot_status=0`
  - `boot_stage=2`

## Ce S-a Făcut Ultima Dată

Ultimul pas tehnic util deja făcut:

- smoke-ul nativ `VM` a fost curățat ca să nu mai citească noduri `procfs` mari integral în `String`
- verificările au fost mutate pe marker-e în streaming
- `BootProcfsNodeKind::Maps` a fost mutat și el pe citire incrementală în `boot-x86_64`
- markerii `QEMU` pentru COW au fost expuși explicit în serial:
  - `vm.smoke.heap`
  - `vm.smoke.region`
  - `vm.smoke.cow.observe`
  - `object=[cow]`
  - `depth=1`
  - `kind=fault`
  - `cow=yes`
- smoke-ul `VM` pe `QEMU` a fost reordonat ca dovada `copy_vm` să ruleze înaintea frontului file-backed mai greu
- verificarea copilului pentru COW folosește acum `vmdecisions` structurale (`shadow-reuse`, `cow-populate`) în loc de o a doua citire costisitoare de `vmepisodes`
- `boot-x86_64` expune acum în `vmepisodes` și familiile:
  - `map`
  - `heap`
  - `region`
- `platform-x86_64` are acum un manager HAL de address-space validat pe teste dedicate
- `platform-x86_64` are acum și integrare `user_mode -> AddressSpaceManager`
- `platform-x86_64` poate acum materializa layout-ul unui address space în page tables reale și are test de integrare `user_mode -> address_space_layout -> page tables`
- page tables-urile materializate în `platform-x86_64` propagă acum corect bitul `USER` pe întreg lanțul `PML4 -> PDPT -> PD -> PT -> leaf` pentru user mappings
- `user_mode` are acum și un mapper dedicat care activează address space-ul și materializează page tables-urile în aceeași trecere semantică
- frontul nou de `platform/user_mode` a fost împărțit explicit în agenți semantici mici pentru validare plan, map install, init capture, activate și page-table materialization; mapper-ele au rămas orchestratori
- blockerul de verificare din `acpi` a fost eliminat; `cargo test -p ngos-platform-x86_64 -- --test-threads=1` trece integral
- `vmepisodes` de boot nu mai depinde doar de obiectele încă prezente; familia `region` rămâne observabilă și după `unmap`
- `boot-x86_64` aplică acum `policy-block` pe toate operațiile VM expuse pe path-ul de boot:
  - `map`
  - `map-file`
  - `protect`
  - `unmap`
  - `advise`
  - `sync`
  - `quarantine`
  - `release`
  - `load`
  - `store`
  - `brk`
  - `reclaim`
- `boot-x86_64` expune acum și `kind=policy` în `vmepisodes`
- testele `boot_vm_*` trec integral
- `tooling/x86_64/build-limine-uefi-hardware.ps1` construiește artefactul hardware pregătit pentru deploy
- a fost adăugat verificator hardware dedicat pentru markerii `VM`:
  - `tooling/x86_64/verify-hardware-vm-log.ps1`
- a fost adăugat wrapper-ul de sesiune hardware pentru `VM` pe `COM1`:
  - `tooling/x86_64/hardware-vm-session-com1.ps1`
- a fost adăugat proof runner-ul automat care:
  - construiește imaginea `limine-uefi-vm.img`
  - pornește `QEMU`
  - urmărește serialul
  - validează markerii `VM`
  - oprește instanța după dovadă
- smoke-ul nativ `VM` exercită acum și fronturile de hardening:
  - `vm.smoke.stress`
  - `vm.smoke.pressure`
  - `vm.smoke.pressure.global`
  - `vm.smoke.production`
- verificatorul `QEMU` pentru `VM` cere acum și markerii de hardening noi

## Hardening Status

Fronturile de hardening nou închise pe `QEMU` sunt:

- stres repetat cu refusal observabil
- reclaim sub presiune cu restore observabil
- reclaim global cross-process
- workload mix explicit în serial

Aceste fronturi ridică `VM` peste nivelul de smoke funcțional de bază.

Totuși, dacă bara este mutată de la `subsystem closed` la `production-grade` în sens tare,
rămâne un blocker care nu se rezolvă doar din cod:

- matrice hardware mai largă pentru aceeași dovadă de hardening

Asta înseamnă:

- `VM` rămâne închis ca subsistem
- hardening-ul nou este dovedit pe `QEMU`
- eticheta tare `production-grade across wider hardware matrix` rămâne încă deschisă până la rulări pe mai multe mașini reale

Scopul acestor schimbări:

- reducerea presiunii pe allocator pe path-ul real `QEMU`
- păstrarea aceleiași dovezi semantice
- eliminarea blocajelor false din userland/harness

## Concluzie

`VM` este închis ca subsistem în termenii frontului urmărit aici:

- logică reală
- integrare reală
- observabilitate reală
- dovadă `QEMU`
- dovadă hardware reală
- final state observabil
