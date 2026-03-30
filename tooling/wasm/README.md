# Local Wasm Tooling

This directory contains the current local WebAssembly tooling used for `ngos`
user-runtime and component work.

## Installed Tools

- `wasmtime` `v43.0.0`
  - official release date: `2026-03-20`
  - local path:
    [tooling/wasm/wasmtime-v43.0.0-x86_64-windows](C:/Users/pocri/OneDrive/Desktop/experiment/tooling/wasm/wasmtime-v43.0.0-x86_64-windows)
- `wasm-tools` `v1.245.1`
  - official release date: `2026-02-12`
  - local path:
    [tooling/wasm/wasm-tools-v1.245.1](C:/Users/pocri/OneDrive/Desktop/experiment/tooling/wasm/wasm-tools-v1.245.1)

## Main Executables

- [wasmtime.exe](C:/Users/pocri/OneDrive/Desktop/experiment/tooling/wasm/wasmtime-v43.0.0-x86_64-windows/wasmtime.exe)
- [wasm-tools.exe](C:/Users/pocri/OneDrive/Desktop/experiment/tooling/wasm/wasm-tools-v1.245.1/wasm-tools-1.245.1-x86_64-windows/wasm-tools.exe)

## Local Launchers

Use the local wrappers instead of relying on global `PATH`:

- [wasmtime.ps1](C:/Users/pocri/OneDrive/Desktop/experiment/tooling/wasm/wasmtime.ps1)
- [wasm-tools.ps1](C:/Users/pocri/OneDrive/Desktop/experiment/tooling/wasm/wasm-tools.ps1)
- [invoke-wasm-tool.ps1](C:/Users/pocri/OneDrive/Desktop/experiment/tooling/wasm/invoke-wasm-tool.ps1)

Examples:

```powershell
.\tooling\wasm\wasmtime.ps1 --version
.\tooling\wasm\wasm-tools.ps1 --version
.\tooling\wasm\invoke-wasm-tool.ps1 wasmtime run .\app.wasm
```

## Download Cache

Original release archives are stored in:

- [tooling/wasm/downloads](C:/Users/pocri/OneDrive/Desktop/experiment/tooling/wasm/downloads)
