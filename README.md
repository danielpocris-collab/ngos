# Next Gen OS

`Next Gen OS` (`ngos`) is an original operating system project with its own kernel, ABI, and internal architecture.

The project is not intended to be a conceptual derivative of Linux, Windows, Android, or any other existing system. External compatibility is treated as a separate layer, never as the internal foundation.

## Direction

- its own kernel and internal model for processes, memory, I/O, security, and observability
- its own native ABI
- an architecture oriented around small semantic subsystems and real vertical slices
- an active transition toward a fully proprietary implementation base

## Principles

- `64-bit only`
- real subsystems, not mocks or symbolic surfaces
- external compatibility does not dictate internal architecture
- internal design is defined in `ngos` terms
- new implementations must be written directly for `ngos`, not mechanically ported from other systems

## Workspace

The current workspace includes the main foundation of the project:

- `kernel-core`
- `platform-hal`
- `platform-host-runtime`
- `platform-x86_64`
- `user-abi`
- `user-runtime`
- `userland-native`

## Running

To run the current runtime:

```bash
cargo run -p ngos-host-runtime
```

To build the entire workspace:

```bash
cargo build --workspace
```

To run tests:

```bash
cargo test --workspace
```

## License and Contributions

This repository is public for visibility, evaluation, and reference. Usage terms are defined in [LICENSE](LICENSE), and contribution terms are defined in [CONTRIBUTING.md](CONTRIBUTING.md).

## Status

Architecturally, `ngos` is original. In terms of full implementation origin, the project is still transitioning toward a fully proprietary codebase.
