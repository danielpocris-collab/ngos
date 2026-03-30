# NGOS Nano-Semantic OS Roadmap

## Scope

Acest document definește roadmap-ul de închidere cap-coadă pentru întregul `ngos` ca sistem de operare complet, matur, original și executabil.

Roadmap-ul nu este organizat în jurul unor feature-uri izolate sau al unor suprafețe de showcase.
Este organizat în jurul închiderii subsistemelor reale ale OS-ului, în ordinea care maximizează maturitatea produsului și împinge proiectul spre o stare superioară unui sistem precum `Redox`.

Acest document respectă legea proiectului:

- fronturile trebuie închise cap-coadă
- nu se acceptă micro-progres prezentat ca progres real
- nu se acceptă subsisteme mari construite prin manageri opaci
- implementarea trebuie să fie `nano-semantic`
- host-side validation este permisă ca accelerator, dar nu ca adevăr final
- truth path final rămâne `boot-x86_64 -> platform-x86_64 -> kernel-core -> user-runtime -> userland-native -> QEMU -> hardware real`

## Principiu Director

`ngos` trebuie închis ca OS complet prin subsisteme mari livrate cap-coadă, dar implementate intern prin unități semantice mici.

Asta înseamnă simultan:

- scope mare la nivel de livrare
- autoritate îngustă la nivel de implementare
- tranziții explicite
- observabilitate clară
- failure paths reale
- recovery paths reale
- stare finală inspectabilă

Formula corectă este:

- se închid subsisteme complete
- se construiesc intern prin agenți/moduluri nano-semantice

## Definition of Done

Un subsistem este considerat închis numai dacă toate familiile lui relevante sunt:

1. implementate cu logică reală
2. integrate în fluxurile existente
3. observabile prin introspecție sau raportare
4. explicabile cauzal
5. validate end-to-end

Validarea end-to-end trebuie să includă:

1. success path
2. refusal/error path atunci când subsistemul poate refuza
3. recovery/release/rollback path atunci când subsistemul suportă revenire
4. expunerea observabilă a stării finale

Orice subsistem care trece doar pe happy path rămâne deschis.

## Roadmap Order

Ordinea de execuție pentru întregul OS este:

1. Foundation Closure
2. Execution Core Closure
3. VM Closure
4. Storage and Persistence Closure
5. VFS Closure
6. Device Runtime Closure
7. Networking Closure
8. ABI and User Runtime Closure
9. Userland Native Closure
10. Diagnostics and Observability Closure
11. Boot and Platform Closure
12. QEMU Full-System Closure
13. Real Hardware Closure
14. Product Surface Closure
15. Ecosystem and Compatibility Closure

## 1. Foundation Closure

### Objective

Repo-ul trebuie să devină bază executabilă, coerentă și disciplinată pentru restul OS-ului.

### Nano-Semantic Families

- `workspace_build_validation`
- `workspace_test_validation`
- `binary_entry_contract`
- `panic_abort_contract`
- `bootstrap_contract`
- `artifact_integrity_contract`

### Closure Requirements

- `cargo build --workspace` trece
- `cargo test --workspace` trece
- binarele din workspace nu sunt rupte
- entrypoint-urile au contracte clare
- modelele de bootstrap și panică sunt coerente
- artefactele reale ale workspace-ului sunt cunoscute și reproductibile

### Done Means

- nu există binare rupte în workspace
- nu există suprafețe locale care sparg build-ul complet
- execuția de bază poate continua fără a sta pe fundație instabilă

## 2. Execution Core Closure

### Objective

Kernelul trebuie să închidă matur ciclul de execuție al proceselor, thread-urilor și al evenimentelor fundamentale.

### Nano-Semantic Families

- `process_spawn_agent`
- `process_exit_agent`
- `thread_lifecycle_agent`
- `scheduler_enqueue_agent`
- `scheduler_budget_agent`
- `scheduler_rebind_agent`
- `scheduler_remove_agent`
- `syscall_dispatch_agent`
- `descriptor_authority_agent`
- `event_wait_agent`
- `sleep_transition_agent`
- `signal_delivery_agent`
- `system_inspection_agent`

### Closure Requirements

