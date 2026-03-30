# Repository Guidelines

## Rule Authority
All rules in this document are mandatory for any agent, LLM, contributor, script, or automation operating in this repository.
These rules are normative, not advisory.
No implicit exceptions, convenient reinterpretations, or local overrides are allowed.
If a local implementation choice conflicts with repository rules, the repository rules prevail.
Any agent working in this repository must treat these rules as project law and follow them in full.

## Project Structure & Module Organization
This repository is a Rust workspace for `Next Gen OS`, an original operating system with its own kernel, ABI, and internal architecture. Core crates live at the top level: `kernel-core` contains kernel logic, `platform-hal` defines platform contracts, `platform-host-runtime` provides the host runtime backend, and `host-runtime` is the main runnable entry point. The active workspace is `ngos`-native only. Use [docs/proprietary-transition.md](C:/Users/pocri/OneDrive/Desktop/experiment/docs/proprietary-transition.md) for origin and ownership policy. Ignore `target/` and temporary `.pdb` outputs.

## Build, Test, and Development Commands
- `cargo run -p ngos-host-runtime`: run the host runtime kernel/runtime.
- `cargo build --workspace`: compile every crate in the workspace.
- `cargo test --workspace`: run all unit tests across crates.
- `cargo test -p ngos-kernel-core <test_name>`: iterate on a single crate or test.
- `cargo fmt --all`: apply standard Rust formatting.
- `cargo clippy --workspace --all-targets -- -D warnings`: catch style and correctness issues before review.

## Coding Style & Naming Conventions
Follow standard Rust style: 4-space indentation, `snake_case` for functions/modules, `CamelCase` for types, and `SCREAMING_SNAKE_CASE` for constants. Match the existing crate naming pattern `ngos-*` in `Cargo.toml`. Prefer small, explicit APIs and keep platform-specific behavior inside the relevant crate instead of leaking it into `kernel-core`. Internal kernel design must be expressed in `ngos` terms, not by copying foreign OS shapes mechanically. New implementation work must be original in code origin, not a translated or mechanically adapted foreign source import.

## Testing Guidelines
Tests are primarily inline `#[cfg(test)]` modules in `src/lib.rs` and `src/main.rs`, so add new unit tests close to the code they verify. Name tests after the behavior under test, for example `scheduler_reports_queue_capacity_exhaustion_explicitly`. Prefer tests that validate real subsystem behavior, invariants, and integration rather than presentation-oriented output. Run `cargo test --workspace` before opening a PR.

## Commit & Pull Request Guidelines
This checkout does not include `.git`, so project-specific commit history is unavailable here. Use short, imperative commit subjects such as `kernel-core: tighten scheduler state checks`. Keep commits scoped to one crate or behavior change when possible. PRs should explain intent, list affected crates, mention any external source or design document consulted, and include test commands run. Add screenshots only for UI or tooling changes with visible output.

## Product Direction
- `ngos` is an original OS, not a Linux or Windows clone.
- `ngos` is not yet fully proprietary in source origin, so all work must move it toward that state, not away from it.
- The active architectural direction for the entire project is `nano-semantic`, not just for the kernel.
- This rule applies to every new or expanded surface: kernel, boot, diagnostics, platform layers, host runtime, user runtime, userland, shell, apps, tools, reports, and internal tooling.
- Large unified surfaces are only acceptable as semantic orchestrators; they must not grow by concentrating opaque responsibility or implicit cross-domain mutation.
- No subsystem is exempt from the nano-semantic rule on the grounds that it is "just tooling", "just shell", "just diagnostics", or "just runtime glue".
- Execution must follow the repository execution contract below.
- No subsystem is exempt from the execution contract.
- Do not add demo-only APIs, reports, binaries, or paths.
- Do not build intentionally minimal subsystem versions that are expected to be discarded later.
- The ban on `mock`, `demo`, `minimal`, `toy`, `showcase`, `sample`, and equivalent disguised forms is absolute and must be treated as project law, not a guideline.
- Do not justify such forms via testing convenience, local validation, bootstrap speed, presentation, temporary scaffolding, or incremental delivery.
- If validation infrastructure is needed, it must remain subordinate to the real implementation and must not define a reduced or symbolic product direction.
- Prefer deepening one real subsystem at a time over scattering effort across unrelated surface-level features.
- Historical compatibility work must remain subordinate to the `ngos` kernel model and must not redefine it.
- There is no project goal to support `macOS`, Apple frameworks, or Apple-specific execution paths.
- No new code may be ported, translated, copied, or mechanically adapted from Linux kernel trees, Windows sources, or other foreign OS sources.
- Foreign-derived crates are migration debt, not expansion targets.
- `host-runtime`, synthetic platforms, and runtime-model validation are auxiliary execution environments only; they are not the product truth surface.
- Any subsystem considered strategically important must ultimately be pushed to the real execution path: `boot-x86_64` + `platform-x86_64` + `kernel-core` + native runtime/userland on real hardware.
- A subsystem is not globally closed if it works only in `host-runtime` or only in synthetic validation paths while the corresponding real-hardware path remains open.
- Host-side validation is allowed only as subordinate proof that accelerates real implementation; it must not become the stopping point, the main acceptance target, or the substitute for real-hardware closure.

