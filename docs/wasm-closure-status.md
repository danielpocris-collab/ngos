# WASM Closure Status

`Subsystem WASM is not yet closed.`

## Closed Families

- real `user-runtime` component host for the first approved front
- explicit capability binding for:
  - `observe-process-capability-count`
  - `observe-system-process-count`
  - `observe-process-status-bytes`
  - `observe-process-cwd-root`
- refusal path when a required capability is missing
- recovery path via re-execution with the missing capability granted
- observable `userland-native` front:
  - `wasm.smoke.start`
  - `wasm.smoke.refusal`
  - `wasm.smoke.grants`
  - `wasm.smoke.observe`
  - `wasm.smoke.recovery`
  - `wasm.smoke.result`
  - `wasm-smoke-ok`
- two useful component families:
  - `semantic-observer`
  - `process-identity`
- `QEMU` proof harness:
  - [tooling/x86_64/prove-qemu-wasm-smoke.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/prove-qemu-wasm-smoke.ps1)
  - [tooling/x86_64/verify-qemu-wasm-log.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/verify-qemu-wasm-log.ps1)
- hardware harness prepared:
  - [tooling/x86_64/hardware-wasm-session-com1.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/hardware-wasm-session-com1.ps1)
  - [tooling/x86_64/deploy-limine-uefi-hardware-wasm.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/deploy-limine-uefi-hardware-wasm.ps1)
  - [tooling/x86_64/verify-hardware-wasm-log.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/verify-hardware-wasm-log.ps1)

## Open Families

- broader host interface set above the kernel boundary beyond process semantics
- hardware-real proof of the `WASM` front

## Concrete Front Closed Now

- `WASM` semantic observer and process identity components on the real boot path through:
  - `user-runtime`
  - `userland-native`
  - `boot-x86_64` via `ngos.boot.proof=wasm`

This front is real WebAssembly, not a synthetic host-side substitute.
