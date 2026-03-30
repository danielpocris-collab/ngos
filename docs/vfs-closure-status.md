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

## Stare Curentă

`Subsystem VFS is not yet closed.`

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
- refusal path pentru rename invalid
- observable final state

## Ce Rămâne Deschis

Frontul de execuție trebuie încă împins pe truth path-ul repo-ului:

- hardware real

## Dovezi Curente

Frontul `QEMU` este acum demonstrat cap-coadă prin:

- `ngos.boot.proof=vfs`
- `boot.proof=vfs`
- `vfs.smoke.mount`
- `vfs.smoke.create`
- `vfs.smoke.symlink`
- `vfs.smoke.rename`
- `vfs.smoke.unlink`
- `vfs.smoke.coherence`
- `vfs-smoke-ok`

Deci:

- `QEMU VFS proof`: închis
- `Subsystem VFS`: nu încă, până la dovada pe hardware real
