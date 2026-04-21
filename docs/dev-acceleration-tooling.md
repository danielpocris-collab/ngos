## Dev Acceleration Tooling

Acest document fixează integrarea celor 6 unelte care chiar accelerează dezvoltarea `ngos` fără să schimbe regula de bază:

- implementarea rămâne pe path-ul real
- `QEMU` rămâne truth surface-ul activ
- `host-runtime` nu devine a doua versiune a produsului

### Ordinea de valoare practică

1. `sccache`
2. `cargo-nextest`
3. `QEMU gdbstub`
4. `QEMU record/replay`
5. `cargo-llvm-cov`
6. `Miri`

### Ce rol are fiecare

#### 1. `sccache`

Scurtează build-urile Rust repetate prin cache de compilare.

Integrare în repo:

- [tooling/x86_64/enable-dev-acceleration.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/enable-dev-acceleration.ps1)
- [tooling/x86_64/build-limine-uefi.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/build-limine-uefi.ps1)

Regulă:

- dacă `sccache` există în `PATH`, scripturile îl activează prin `RUSTC_WRAPPER`
- dacă nu există, build-ul continuă normal

#### 2. `cargo-nextest`

Scurtează execuția suitelor mari de test și oferă control mai bun pe profile de rulare.

Integrare în repo:

- [.config/nextest.toml](/C:/Users/pocri/OneDrive/Desktop/experiment/.config/nextest.toml)
- [tooling/x86_64/run-nextest-workspace.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/run-nextest-workspace.ps1)

Regulă:

- `nextest` este runner-ul rapid pentru workspace tests
- nu înlocuiește proof-urile `QEMU`, doar accelerează testele locale

#### 3. `QEMU gdbstub`

Dă debug real pe path-ul de boot/kernel, nu pe o simulare separată.

Integrare în repo:

- [tooling/x86_64/run-qemu-limine-uefi-gdb.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/run-qemu-limine-uefi-gdb.ps1)

Regulă:

- pornește `QEMU` cu `-gdb tcp::<port>` și `-S`
- VM-ul așteaptă debugger-ul înainte să execute guest-ul

#### 4. `QEMU record/replay`

Ajută la buguri nedeterministe pe path-ul real.

Integrare în repo:

- [tooling/x86_64/run-qemu-limine-uefi-replay.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/run-qemu-limine-uefi-replay.ps1)

Regulă:

- `record` produce:
  - overlay pentru imaginea de boot
  - fișier de replay
- `replay` rerulează exact scenariul înregistrat

#### 5. `cargo-llvm-cov`

Arată ce părți din cod nu sunt lovite de teste.

Integrare curentă:

- detectat de [tooling/x86_64/enable-dev-acceleration.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/enable-dev-acceleration.ps1)
- wrapper direct:
  - [tooling/x86_64/run-llvm-cov-workspace.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/run-llvm-cov-workspace.ps1)

Rol:

- disciplină pe coverage
- nu este truth surface și nu înlocuiește proof-urile `QEMU`

#### 6. `Miri`

Ajută la detectarea bugurilor Rust de memorie și UB în componente izolate care pot fi rulate acolo.

Integrare curentă:

- detectat de [tooling/x86_64/enable-dev-acceleration.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/enable-dev-acceleration.ps1)
- wrapper direct:
  - [tooling/x86_64/run-miri-package.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/run-miri-package.ps1)

Rol:

- verificare adâncă pe bucăți Rust izolate
- nu este unealtă de closure pentru subsisteme reale de boot/platform/kernel

### Fluxul recomandat

1. rulezi [tooling/x86_64/enable-dev-acceleration.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/enable-dev-acceleration.ps1)
2. pentru testare rapidă folosești [tooling/x86_64/run-nextest-workspace.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/run-nextest-workspace.ps1)
3. pentru debug real folosești [tooling/x86_64/run-qemu-limine-uefi-gdb.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/run-qemu-limine-uefi-gdb.ps1)
4. pentru buguri greu de reprodus folosești [tooling/x86_64/run-qemu-limine-uefi-replay.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/run-qemu-limine-uefi-replay.ps1)
5. pentru coverage folosești [tooling/x86_64/run-llvm-cov-workspace.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/run-llvm-cov-workspace.ps1)
6. pentru verificări Rust adânci pe componente izolate folosești [tooling/x86_64/run-miri-package.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/run-miri-package.ps1)
7. closure rămâne pe proof-urile `QEMU` deja existente

### Limită explicită

Niciuna dintre aceste unelte nu schimbă regula repo-ului:

- implementarea strategică rămâne pe path-ul real
- `QEMU` rămâne truth surface-ul activ
- uneltele de accelerare există ca să scurteze build, test și debug, nu ca să creeze o a doua versiune a subsistemelor
