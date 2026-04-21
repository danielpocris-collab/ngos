## Dev Acceleration Benchmark

Generat la: 2026-04-02 00:07:19 +03:00

### Config

- `CARGO_BUILD_JOBS=19`
- benchmark build:
  - `cargo test -p ngos-userland-native -p ngos-boot-x86_64 --lib --no-run`
- benchmark test runner:
  - `cargo test -p ngos-userland-native --lib`
  - `cargo nextest run -p ngos-userland-native --lib`

### Rezultate

#### Build

- baseline fara `sccache`: `16.04s`
- `sccache` cold: `11.47s`
- `sccache` warm: `11.14s`
- castig build warm vs baseline: `30.5%`

#### Test runner

- `cargo test`: `0.14s`
- `cargo nextest`: `0.47s`
- castig `nextest` vs `cargo test`: `-235.7%`

### Observatii

- `sccache` ajuta mult pe build-uri repetate si pe compilare in target dir separat.
- in benchmark-ul asta, `nextest` a fost mai lent decat `cargo test` pentru un singur pachet mic deja compilat.
- `QEMU gdbstub` si `QEMU record/replay` ajuta la timp de debug si reproductibilitate, nu la throughput brut de build/test.

### Artefacte

- [results.json](/C:/Users/pocri/OneDrive/Desktop/experiment/target/bench-dev/results.json)
