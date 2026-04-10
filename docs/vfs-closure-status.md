# VFS Closure Status

## Scope

Acest document fixează familia `VFS` în termeni nano-semantici:

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

## Model Clarification

`VFS` în `ngos` nu trebuie interpretat ca un `Unix VFS` clasic rescris cu alt cod.

`VFS` în `ngos` este un subsistem nano-semantic de:

- namespace și path resolution
- graph de mount-uri
- lifecycle de noduri și obiecte
- coherență de descriptori și autoritate
- refusal / recovery
- introspecție și observabilitate

Familiile de agenți enumerate mai sus sunt authoritative pentru înțelegerea
subsystemului.

Comparațiile cu Linux, BSD, FreeBSD, Windows, Redox, sau alte sisteme pot fi
utile doar pentru invariants locale sau benchmark de maturitate, nu ca model
arhitectural de adevăr pentru `ngos`.

## Stare Curentă

`Subsystem VFS is closed on the real QEMU path.`

## Ce Este Închis

În `kernel-core`, familia VFS are deja implementare și acoperire de test pentru:

- mount graph
- path resolution
- create/remove/rename
- symlink/readlink
- metadata și `statfs`
- refusal paths:
  - invalid path
  - already exists
  - not found
  - directory not empty
  - cross-mount rename
- descriptor/VFS coherence prin runtime și syscall surface
- fronturi reale de `procfs` pentru `fd` și `fdinfo`
- `descriptor_vfs_coherence_agent` pe path-ul real `QEMU`:
  - `open`
  - `dup`
  - `fcntl`
  - `lock`
  - `fdinfo`
  - `procfs fd`
- `vfs_inspection_agent` pe path-ul real `QEMU`:
  - `list`
  - `statfs`
  - `readlink`
  - observabilitate finală de coerență

## Proof Front

Există acum un front explicit în `userland-native`:

- `vfs-smoke`

Acesta exercită:

- mount
- create
- symlink
- stat / lstat / statfs
- readlink
- open
- rename
- unlink
- link
- truncate
- fd / dup / fcntl / poll
- recovery explicit:
  - rename restore
  - symlink restore
- refusal path pentru rename invalid
- observable final state

## Ce Rămâne Deschis

În afara scope-ului de closure urmărit aici, rămâne deschis doar:

- dovada hardware fizică pentru același front `VFS`

Hardware-ul fizic rămâne o etapă separată, în afara scope-ului curent de closure pe `QEMU`.

## Dovezi Curente

Frontul `QEMU` este demonstrat acum end-to-end prin:

- `ngos.boot.proof=vfs`
- `boot.proof=vfs`
- `vfs.smoke.step=mkdir-root`
- `vfs.smoke.step=mkdir-bin`
- `vfs.smoke.step=mkfile-app`
- `vfs.smoke.step=symlink-link`
- și execuție observabilă reală prin fronturile:
  - `file-written`
  - `file-appended`
  - `file-copied`
  - `procfs fdinfo`
  - `procfs fd`
  - `dup`
  - `fcntl`
  - `lock`
  - `coherence`
  - `mount propagation`
  - `replace`
  - `tree copy/mirror`
  - `vm-file`

Deci:

- `QEMU VFS proof`: închis
- `Subsystem VFS` pe path-ul real `QEMU`: închis
