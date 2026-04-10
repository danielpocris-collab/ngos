# Bus Closure Status

Subsystem bus is not yet closed.

## Familii închise

- pe suprafața reală `QEMU`, `bus` are acum smoke proof cap-coadă pentru:
  - refusal la `/proc/system/bus` fără contract `observe`
  - observare globală prin `/proc/system/bus`
  - creare reală de:
    - `domain`
    - `channel resource`
    - `channel path`
    - `bus peer`
    - `bus endpoint`
  - `attach`
  - `publish`
  - `receive`
  - overflow de coadă la capacitate maximă
  - refusal cu `Again` la publish peste limită
  - recovery după `receive` urmat de `publish`
  - `detach`
  - refusal după `detach`
  - recovery prin `reattach + publish + receive`
  - stare finală observabilă pentru:
    - `queue-depth`
    - `queue-capacity`
    - `queue-peak`
    - `overflows`
    - `publishes`
    - `receives`
- pe path-ul `kernel-core -> user-runtime -> userland-native`, rămân închise:
  - modelul canonic `peer + endpoint`
  - rights explicite `READ/WRITE` per attachment `peer -> endpoint`
  - refusal real la `publish` fără `WRITE`
  - refusal real la `receive` fără `READ`
  - recovery prin reatașare cu `readwrite`
  - observabilitate locală pentru:
    - `readable-endpoints`
    - `writable-endpoints`
    - `readers`
    - `writers`
  - syscall surface numeric și semantic
  - `event queue` integration
  - `verified core` pentru `bus-integrity`
  - policy `Io`
  - telemetrie de coadă și overflow
  - izolare multi-peer și multi-endpoint

## Familii rămase deschise

- model mai fin de revocare / delegare peste rights-urile explicite `READ/WRITE` deja atașate pe `peer -> endpoint`
- integrare într-un subsistem mai mare peste `bus`
- closure globală pe hardware fizic
- fault/stress/concurență mai grea decât smoke-ul actual `QEMU`

## Implementare

- proof și verificare `QEMU`:
  - [tooling/x86_64/prove-qemu-bus-smoke.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/prove-qemu-bus-smoke.ps1)
  - [tooling/x86_64/verify-qemu-bus-log.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/verify-qemu-bus-log.ps1)
- tooling dedicat pentru hardware fizic:
  - [tooling/x86_64/deploy-limine-uefi-hardware-bus.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/deploy-limine-uefi-hardware-bus.ps1)
  - [tooling/x86_64/hardware-bus-session-com1.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/hardware-bus-session-com1.ps1)
  - [tooling/x86_64/verify-hardware-bus-log.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/verify-hardware-bus-log.ps1)
- boot path:
  - [boot-x86_64/src/user_process_bootstrap.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/user_process_bootstrap.rs)
  - [boot-x86_64/src/user_process.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/user_process.rs)
- userland proof:
  - [userland-native/src/proof_agents.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/userland-native/src/proof_agents.rs)
  - [userland-native/src/surface_agents.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/userland-native/src/surface_agents.rs)
  - [userland-native/src/lib.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/userland-native/src/lib.rs)

## Verificare reală

- `cargo test -p ngos-userland-native native_program_runs_bus_bootproof_and_reports_bus_markers -- --nocapture`
- `cargo test -p ngos-boot-x86_64 bootstrap_inputs_propagate_supported_boot_proof_from_cmdline -- --nocapture`
- `powershell -ExecutionPolicy Bypass -File .\tooling\x86_64\prove-qemu-bus-smoke.ps1`
- `powershell -ExecutionPolicy Bypass -File .\tooling\x86_64\verify-hardware-bus-log.ps1 -LogPath .\target\qemu\serial-bus.log`

## Blocaj real rămas pe hardware fizic

- tooling-ul dedicat pentru `bus` pe hardware fizic există acum, dar în acest mediu nu am un target hardware serial conectat ca să execut `hardware-bus-session-com1.ps1`
- deci closure globală pe hardware fizic rămâne încă deschisă din lipsa execuției pe mașina reală, nu din lipsa lanțului de proof

## Dovezi

- [target/qemu/serial-bus.log](/C:/Users/pocri/OneDrive/Desktop/experiment/target/qemu/serial-bus.log)

## Comportament nou observabil

- `boot.proof=bus`
- `bus.smoke.refusal path=/proc/system/bus contract=observe outcome=expected`
- `bus.smoke.observe path=/proc/system/bus peer=... endpoint=... path=/ipc/render capacity=64 outcome=ok`
- `bus.smoke.attach peer=... endpoint=... token=... kind=attached outcome=ok`
- `bus.smoke.success peer=... endpoint=... published=10 received=10 token-pub=... token-recv=... payload=hello-qemu outcome=ok`
- `bus.smoke.overflow peer=... endpoint=... errno=Again drain-token=... refill-token=... peak=64 overflows=1 outcome=ok`
- `bus.smoke.detach peer=... endpoint=... token=... outcome=ok`
- `bus.smoke.refusal peer=... endpoint=... errno=Inval outcome=expected`
- `bus.smoke.recovery peer=... endpoint=... attach-token=... publish-token=... receive-token=... bytes=14 payload=recovered-qemu outcome=ok`
- `bus.smoke.state peer=... endpoint=... attached=1 depth=0 publishes=67 receives=67 peak=64 overflows=1 outcome=ok`
- `bus-smoke-ok`
