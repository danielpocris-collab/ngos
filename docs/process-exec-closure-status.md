# Process Exec Closure Status

Subsystem process-exec is closed.

## Familii închise

- pe suprafața reală `QEMU`, process execution trece acum cap-coadă prin proof-ul dedicat:
  - metadata-only path observabil ca `refusal` sau `observe`
  - recovery pentru child-ul metadata-only
  - `spawn` real pentru child blocking pe aceeași imagine
  - compat proc probe pentru:
    - `fd`
    - `cwd`
    - `exe`
    - `cmdline`
    - `environ`
  - refusal pentru `/proc/<pid>/fd/9999`
  - recovery pentru listarea `fd`
  - succes final cu `exit=0`
  - stare finală observabilă după reap
- pe path-ul `boot-x86_64`, runtime-ul de process execution este instalat real înainte de user launch:
  - [boot-x86_64/src/main.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/main.rs)
  - [boot-x86_64/src/user_syscall.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/user_syscall.rs)
- pe owner-ul smoke-ului:
  - [ngos-shell-proc/src/process_exec_smoke.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/ngos-shell-proc/src/process_exec_smoke.rs)

## Familii rămase deschise

- none pe truth path-ul actual `QEMU`

## Verificare reală

- `powershell -ExecutionPolicy Bypass -File .\tooling\x86_64\prove-qemu-process-exec-smoke.ps1`
- `powershell -ExecutionPolicy Bypass -File .\tooling\x86_64\verify-qemu-process-exec-log.ps1 -LogPath .\target\qemu\serial-process-exec.log`

## Dovezi

- [target/qemu/serial-process-exec.log](/C:/Users/pocri/OneDrive/Desktop/experiment/target/qemu/serial-process-exec.log)

## Comportament nou observabil

- `boot.proof=process-exec`
- `process.exec.smoke.refusal pid=... mode=metadata-only outcome=expected`
- `process.exec.smoke.recovery pid=... mode=signal outcome=ok`
- `process.exec.smoke.spawn pid=... mode=same-image-blocking outcome=ok`
- `compat.abi.smoke.proc.success pid=... fd-count=3 fd0=present fd1=present fd2=present cwd=/ exe=/kernel/ngos-userland-native cmdline=present`
- `compat.abi.smoke.proc.refusal pid=... path=/proc/.../fd/9999 outcome=expected`
- `compat.abi.smoke.proc.recovery pid=... fd-list=ok outcome=ok`
- `process.exec.smoke.success pid=... exit=0 outcome=ok`
- `process.exec.smoke.state pid=... present=no outcome=ok`
- `process-exec-smoke-ok`
