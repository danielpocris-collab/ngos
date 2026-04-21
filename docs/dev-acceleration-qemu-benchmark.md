## QEMU Path Acceleration Benchmark

Generat la: 2026-04-02 00:58:38 +03:00

### Config

- CARGO_BUILD_JOBS=19
- benchmark build real:
  - 	ooling/x86_64/build-limine-uefi.ps1
- benchmark proof real:
  - 	ooling/x86_64/prove-qemu-compat-gfx-smoke.ps1

### Rezultate

#### Build Limine UEFI

- baseline fara sccache: 20.26s
- sccache cold: 20.63s
- sccache warm: 20.64s
- castig warm vs baseline: -1.9%

#### QEMU Proof

- baseline fara sccache: 41.02s
- baseline status: True
- baseline error: 
- sccache cold: 41s
- sccache cold status: True
- sccache cold error: 
- sccache warm: 38.96s
- sccache warm status: True
- sccache warm error: 
- castig warm vs baseline: 5%

### Observatii

- pe build-ul real, sccache reduce timpul total de compilare daca rebuild-ul porneste de la zero.
- pe proof-ul real, castigul este mai mic decat pe build pur, pentru ca o parte din timp este consumata de QEMU, boot si verificarea logurilor.
- daca artefactul exact de bootloader Limine lipseste, scriptul raporteaza blocajul explicit in loc sa opreasca tot benchmark-ul.
- QEMU gdbstub si QEMU record/replay ajuta la viteza de debug, nu la throughput brut; aici ele nu sunt incluse in procentul de accelerare.

### Artefacte

- [results.json](/C:/Users/pocri/OneDrive/Desktop/experiment/target/bench-qemu/results.json)
