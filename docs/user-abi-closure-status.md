# User ABI Closure Status

`Subsystem User ABI is closed.`

## Scope

Conform [docs/ngos-subsystem-closure-matrix.md](/C:/Users/pocri/OneDrive/Desktop/experiment/docs/ngos-subsystem-closure-matrix.md), în scope intră:

- native records
- bootstrap contract
- snapshot contract
- typed transport

Conform aceleiași matrice, minimul de closure pentru acest subsistem este:

- `kernel-core`
- `user-abi`
- `user-runtime`

Pentru `User ABI`, `QEMU` nu este condiție minimă de oprire.

## Familii Închise

- contract bootstrap canonic:
  - `BootstrapArgs`
  - `BootContext`
  - `SessionContext`
  - `BootCpuContext`
  - `BootOutcomePolicy`
  - `AuxvEntry`
- helperi canonici de transport:
  - `env_value`
  - `has_flag`
  - `aux_value`
  - `from_raw(...)` pentru stări, clase și enum-uri native
  - `SyscallReturn::{ok, err, from_raw, into_result}`
- records native structurate pentru kernel truth:
  - procese
  - snapshot de sistem
  - evenimente
  - networking
  - storage
  - VFS
  - bus
  - resource / contract / domain
  - GPU / device / driver
- transport typed fără drift între layer-ele reale:
  - `kernel-core` scrie record-uri ABI în user memory
  - `user-runtime` emite frame-uri syscall care transportă exact aceste record-uri și contracte
  - `user-abi` rămâne owner-ul structurii și al helperilor de decodare
- bootstrap stack materializat canonic pentru first-user:
  - `argc`
  - `argv`
  - `envp`
  - `auxv`
  - aliniere și `StartFrame`
- boot/session reporting typed:
  - `BootSessionStatus`
  - `BootSessionStage`
  - `BootSessionReport`

## Familii Rămase Deschise

- none pe scope-ul activ al `User ABI`

## Dovezi

- owner-ul ABI:
  - [user-abi/src/lib.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/user-abi/src/lib.rs)
  - [user-abi/src/bootstrap.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/user-abi/src/bootstrap.rs)
- consumatorul direct:
  - [user-runtime/src/lib.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/user-runtime/src/lib.rs)
- writer-ul de truth în kernel:
  - [kernel-core/src/user_syscall_runtime/tests/basic.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/src/user_syscall_runtime/tests/basic.rs)
  - [boot-x86_64/src/user_syscall.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/user_syscall.rs)

## Fluxul Închis

Fluxul închis acum dovedește cap-coadă:

- `user-abi` definește structurile și helperii canonici
- `user-runtime` împachetează aceste contracte în frame-uri syscall exacte
- `kernel-core` și `boot-x86_64` materializează record-uri ABI structurate în user memory
- testele citesc acele structuri înapoi ca tipurile din `user-abi`, nu ca text ad-hoc
- bootstrap/session contractele sunt parse-uite și validate canonic din `argv/envp/auxv`

## Verificare

- `cargo test -p ngos-user-abi`
- `cargo test -p ngos-user-runtime bootstrap_builder_emits_expected_stack_layout -- --nocapture`
- `cargo test -p ngos-user-runtime boot_report_wrapper_emits_expected_syscall_frame -- --nocapture`
- `cargo test -p ngos-kernel-core stat_and_statfs_user_syscalls_copy_structured_records_into_user_memory -- --nocapture`
- `cargo test -p ngos-kernel-core networking_user_syscalls_configure_bind_inspect_and_move_udp_traffic -- --nocapture`
- `cargo test -p ngos-boot-x86_64 boot_report_syscall_records_structured_boot_session_report -- --nocapture`

## Comportament Observabil

- parsing canonic al bootstrap/session context din `argv/envp/auxv`
- record-uri native copiate în user memory ca structuri ABI:
  - `NativeFileStatusRecord`
  - `NativeFileSystemStatusRecord`
  - `NativeNetworkInterfaceRecord`
  - `NativeNetworkSocketRecord`
  - `NativeSystemSnapshotRecord`
- syscall wrappers din `user-runtime` emit frame-uri stabile pentru:
  - inspect
  - list
  - boot report
  - networking
  - event queue
  - resource / bus / process / VFS