- procesele pot fi create, executate, blocate, trezite, terminate și reap-uite
- scheduler-ul este observabil și cauzal explicabil
- syscalls nu sunt doar expuse, ci integrate în fluxuri reale
- descriptor model și object authority sunt coerente
- state transitions sunt inspectabile

### Done Means

- kernelul poate susține user/runtime stabil fără ambiguități majore de lifecycle
- failure paths și cleanup paths sunt închise

## 3. VM Closure

### Objective

Subsistemul VM trebuie închis integral ca memorie reală, nu doar ca mapări funcționale.

### Nano-Semantic Families

- `vm_map_agent`
- `vm_unmap_agent`
- `vm_protect_agent`
- `page_fault_resolution_agent`
- `cow_shadow_agent`
- `region_split_agent`
- `region_coalesce_agent`
- `file_mapping_agent`
- `reclaim_agent`
- `pressure_policy_agent`
- `fault_quarantine_agent`
- `vm_introspection_agent`

### Closure Requirements

- map/unmap/protect funcționează real
- page faults sunt rezolvate corect și observabil
- COW este complet, nu doar parțial
- split/coalesce sunt cauzal explicabile
- pressure și reclaim au policy și rezultat observabil
- quarantine/fault containment există acolo unde este relevant

### Done Means

- VM este închis ca subsistem, nu doar câteva fronturi izolate
- success, refusal, recovery și final state sunt toate acoperite

## 4. Storage and Persistence Closure

### Objective

`ngos` trebuie să existe în timp, nu doar în sesiunea curentă.

### Nano-Semantic Families

- `block_device_probe_agent`
- `volume_identity_agent`
- `superblock_validation_agent`
- `mount_transition_agent`
- `space_allocation_agent`
- `file_block_mapping_agent`
- `writeback_commit_agent`
- `recovery_replay_agent`
- `storage_refusal_agent`
- `storage_inspection_agent`

### Closure Requirements

- block device real este detectat și integrat
- volume identity este explicită
- mount/unmount sunt reale
- create/open/read/write ajung la persistență reală
- writeback este observabil
- recovery/replay funcționează
- corupția și refuzurile sunt detectabile și explicabile

### Done Means

- fișierele persistă între sesiuni reale ale subsistemului
- starea persistentă poate fi inspectată și explicată
- storage-ul nu mai este doar suprafață de device, ci subsistem matur

## 5. VFS Closure

### Objective

VFS trebuie închis ca namespace, metamodel de fișiere și orchestrator coerent al persistenței.

### Nano-Semantic Families

- `path_resolution_agent`
- `mount_graph_agent`
- `node_create_agent`
- `node_remove_agent`
- `rename_transition_agent`
- `symlink_resolution_agent`
- `metadata_state_agent`
- `descriptor_vfs_coherence_agent`
- `permission_refusal_agent`
- `vfs_inspection_agent`

### Closure Requirements

- path lookup este coerent
- mount graph este explicit și inspectabil
- create/remove/rename au tranziții clare
- symlink/readlink funcționează corect
- descriptor state și VFS state rămân coerente
- failure paths sunt reale și testate
- proof front explicit este expus prin `userland-native` ca `vfs-smoke`
- statusul de închidere este urmărit în [docs/vfs-closure-status.md](C:/Users/pocri/OneDrive/Desktop/experiment/docs/vfs-closure-status.md)

### Done Means

- VFS este închis cap-coadă, nu doar `lookup/open`

## 6. Device Runtime Closure

### Objective

Modelul de device lifecycle trebuie închis pentru toate clasele relevante de dispozitive.

### Nano-Semantic Families

- `device_discovery_agent`
- `device_bind_agent`
- `device_configure_agent`
- `queue_submit_agent`
- `interrupt_completion_agent`
- `device_teardown_agent`
- `device_rebind_agent`
- `device_fault_isolation_agent`
- `device_state_render_agent`

### Device Classes

- `storage_device_agents`
- `network_device_agents`
- `gpu_device_agents`
- `input_device_agents`
- `audio_device_agents`

### Closure Requirements

- device-urile pot fi descoperite, legate, configurate, folosite și eliberate
- lifecycle-ul este observabil
- completions și interrupts sunt integrate real
- rebinding și failure isolation sunt explicite

### Done Means

