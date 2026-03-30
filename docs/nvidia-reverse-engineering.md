# NVIDIA Reverse Engineering Discipline

This document defines how the `ngos` NVIDIA path must evolve while preserving the nano-agent model.

## Rule

Every NVIDIA nano-agent must declare whether its current behavior is:

- `confirmed`: directly observable through PCI config space, runtime-owned state, or validated traces
- `inferred`: reconstructed from observable topology or behavior, but not yet hardware-confirmed
- `experimental`: semantic scaffolding kept isolated until reverse-engineering evidence exists

No agent may present `inferred` or `experimental` behavior as confirmed hardware behavior.

## Current Nano-Agent Status

- `probe`: confirmed
- `vbios`: inferred
- `gsp-control`: inferred
- `vram`: inferred
- `display`: experimental
- `neural`: experimental
- `power`: experimental
- `ray-tracing`: experimental
- `media`: experimental
- `tensor`: experimental

## Required Development Pattern

When extending the NVIDIA driver:

1. Attach every new register, opcode, queue, or capability to one nano-agent.
2. Classify it as `confirmed`, `inferred`, or `experimental`.
3. Keep speculative logic behind explicit nano-agent boundaries.
4. Add at least one test that validates the classification or the observable behavior.
5. Do not describe an agent as reverse-engineered unless the evidence is at least `confirmed`.

## Evidence Expectations

- `confirmed`:
  PCI-visible IDs, BAR layout, directly observed offsets, validated interrupt routes, validated mailbox responses
- `inferred`:
  partially reconstructed command paths, mailbox semantics not yet validated on hardware, provisional allocators
- `experimental`:
  forward-looking semantic agents such as neural rendering, tensor dispatch, media orchestration, or placeholder display control

## Reporting

When reporting progress on the NVIDIA subsystem:

- name the nano-agent
- state the evidence class
- describe the runtime effect
- describe the remaining uncertainty

Do not collapse speculative nano-agents into a claim that the full NVIDIA subsystem is reverse-engineered.
