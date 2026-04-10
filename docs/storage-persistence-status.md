Subsystem Storage + Persistence is closed on the QEMU truth surface.

Familii închise:
- `block_device_probe_agent`
  - `virtio-blk` real este detectat și integrat pe calea:
    - `boot-x86_64`
    - `platform-x86_64`
    - `user-runtime`
    - `userland-native`
    - `QEMU`
- `volume_identity_agent`
  - volumul persistent expune identitate explicită:
    - `volume_id`
    - `superblock_sector`
    - `journal_sector`
    - `data_sector`
    - `index_sector`
    - `alloc_sector`
    - `data_start_sector`
- `superblock_validation_agent`
  - superblock-ul are:
    - `magic`
    - `version`
    - `generation`
    - `dirty`
    - `payload_checksum`
    - `last_commit_tag`
- `writeback_commit_agent`
  - commit-ul pregătit scrie jurnalul și marchează superblock-ul ca `prepared`
- `recovery_replay_agent`
  - replay-ul citește jurnalul, îl aplică și mută volumul în starea `recovered`
- `storage_refusal_agent`
  - refusal-urile reale expuse și demonstrate sunt:
    - `prepare` oversized cu `E2BIG`
    - `unmount` peste limita tabelei de snapshot cu `E2BIG`
    - `mount` pentru snapshot corupt cu `EINVAL`
    - `unmount` repetat cu `ENOENT`
- `storage_inspection_agent`
  - `inspect_storage_volume` expune starea persistentă observabilă, inclusiv:
    - `allocation_total_blocks`
    - `allocation_used_blocks`
    - `mapped_file_count`
    - `mapped_directory_count`
    - `mapped_symlink_count`
    - `mapped_extent_count`
- `mount_transition_agent`
  - `storage-mount` și `storage-unmount` leagă volumul persistent de `BOOT_VFS`
- `persistent_vfs_snapshot_agent`
  - snapshot-ul persistent al subtree-ului montat este serializat și reaplicat real pe `QEMU`
  - persistă:
    - directoare
    - fișiere
    - symlink-uri
- `space_allocation_agent`
  - allocatorul persistent nu mai este fix la `8` blocuri
  - layout-ul folosește bitmap multi-sector derivat din capacitatea reală a device-ului
  - pe discul QEMU curent:
    - `alloc-total=131035`
- `file_block_mapping_agent`
  - mapping-ul persistent nu mai este un singur extent contiguu per fișier
  - snapshot-ul folosește extent table reală
  - pe proba QEMU:
    - `extents=4`
    - `asset.bin` este reconstruit corect prin mapping multi-extent
- `storage_corruption_repair_agent`
  - corupția bitmap-ului de alocare este injectată real pe `virtio-blk`
  - `mount` refuză snapshot-ul corupt cu `EINVAL`
  - `repair_storage_snapshot` reconstruiește bitmap-ul și readuce volumul în stare montabilă

Familii rămase deschise:
- niciuna pe scope-ul `Storage + Persistence` pe `QEMU`

Implementare:
- ABI storage semantic:
  - [user-abi/src/lib.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/user-abi/src/lib.rs)
- wrappere runtime:
  - [user-runtime/src/lib.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/user-runtime/src/lib.rs)
- implementare boot-side peste `virtio-blk` real:
  - [boot-x86_64/src/virtio_blk_boot.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/virtio_blk_boot.rs)
  - [boot-x86_64/src/user_syscall.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/user_syscall.rs)
  - [boot-x86_64/src/user_process.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/user_process.rs)
- shell/userland exposure:
  - [userland-native/src/lib.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/userland-native/src/lib.rs)
  - [userland-native/src/surface_agents.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/userland-native/src/surface_agents.rs)
- proof și verificare QEMU:
  - [tooling/x86_64/prove-qemu-storage-smoke.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/prove-qemu-storage-smoke.ps1)
  - [tooling/x86_64/verify-qemu-storage-log.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/verify-qemu-storage-log.ps1)

Verificare reală:
- `cargo test -p ngos-userland-native --lib --no-run`
- `cargo test -p ngos-boot-x86_64 --lib --no-run`
- `powershell -ExecutionPolicy Bypass -File .\tooling\x86_64\prove-qemu-storage-smoke.ps1`

Comportament nou observabil:
- primul boot:
  - `storage.smoke.success ...`
  - `storage.smoke.refusal op=prepare errno=E2BIG outcome=expected`
  - `storage.smoke.recovery ... outcome=ok`
  - `storage.smoke.clear ... outcome=ok`
  - `storage.smoke.mapping.refusal op=unmount errno=E2BIG outcome=expected`
  - `storage.smoke.mount.commit mount=/persist entries=0 files=2 dirs=2 symlinks=1 alloc-total=131035 ... outcome=ok`
  - `storage.smoke.mount.refusal op=unmount errno=ENOENT outcome=expected`
- al doilea boot:
  - `storage.smoke.mount.recovery mount=/persist entries=5 files=2 dirs=2 symlinks=1 payload=persist:qemu-vfs-session-001 asset-bytes=900 alloc-total=131035 alloc-used=4 extents=4 generation=4 outcome=ok`
- al treilea boot:
  - `storage.smoke.corruption sector=... kind=alloc-bitmap outcome=written`
  - `storage.smoke.corruption.refusal op=mount errno=EINVAL outcome=expected`
  - `storage.smoke.corruption.repair generation=4 alloc-total=131035 alloc-used=4 files=2 dirs=2 symlinks=1 extents=4 outcome=ok`
  - `storage.smoke.corruption.recovery mount=/persist ... outcome=ok`

Dovezi:
- [target/qemu/serial-storage.log](/C:/Users/pocri/OneDrive/Desktop/experiment/target/qemu/serial-storage.log)
- [target/qemu/serial-storage-commit.log](/C:/Users/pocri/OneDrive/Desktop/experiment/target/qemu/serial-storage-commit.log)
- [target/qemu/serial-storage-recover.log](/C:/Users/pocri/OneDrive/Desktop/experiment/target/qemu/serial-storage-recover.log)
- [target/qemu/serial-storage-corrupt.log](/C:/Users/pocri/OneDrive/Desktop/experiment/target/qemu/serial-storage-corrupt.log)