- device runtime nu mai este doar o colecție de hooks, ci un subsistem complet

## 7. Networking Closure

### Objective

Networking trebuie închis ca subsistem complet, nu ca un singur path de socket.

### Nano-Semantic Families

- `interface_identity_agent`
- `interface_admin_agent`
- `address_config_agent`
- `link_state_agent`
- `socket_bind_agent`
- `socket_connect_agent`
- `tx_submission_agent`
- `rx_delivery_agent`
- `network_backpressure_agent`
- `network_watch_agent`
- `network_teardown_agent`
- `network_inspection_agent`

### Closure Requirements

- interfaces au lifecycle complet
- address config și admin state sunt reale
- sockets bind/connect/send/recv funcționează end-to-end
- readiness/watchers sunt integrate
- tx/rx/drop/backpressure sunt observabile
- teardown și recovery sunt închise

### Done Means

- networking este închis la nivel de subsistem, nu doar la nivel de demo-path

## 8. ABI and User Runtime Closure

### Objective

Contractul user/kernel trebuie să devină stabil, coerent și matur.

### Nano-Semantic Families

- `bootstrap_decode_agent`
- `process_exec_contract_agent`
- `syscall_abi_agent`
- `fd_abi_agent`
- `memory_abi_agent`
- `signal_abi_agent`
- `event_abi_agent`
- `network_abi_agent`
- `errno_contract_agent`

### Closure Requirements

- bootstrap args sunt decodate coerent
- exec contract este stabil
- syscall ABI este consistent între kernel și user
- erorile sunt exprimate clar și uniform
- contractele de memorie și descriptori sunt testabile și observabile

### Done Means

- user/runtime nu depinde de comportamente fragile sau implicite

## 9. Userland Native Closure

### Objective

Sistemul trebuie să poată fi administrat și înțeles din interiorul lui.

### Nano-Semantic Families

- `shell_parse_agent`
- `shell_dispatch_agent`
- `process_admin_agent`
- `storage_admin_agent`
- `mount_admin_agent`
- `network_admin_agent`
- `inspect_render_agent`
- `recovery_operator_agent`
- `system_control_agent`

### Closure Requirements

- shell-ul poate administra procese, storage, mounts, networking și introspecția
- operatorul poate înțelege starea sistemului fără unelte externe ad-hoc
- userland-ul nu este doar control plane experimental, ci suprafață administrabilă matură

### Done Means

- sistemul este utilizabil ca OS, nu doar ca runtime care rulează niște scripturi

## 10. Diagnostics and Observability Closure

### Objective

`ngos` trebuie să aibă un avantaj structural puternic în observabilitate și diagnostic cauzal.

### Nano-Semantic Families

- `process_trace_agent`
- `scheduler_trace_agent`
- `vm_trace_agent`
- `storage_trace_agent`
- `network_trace_agent`
- `device_trace_agent`
- `fault_history_agent`
- `causal_explain_agent`
- `trust_surface_agent`
- `operator_report_agent`

### Closure Requirements

- subsistemele mari pot fi inspectate coerent
- fault history există
- explanation paths sunt reale
- reports sunt utile operatorului
- trust/completeness reporting există acolo unde modelul o cere

### Done Means

- debugging-ul nu mai este bazat pe ghicit sau log spam

## 11. Boot and Platform Closure

### Objective

Path-ul real de boot și platformă trebuie închis înainte de a pretinde closure de OS complet.

### Nano-Semantic Families

- `boot_sequence_agent`
- `platform_memory_agent`
- `interrupt_route_agent`
- `timer_platform_agent`
- `smp_boot_agent`
- `pci_enumeration_agent`
- `platform_fault_agent`
- `boot_observability_agent`

### Closure Requirements

- boot flow este stabil și reproductibil
- memory map și paging bootstrap sunt coerente
- interrupts și timer path sunt stabile
- SMP este funcțional și observabil
- PCI/device enumeration este corectă
- boot diagnostics explică cauzal starea

### Done Means

- boot/platform nu mai este o fundație experimentală, ci path matur

## 12. QEMU Full-System Closure

### Objective

`QEMU` este primul full-system truth surface acceptat pentru închiderea reală a subsistemelor importante.

### Nano-Semantic Families

