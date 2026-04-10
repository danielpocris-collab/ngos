# NGOS Architecture Direction

## Authority

This document is normative for `ngos`.
It defines the architectural direction for the operating system as a whole.
It is not limited to one subsystem.

## Core Rule

`ngos` must not evolve into a clone of Unix, Linux, Windows, macOS, FreeBSD, Redox, or any other foreign operating system shape.

`ngos` must be built as its own operating system architecture.

The required direction is:

- object-centric
- capability-aware
- typed
- observable
- reactive
- refusal-first
- recovery-aware
- nano-semantic

These are not optional traits.
They are the expected system model for the full OS.

## System-Wide Scope

This direction applies to all strategically important surfaces:

- `boot-x86_64`
- `platform-x86_64`
- `kernel-core`
- `user-runtime`
- `userland-native`
- `VFS`
- `VM`
- `device runtime`
- `networking`
- `diagnostics`
- `shell`
- internal tools that expose or steer real system behavior

No major subsystem is exempt.

## Object-Centric Rule

`ngos` should treat system resources as explicit objects with lifecycle, state, ownership, and observable transitions.

Examples:

- files
- directories
- mounts
- VM objects
- device queues
- sockets
- contracts
- readiness/watch objects

The system must not depend primarily on anonymous or weakly-typed integer surfaces when a typed object model is reasonable.

## Capability Rule

Access should move toward capability-bearing handles and explicit authority, not path-only or ambient authority.

Path-based access may still exist where needed, but it must not remain the highest-confidence security model for the OS.

The intended direction is:

- authority comes from explicit handles, contracts, or capabilities
- access is tied to object identity and policy
- path lookup is subordinate to object authority, not the final security truth

## Typed Handle Rule

Important kernel and runtime objects should be exposed through typed handles or typed records where reasonable.

The direction is away from weak untyped integer conventions and toward explicit semantic handle classes such as:

- file handles
- directory handles
- mount handles
- VM-backed file mappings
- watch handles
- device request handles

The purpose is stronger correctness, clearer ownership, and safer evolution.

## Observability Rule

Observability is first-class system behavior, not an afterthought.

Every major subsystem should be explainable through explicit runtime state and causal inspection.

This includes:

- who created an object
- who currently holds it
- what state it is in
- what refused and why
- what recovered and how
- what changed over time

If a subsystem is difficult to inspect causally, it is architecturally incomplete.

## Reactive Rule

`ngos` should evolve toward a reactive operating system model.

This means system objects and subsystems should be able to expose state transitions and event streams as first-class behavior.

The direction is not:

- poll everything
- infer changes indirectly
- bolt eventing on as a narrow compatibility feature

The direction is:

- objects emit meaningful changes
- processes may observe those changes through real watch surfaces
- runtime behavior is event-capable, not only call/return oriented

Examples of desired first-class events:

- open
- close
- write
- rename
- unlink
- mount
- unmount
- lock acquired
- lock refused
- permission refused
- recovery completed

## Refusal-First Rule

Operations that cannot be accepted safely should refuse explicitly and observably.

The architecture should prefer:

- strong refusal with clear reason
- preserved invariants
- visible final state

over:

- silent weakening
- implicit fallback
- ambiguous partial mutation

## Recovery Rule

Where release, restoration, rollback, cleanup, or recovery are meaningful, they are part of the subsystem model and not optional extras.

Subsystem design should include:

- refusal paths
- release paths
- recovery paths
- observable terminal state

## Nano-Semantic Rule

Large opaque managers are not the target architecture.

The expected model is:

- semantic agents
- explicit responsibility
- bounded state
- bounded mutation
- composable orchestration

Unified surfaces are acceptable only as semantic orchestrators.
They must not become giant implicit control centers.

## VFS-Specific Implication

The `VFS` direction is not "rebuild a classic Unix VFS with different code".

The target is a mature `ngos` VFS that is:

- object-centric
- capability-aware
- typed
- observable
- reactive
- refusal-first
- recovery-aware

That means classic path operations alone are not sufficient as the final architectural bar.

## Non-Goals

The project direction is not:

- reproducing Unix semantics as the architectural truth
- reproducing Linux internals
- reproducing NT object models mechanically
- reproducing BSD VFS internals mechanically
- wrapping a classic kernel model in semantic naming

## Closure Standard

A subsystem is not architecturally complete merely because it behaves like a familiar legacy subsystem.

For strategically important subsystems, closure requires both:

1. mature runtime behavior
2. alignment with the `ngos` architecture direction in this document

If a subsystem is mature but still fundamentally shaped around foreign legacy assumptions, it is not yet architecturally closed.
