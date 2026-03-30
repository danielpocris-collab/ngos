# Proprietary Transition

## Purpose

This document defines the transition from the current mixed-origin workspace to
an owned `ngos` codebase with original implementation authority.

`ngos` may already have its own kernel direction, ABI, and architecture, but it
is not yet fully proprietary in implementation origin while the workspace still
contains migration-era residue and external semantic debt.

This document is normative.

## Hard Rules

- no new code may be ported, translated, copied, or mechanically adapted from
  Linux kernel sources, Windows sources, or any other foreign OS source tree
- no new workspace crate may be introduced as a foreign-derived helper layer
- compatibility behavior may be reimplemented from owned `ngos` models, but
  foreign source shape must not be imported as implementation material
- all future retained kernel/runtime code must be original in implementation,
  even when its semantics overlap with historical systems
- if an existing foreign-derived implementation is touched for correctness or
  stability, that does not re-authorize new foreign-derived expansion around it

## Current Non-Proprietary Blockers

### 1. Active workspace dependence on derived helper crates

This blocker has been cleared for the active kernel/runtime workspace:

- [kernel-core/Cargo.toml](C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/Cargo.toml)
  no longer depends on `ngos-kern-util-rs`
- historical derived helper crates are no longer workspace members and are no
  longer part of the active build

This means the active kernel/runtime workspace is now isolated from historical
derived helper crates.

### 2. Migration residue in naming and policy

The repository still contains migration-oriented wording, ownership cleanup
tasks, and compatibility debt that prevent a fully proprietary claim.

## Transition Goal

The target state is:

- original kernel implementation
- original support libraries used by the kernel/runtime
- compatibility adapters implemented from owned `ngos` models
- historical source analysis retained only as archival audit, not as active
  development input

## Replacement Order

The replacement order should be aggressive and practical.

### Phase 1: stop new foreign expansion

- freeze all new foreign OS source porting
- route new subsystem work into `kernel-core`, `platform-*`, `user-*`, and
  original `ngos` support code only

### Phase 2: replace helper foundations

Build original replacements for any remaining foreign-shaped support code in
this order:

1. buffer/range/queue primitives now provided by `ngos-core-util`
2. trie / task queue / sleep queue / uio / scatter-gather utilities now
   provided by `ngos-core-util`
3. string, parsing, hashing, and libc-like helper routines that still require
   ownership cleanup

These replacements should land under owned `ngos` naming and ownership, not as
renamed mirrors of the existing crates.

### Phase 3: cut kernel dependency on derived crates

- completed: [kernel-core/Cargo.toml](C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core/Cargo.toml)
  no longer depends on `ngos-kern-util-rs`
- completed: all active kernel users were migrated to owned replacements in
  [ngos-core-util](C:/Users/pocri/OneDrive/Desktop/experiment/ngos-core-util)
- completed: historical derived helper crates were removed from the active
  workspace
- next: delete or replace any remaining migration-era residue once no audit
  value remains

### Phase 4: remove migration inventory

When no active crate depends on foreign-derived implementation:

- delete stale migration inventory and analysis files
- stop carrying cleanup-era naming that no longer reflects the active repo

## Owned Replacement Criteria

A replacement counts as proprietary only if:

- implementation is authored directly in `ngos`
- no line-by-line or structure-preserving translation from foreign code is used
- interfaces, invariants, tests, and naming are expressed in `ngos` terms
- the replacement can stand on its own without the old foreign-derived version

Semantic overlap is acceptable.
Implementation dependence is not.

## Immediate Working Rules

Until the transition is complete:

- prefer extending owned `ngos` code over touching foreign-derived crates
- if a required primitive exists only in a foreign-derived crate, record that
  dependency as migration debt
- do not market or describe the repository as fully proprietary yet
- do describe the repository as moving toward a fully proprietary
  implementation base

## Immediate Audit Summary

As of this transition document:

- kernel architecture: owned direction
- workspace implementation origin: mixed
- helper utility origin: active kernel/runtime helper layer now owned
- compatibility layers: acceptable as adapters, but not as architectural
  authorities

The correct current statement is:

`ngos` is architecturally original, but not yet fully proprietary in source
origin.
