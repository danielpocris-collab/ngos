# Scheduler Closure Status

Subsystem scheduler is closed.

## Familii închise

- pe suprafața reală `QEMU`, schedulerul trece acum cap-coadă prin proof-ul dedicat:
  - refusal pentru `/proc/system/scheduler` fără contract `observe`
  - observare globală prin `/proc/system/scheduler`
  - observare fairness prin:
    - `lag-debt`
    - `dispatches`
    - `runtime-ticks`
    - `fairness-dispatch-total`
    - `fairness-runtime-total`
    - `fairness-runtime-imbalance`
  - observare `per-CPU / balancing` prin:
    - `cpu-summary`
    - `cpu\tindex=...`
    - `apic-id`
    - `package`
    - `core-group`
    - `sibling-group`
    - `inferred-topology=true`
    - `cpu-queue\tindex=...\tclass=...\ttids=[...]`
    - `rebalance-ops`
    - `rebalance-migrations`
    - `last-rebalance`
  - dovadă explicită `SMP` pe `QEMU`:
    - `ngos/x86_64: smp bootstrap_apic=0 cpus=2 ...`
    - `scheduler.smoke.cpu count=2 ...`
  - `spawn` real pentru worker
  - `renice` real la `background`
  - `pause`
  - `resume`
  - vizibilitate în coada schedulerului
  - `recovery` prin `kill + reap`
  - stare finală observabilă după ieșirea procesului
- pe path-ul `boot-x86_64`, există acum:
  - `/proc/system`
  - `/proc/system/scheduler`
  - expunere reală pentru:
    - `tokens`
    - `wait-ticks`
    - `decision`
    - `running`
    - `queue ... tids=[...]`
  - metadata reale de scheduler în `inspect_process`
  - syscall-uri reale pentru:
    - `pause`
    - `resume`
    - `renice`
- pe modelul `kernel-core`, rămân valide:
  - anti-starvation între clase
  - fast reschedule
  - urgent wakeup
  - observabilitate `tokens` / `wait-ticks`
  - service accounting per clasă
  - fairness summary agregat

## Familii rămase deschise

- none pe truth path-ul actual `QEMU`

## Implementare

- proof și verificare `QEMU`:
  - [tooling/x86_64/prove-qemu-scheduler-smoke.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/prove-qemu-scheduler-smoke.ps1)
  - [tooling/x86_64/verify-qemu-scheduler-log.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/verify-qemu-scheduler-log.ps1)
- boot path:
  - [boot-x86_64/src/user_syscall.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/user_syscall.rs)
  - [boot-x86_64/src/user_process.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/user_process.rs)
- userland proof:
  - [userland-native/src/lib.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/userland-native/src/lib.rs)

## Verificare reală

- `powershell -ExecutionPolicy Bypass -File .\tooling\x86_64\prove-qemu-scheduler-smoke.ps1`

## Dovezi

- [target/qemu/serial-scheduler.log](/C:/Users/pocri/OneDrive/Desktop/experiment/target/qemu/serial-scheduler.log)

## Comportament nou observabil

- `boot.proof=scheduler`
- `scheduler.smoke.refusal path=/proc/system/scheduler contract=observe outcome=expected`
- `scheduler.smoke.observe path=/proc/system/scheduler tokens=yes wait-ticks=yes lag=yes fairness=yes decisions=yes running=yes cpu=yes cpu-topology=yes cpu-queue=yes rebalance=yes outcome=ok`
- `scheduler.smoke.spawn pid=2 class=interactive outcome=ok`
- `scheduler.smoke.renice pid=2 class=background budget=1 outcome=ok`
- `scheduler.smoke.pause pid=2 state=Blocked outcome=ok`
- `scheduler.smoke.resume pid=2 state=Ready outcome=ok`
- `scheduler.smoke.queue pid=2 class=background visible=yes outcome=ok`
- `scheduler.smoke.fairness dispatch=yes runtime=yes imbalance=yes outcome=ok`
- `scheduler.smoke.cpu count=2 running=yes load=yes cpu-topology=yes cpu-queue=yes rebalance=yes outcome=ok`
- `scheduler.smoke.recovery pid=2 exit=143 outcome=ok`
- `scheduler.smoke.state pid=2 present=no outcome=ok`
- `scheduler-smoke-ok`