- `qemu_boot_validation_agent`
- `qemu_storage_validation_agent`
- `qemu_network_validation_agent`
- `qemu_input_validation_agent`
- `qemu_display_validation_agent`
- `qemu_fault_validation_agent`
- `qemu_reboot_validation_agent`

### Closure Requirements

- subsistemele mari merg pe lanțul real complet
- boot, storage, networking și userland sunt observabile end-to-end
- reboot și recovery sunt validate
- failure paths sunt validate și în mediu full-system

### Done Means

- sistemul nu mai este valid doar pe host-runtime

## 13. Real Hardware Closure

### Objective

Sistemul trebuie să ruleze observabil și matur pe hardware real.

### Nano-Semantic Families

- `hardware_probe_agent`
- `hardware_storage_agent`
- `hardware_input_agent`
- `hardware_display_agent`
- `hardware_network_agent`
- `hardware_recovery_agent`
- `hardware_diagnostics_agent`

### Closure Requirements

- probe real al hardware-ului
- init real al dispozitivelor strategice
- I/O real pe hardware
- recovery și diagnostics reale pe hardware
- fault paths și degradare controlată acolo unde este necesar

### Done Means

- `ngos` nu mai este doar un sistem validat sintetic, ci un OS executabil pe mașină reală

## 14. Product Surface Closure

### Objective

Suprafața de produs vine după closure-ul OS-ului, nu înainte.

### Nano-Semantic Families

- `ui_session_agent`
- `window_or_surface_agent`
- `graphics_composition_agent`
- `browser_integration_agent`
- `app_lifecycle_agent`

### Closure Requirements

- UI sau shell-ul grafic este integrat peste fundație reală
- graphics composition este observabilă
- aplicațiile au lifecycle real
- browserul și suprafețele de produs stau pe OS matur, nu pe fundație fragilă
- planul de execuție pentru engine-ul de rendering este urmărit în [docs/graphics-render-engine-plan.md](C:/Users/pocri/OneDrive/Desktop/experiment/docs/graphics-render-engine-plan.md)

### Done Means

- produsul are suprafață de utilizare finală fără a compromite identitatea internă

## 15. Ecosystem and Compatibility Closure

### Objective

Compatibilitatea externă și ecosistemul vin după ce identitatea internă este închisă și matură.

### Nano-Semantic Families

- `native_package_agent`
- `native_toolchain_agent`
- `linux_compat_adapter_agent`
- `windows_compat_adapter_agent`
- `app_distribution_agent`
- `developer_workflow_agent`

### Closure Requirements

- aplicațiile native au cale de distribuție și rulare
- toolchain-ul de dezvoltare este coerent
- compatibilitatea rămâne adaptor, nu fundație
- ecosistemul extinde `ngos`, nu îi diluează modelul

### Done Means

- `ngos` poate susține software util fără a-și pierde identitatea arhitecturală

## Strategic Non-Goals During Core Closure

Până la închiderea fazelor structurale, următoarele nu trebuie să consume bugetul principal:

- extindere de browser ca suprafață dominantă
- showcase UI
- compat layers care devin pseudo-identitate
- demo binaries și fluxuri fără valoare structurală
- suprafețe mari care ascund mutația în manageri opaci

## Immediate Execution Priority

Ordinea imediată de lucru trebuie să fie:

1. `Foundation Closure`
2. `Execution Core Closure`
3. `VM Closure`
4. `Storage and Persistence Closure`
5. `VFS Closure`

Acesta este nucleul care transformă repo-ul din fundație ambițioasă în OS matur.

## Success Condition Versus Redox

`ngos` nu depășește `Redox` doar prin:

- idei mai originale
- model intern mai interesant
- mai multe crate-uri
- o suprafață de browser în workspace

`ngos` depășește `Redox` numai dacă închide superior:

- execuția kernelului
- VM-ul
- persistența
- VFS-ul
- device runtime-ul
- networking-ul
- observabilitatea cauzală
- boot/platform path-ul
- userland-ul administrabil
- truth path-ul QEMU și hardware real

Formula finală este:

- mai puțină suprafață ornamentală
- mai multă închidere de subsisteme reale
- mai puțină centralizare opacă
- mai multă nano-semantică observabilă
- mai puțină validare simbolică
- mai multă closure cap-coadă
