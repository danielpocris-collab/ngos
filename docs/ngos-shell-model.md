# `ngos` Shell Model

`ngos shell` nu este un CLI clasic tratat ca utilitar periferic.
În direcția activă a proiectului, shell-ul este un subsistem nano-semantic de control, orchestrare și observabilitate pe calea reală:

- `boot-x86_64`
- `platform-x86_64`
- `kernel-core`
- `user-runtime`
- `userland-native`
- `QEMU`
- hardware real

## Ce Este

`ngos shell` este suprafața interactivă și scriptabilă prin care sunt expuse:

- comenzi și fluxuri reale asupra subsistemelor
- introspecție `procfs` și runtime state
- agenți de workflow pentru mutații compuse
- semantici de refusal, recovery și observabilitate
- fronturi de proof end-to-end pentru subsisteme reale

În implementarea curentă, shell-ul este găzduit în:

- [userland-native/src/lib.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/userland-native/src/lib.rs)
- [userland-native/src/workflow_agents.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/userland-native/src/workflow_agents.rs)
- [userland-native/src/shell_lang.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/userland-native/src/shell_lang.rs)
- [userland-native/src/shell_state_agents.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/userland-native/src/shell_state_agents.rs)

## Ce Nu Este

`ngos shell` nu este:

- un wrapper superficial peste un model străin de shell
- un shell monolitic acceptat să crească prin acumulare opacă
- o suprafață demo separată de subsistemele reale
- un front validat doar în `host-runtime`

## Unități Corecte De Înțelegere

Shell-ul trebuie gândit ca ansamblu de agenți și suprafețe mici, nu ca un singur manager intern mare.

Familii structurale vizibile azi:

- `shell_parse_agent`
- `shell_dispatch_agent`
- `shell_state_agents`
- `workflow_agents`
- `procfs_render_agent`
- `resource/control surface`
- `device/runtime control surface`
- `compat/game/render exposure`
- `proof/smoke orchestration`

## Contract Semantic

Un front de shell este valid doar dacă:

- execută logică reală
- mută sau observă stare reală
- are refusal path când comanda poate fi respinsă
- are recovery sau cleanup când fluxul suportă revenire
- este observabil prin output sau introspecție
- este demonstrabil end-to-end pe calea reală relevantă

## Relația Cu `userland-native`

`userland-native` este containerul de execuție curent al shell-ului, dar nu este sinonim cu shell-ul.

- `userland-native` include și alte suprafețe și smoke-uri
- `shell` este control plane-ul interactiv și scriptabil expus de acolo
- closure-ul shell-ului trebuie evaluat ca subsistem propriu, nu doar prin existența binarului