## Real Hardware Closure Law

- Real hardware execution is the mandatory product destination for `ngos`.
- `host-runtime`, synthetic backends, emulated paths, and model-only execution are development instruments, not final product truth.
- No subsystem that matters to the operating system may be treated as complete merely because it works in `host-runtime`.
- For any subsystem that has a real boot/platform/device path, closure requires that path to be implemented, integrated, observable, and executable on the real system path.
- The authoritative execution chain is:
  - `boot-x86_64`
  - `platform-x86_64`
  - `kernel-core`
  - `user-runtime`
  - `userland-native`
- If a subsystem works only through host-side runtime validation while the corresponding real execution path is still missing or incomplete, then that subsystem is not closed.
- Host-side proofs may accelerate work, isolate failures, and verify semantics, but they are never sufficient as the final acceptance target for real OS subsystems.
- It is forbidden to present host-validated completion as hardware-real completion.
- When choosing between improving synthetic validation and pushing an already-realistic subsystem onto the actual boot/platform/hardware path, preference must be given to the real-hardware path unless a concrete blocker prevents it.
- `host-runtime` must not become the primary implementation path for subsystem closure.
- If work on `host-runtime` starts consuming the main execution budget while the corresponding real boot/platform/hardware path remains open, that work direction is considered off-course and must be corrected.
- New behavior that is strategically important should be implemented first on the real system path whenever there is a reasonable path to do so.
- The primary subsystem-closure path is `QEMU` and then physical hardware, not `host-runtime`.
- `QEMU` is the first acceptable full-system truth surface for real boot/platform/kernel execution; `host-runtime` is not.
- A subsystem that has not been demonstrated on the `QEMU` path remains open, even if host-side validation is strong.

## Execution Contract

### 1. Forbidden: Micro-Progress

The following are not valid deliveries:

- "added structures"
- "hooked this up"
- "prepared the base"
- "will implement next"
- "can continue with"

Any such state is incomplete and not accepted as repository progress.

### 2. Mandatory: Complete Vertical Front

Any front that is started must be pushed through to an observable, executable, and verifiable end-to-end result.

A front is valid only if it includes all of the following:

- real logic, not stubs
- integration into the existing system
- visible runtime effect
- observability or introspection
- exposure through a relevant interface: CLI, host runtime, syscall surface, or real internal API
- real test or real demonstration

If any of these are missing, the front is not done.

### 3. Global Definition of Done

A subsystem is done only if:

1. it produces real runtime behavior
2. it is integrated into existing flows
3. it can be observed
4. it can be explained causally
5. it can be tested or demonstrated end-to-end

Without all of these, it is not considered implemented.

Done validation is not satisfied by a happy-path-only demonstration.
For any front declared done, validation must also include:

1. a success path
2. a blocking, refusal, or error path when the subsystem can reject or refuse work
3. reversibility or recovery when the subsystem supports restoration, release, or rollback
4. observable exposure of the final state after the flow completes

If only the positive path is demonstrated, the front is only partially closed.

### 3A. Scope Clause: No Abusive Objective Narrowing

When the user or repository law requests closing a subsystem or a large family such as `VM`, `scheduler`, `VFS`, or `networking`, that term is authoritative.

It must not be narrowed unilaterally into:

- "the front worked in this cycle"
- "the current sub-front"
- "the path followed here"
- any equivalent reformulation that reduces the requested scope

Examples:

- "close VM" means close the VM subsystem as a whole, not only `map/unmap/quarantine`
- "close networking" means close networking as a subsystem, not only one socket path
- "close VFS" means close VFS end-to-end, not only `lookup/open`

Done must not be declared while relevant families of the requested subsystem remain open end-to-end.

### 3B. Subsystem Completeness Rule

A subsystem is closed only if every relevant family inside that subsystem has been either:

1. implemented and validated end-to-end, or
2. explicitly declared out of scope by the user before execution

If the user did not exclude a family explicitly, it remains in scope.

### 3C. Forbidden: Local Done Presented as Global Done

The following are not valid substitutes for a user request to close the full subsystem:

- "the front worked now is closed"
- "there is no more gap in this flow"
- "this path is complete"

If the larger subsystem is not complete, the only valid formulation is:

- `Subsystem <name> is not yet closed.`
- `I closed sub-front <x> inside subsystem <y>.`

Any wording that presents local closure as global closure is invalid.

### 3D. Mandatory Continuation Until the Requested Subsystem Is Closed

If relevant families of the same subsystem still remain:

