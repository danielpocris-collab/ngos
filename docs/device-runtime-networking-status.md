# Device Runtime And Networking Status

Subsystem `Device Runtime + Networking` is not yet closed.

## Familii închise

- networking pe calea reală `boot-x86_64 -> user-runtime -> userland-native -> QEMU` pentru:
  - configurare IPv4 a interfeței
  - administrare interfață (`mtu`, capace TX/RX, limită inflight, `admin_up`, `promisc`)
  - `link up/down`
  - `udp bind`
  - `udp connect`
  - `udp sendto`
  - `udp recvfrom`
  - queue driver `/drv/net0`
  - completări TX
  - injectare RX prin frame IPv4/UDP real
  - `inspect_device`
  - `inspect_driver`
  - `inspect_device_request`
  - `inspect_network_interface`
  - `inspect_network_socket`
  - event queues pentru:
    - `link-changed`
    - `rx-ready`
    - `tx-drained`
- refusal și recovery pe aceeași cale reală:
  - refuz la `send_udp_to` când `link` este `down`
  - recovery după `link up`
  - teardown la `unlink` pentru path-ul de socket UDP
  - recovery prin recreare și rebind pe același path de socket
- multi-interface simultan pe aceeași cale reală:
  - `/dev/net0` și `/dev/net1` active în același boot `QEMU`
  - configurare IPv4 independentă pentru fiecare interfață
  - sockete UDP distincte `/run/net0.sock` și `/run/net1.sock`
  - completări TX și refusal `link-down` observabile specific pe `net1`
  - teardown pentru `net0` fără pierderea socketului atașat la `net1`
  - verificare explicită repo-owned în [tooling/x86_64/verify-device-runtime-multi-interface-qemu.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/verify-device-runtime-multi-interface-qemu.ps1)
- dovadă `QEMU` dedicată:
  - [tooling/x86_64/prove-qemu-network-smoke.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/prove-qemu-network-smoke.ps1)
  - [tooling/x86_64/verify-qemu-network-log.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/verify-qemu-network-log.ps1)
  - [target/qemu/serial-network.log](/C:/Users/pocri/OneDrive/Desktop/experiment/target/qemu/serial-network.log)
- lifecycle-ul unificat `device-runtime` pe calea reală `boot-x86_64 -> user-runtime -> userland-native -> QEMU` pentru:
  - graphics
  - audio
  - input
  - networking
  - storage
  - observație semantică unificată prin markerii:
    - `device.runtime.smoke.graphics ...`
    - `device.runtime.smoke.audio ...`
    - `device.runtime.smoke.input ...`
    - `device.runtime.smoke.storage ...`
    - `device-runtime-smoke-ok`
- dovadă `QEMU` dedicată pentru lifecycle-ul unificat:
  - [tooling/x86_64/prove-qemu-device-runtime-smoke.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/prove-qemu-device-runtime-smoke.ps1)
  - [tooling/x86_64/verify-qemu-device-runtime-log.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/verify-qemu-device-runtime-log.ps1)
  - [target/qemu/serial-device-runtime.log](/C:/Users/pocri/OneDrive/Desktop/experiment/target/qemu/serial-device-runtime.log)

## Familii încă deschise

- hardware fizic, când reintră în scope
- extinderi viitoare peste modelul actual de networking și device runtime deja închis pe `QEMU`:
  - routing mai bogat
  - alte familii/protocoale de socket
  - teardown și recovery mai adânci pentru mai multe clase de device

## Ce este implementat acum

- runtime nou de boot pentru rețea:
  - [boot-x86_64/src/boot_network_runtime.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/boot_network_runtime.rs)
- integrare în syscall surface-ul real de boot:
  - [boot-x86_64/src/user_syscall.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/user_syscall.rs)
- activare boot proof:
  - [boot-x86_64/src/user_process.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/user_process.rs)
  - [boot-x86_64/src/lib.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/lib.rs)
  - [boot-x86_64/src/main.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/main.rs)
- smoke real în userland:
  - [userland-native/src/lib.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/userland-native/src/lib.rs)
  - [userland-native/src/surface_agents.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/userland-native/src/surface_agents.rs)
- proof unificat pentru lifecycle de device:
  - [tooling/x86_64/prove-qemu-device-runtime-smoke.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/prove-qemu-device-runtime-smoke.ps1)
  - [tooling/x86_64/verify-qemu-device-runtime-log.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/verify-qemu-device-runtime-log.ps1)
- closure pentru multi-interface pe owner-ul real:
  - [boot-x86_64/src/boot_network_runtime.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/boot_network_runtime.rs)
  - [boot-x86_64/src/user_syscall.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/user_syscall.rs)
  - [ngos-shell-network/src/lib.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/ngos-shell-network/src/lib.rs)
  - [tooling/x86_64/verify-device-runtime-multi-interface-qemu.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/verify-device-runtime-multi-interface-qemu.ps1)
