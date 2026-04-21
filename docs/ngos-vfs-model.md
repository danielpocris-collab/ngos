# NGOS VFS Model

## Authority

This document defines the intended architectural model of `VFS` in `ngos`.

It is a model document, not a closure-status document.

It explains what `VFS` is supposed to be in `ngos`, what it is not supposed to
be, and which families define it.

## Core Definition

`VFS` in `ngos` is a nano-semantic subsystem for namespace, object lifecycle,
descriptor coherence, refusal, recovery, and observability.

It is not merely a classic Unix-style path dispatch layer.

It is also not defined by similarity to Linux, BSD, FreeBSD, Windows, Redox,
or any other foreign operating system architecture.

## Primary Responsibilities

The `ngos` `VFS` is responsible for:

- path resolution
- mount graph management
- node creation and removal
- rename transitions
- symlink resolution
- metadata and state exposure
- descriptor and object coherence
- permission refusal
- observable inspection surfaces

These responsibilities must be expressed through explicit semantic families,
not through one giant opaque manager.

## Family Model

The authoritative family breakdown for `VFS` is:

- `path_resolution_agent`
- `mount_graph_agent`
- `node_create_agent`
- `node_remove_agent`
- `rename_transition_agent`
- `symlink_resolution_agent`
- `metadata_state_agent`
- `descriptor_vfs_coherence_agent`
- `permission_refusal_agent`
- `vfs_inspection_agent`

This family model is the correct unit of understanding for the subsystem.

## What VFS Is

In `ngos`, `VFS` is:

- a semantic namespace subsystem
- an object lifecycle subsystem
- a descriptor coherence subsystem
- a refusal-first subsystem
- a recovery-aware subsystem
- an observability-capable subsystem

It includes path operations, but it is not reducible to path operations.

It includes descriptor operations, but it is not reducible to weak integer file
descriptor conventions alone.

It includes inspection surfaces, but those are part of the subsystem model, not
optional tooling.

## What VFS Is Not

`VFS` in `ngos` is not:

- a direct Linux VFS analogue
- a BSD VFS clone
- a FreeBSD transplant target
- a path-only access model presented as architectural truth
- a compatibility shell around a foreign kernel object model
- a giant monolithic manager with implicit cross-domain mutation

Classic Unix-style operations such as `open`, `rename`, `unlink`, `stat`,
`readlink`, or `poll` are useful, but they are not by themselves the complete
architectural bar for `ngos VFS`.

## Object-Centric Direction

The intended direction is object-centric.

That means `VFS` should evolve toward explicit objects and semantic records for:

- files
- directories
- mounts
- links
- watch surfaces
- locks
- descriptor-bearing handles

Path lookup may remain important, but it must not remain the only or highest
confidence truth surface.

## Capability-Aware Direction

The intended direction is capability-aware.

That means:

- authority should increasingly attach to explicit handles
- descriptor state and object identity matter more than path text alone
- path resolution is subordinate to object authority where reasonable

The mature form of `ngos VFS` is therefore not a pure path-based system.

## Typed Handle Direction

The intended direction is typed and semantic.

Where reasonable, `VFS` should move toward:

- typed file handles
- typed directory handles
- typed mount handles
- typed watch handles
- typed inspection records

This is preferred over weakly-typed integer-only conventions as the subsystem
matures.

## Observability Direction

Observability is first-class `VFS` behavior.

`VFS` should expose causal and inspectable state for:

- object identity
- descriptor state
- path state
- refusal reasons
- recovery behavior
- lock state
- watch state
- final state after transitions

If a `VFS` flow cannot be explained causally through runtime inspection, it is
architecturally incomplete.

## Reactive Direction

`VFS` should evolve toward reactive object behavior, not only call/return path
mutation.

That includes meaningful event surfaces for:

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

## Refusal And Recovery

Refusal and recovery are part of the subsystem model.

`VFS` closure requires:

- success path
- refusal path
- recovery path where meaningful
- observable final state

Any implementation that only proves happy-path namespace mutation is still
architecturally partial.

## Relationship To Closure

This document does not declare the subsystem closed.

Closure status belongs in:

- [docs/vfs-closure-status.md](/C:/Users/pocri/OneDrive/Desktop/experiment/docs/vfs-closure-status.md)

This document only defines what the subsystem means and what architectural
direction it must follow.

## Comparison Rule

Other systems may be studied for:

- local invariants
- maturity benchmarks
- semantic edge cases
- failure and recovery patterns

Other systems must not define the architectural truth of `ngos VFS`.

Useful comparison is allowed.
Mechanical adoption is not.