- do not stop
- do not relabel them as "other fronts" merely to end the conversation
- continue execution until the requested subsystem is actually closed

Stopping is valid only if:

- the subsystem is actually closed end-to-end, or
- there is a concrete blocker, demonstrated clearly, that makes continuation impossible at that moment

"I closed what I worked on here" is not a valid stopping condition.

### 3E. Mandatory Reporting Format When a Subsystem Is Not Yet Closed

If the requested subsystem is still open, the response must begin explicitly with:

`Subsystem <name> is not yet closed.`

Then it must enumerate exactly:

- which families are closed
- which families remain open
- what was implemented now

and execution must continue on the remaining families.

### 3F. Anti-Premature-Stop Clause

When the user says "do not stop until you close `<X>`", then:

- every intermediate response is only partial progress
- no intermediate response may contain conclusions such as:
  - "front closed"
  - "there are no more gaps"
  - "it is complete"
  - or equivalent closure wording

unless `<X>` itself is actually fully closed.

For a request such as "close VM", expressions such as:

- "the VM front worked here is closed"
- "the tracked gap is gone"
- "what remains are other VM fronts"

are forbidden and count as direct violations when the VM subsystem is still open.

### 3G. Hard Scope Law

When the user says "close `<X>`", `<X>` means the complete subsystem, not a sub-front chosen by the implementer.

It is forbidden to:

- reduce scope to "the front worked now"
- declare local done as a substitute for global done
- stop while relevant families from the same subsystem still remain
- reclassify the remainder as "other fronts" merely to justify stopping

Absolute rule:

If relevant families inside `<X>` still remain open, the only valid opening is:

`Subsystem <X> is not yet closed.`

Then the response must:

- enumerate what is closed
- enumerate what is still open
- continue execution on what remains

The following expressions are forbidden until `<X>` is fully closed:

- "front closed"
- "there are no more gaps"
- "what remains are other fronts"
- "I closed what I worked on here"

### 3H. Premature Completeness Is Execution Failure

Any response that declares completeness before the entire requested subsystem is actually closed is invalid and must be treated as failed execution.

This remains true even if the technical modifications completed up to that point are good in themselves.

Technical progress may still be real, but the execution is failed if it is presented as subsystem closure before full subsystem completion.

### 4. No Artificial Fragmentation

Work must not be split into tiny slices merely to report progress.

Grouping must be by:

- complete subsystem
- complete runtime flow
- real capability

Prefer one coherent large delivery over many small partial deliveries.

### 5. Autonomous Decisions

Implementation must not stop for routine confirmation.

The responsible agent or contributor must:

- choose implementation order
- refactor when necessary
- connect subsystems together
- resolve local inconsistencies

Blocking is acceptable only for major logical conflict, not ordinary execution detail.

### 6. No Stubs Where Real Implementation Is Reasonable

Do not introduce fake, symbolic, or placeholder variants where there is a reasonable path to real implementation.

If a simplification is chosen, it must:

- be functional
- produce real effect

### 6A. Forbidden: Minimal Implementations That Reopen the Same Work

Do not introduce "minimal", "for now", "temporary", "thin", "bootstrap-only", or equivalent reduced implementations for a subsystem when the real end-to-end path is reasonably implementable.

Absolute rule:

- the same subsystem work must not be re-done repeatedly because an intentionally narrow implementation was chosen first
- do not ship partial execution models that force the same job to be implemented again later in a second or third pass
- do not justify reduced implementations via speed, convenience, local debugging, incremental comfort, or easier testing

When implementation of a front begins, the default requirement is:

- implement it fully
- integrate it into the real system path
- close the success path, refusal/error path, recovery/release path, and final observable state in the same pass

If the implementation is not full end-to-end, then that front is still open and execution must continue immediately on the missing parts.

Any implementation strategy that predictably causes the same front to be rebuilt multiple times is forbidden.

Repository law:

- implement once, full cap-to-cap
- do not implement the same real job 100 times through deliberately reduced intermediate versions

### 7. Strict Reporting

Reporting must describe only:

- the closed front
- the concrete modifications
- the new behavior
- the end-to-end execution
- the real verification
- the real gaps

Speculative "next steps" reporting is forbidden.

## Porting Notes
No new foreign-source porting is authorized. Any future foreign-facing behavior must keep internal design clean and must not dictate core kernel architecture.
The active direction is replacement, not expansion: replace foreign-derived helper crates and utilities with owned `ngos` implementations one subsystem at a time.
Prefer coherent vertical slices that leave the subsystem measurably stronger, better integrated, and better tested at the end of each pass.
When choosing the next front, prefer `kernel-core` subsystem depth or replacement of foreign-derived dependencies over adding thin wrappers or duplicate surfaces.
Do not introduce compatibility behavior that forces `ngos` internals to mirror foreign kernel object models, eventing models, or memory models without a deliberate architectural decision.
