# NGOS OS Fill Order

This document defines the mandatory execution order for growing `ngos` as an operating system.

It exists to remove ambiguity for humans and LLMs.

The project must not be expanded as a flat feature list.
It must be filled in nested operating-system volumes:

1. cell
2. tissue
3. organ
4. apparatus
5. organism

The rule is simple:

- do not spray partial work across the system
- fill one smaller real volume completely
- then fill the next volume that contains it

## 1. Cell

A cell is the smallest real OS operation that has runtime meaning.

Examples:

- spawn a process
- stop a process
- map a page
- unmap a page
- open a path
- read a file
- write a file
- create a bus endpoint
- attach a contract
- publish an event
- receive an event
- administrate a network link
- watch a network device
- submit a graphics frame
- submit an audio mix
- submit an input batch

A cell is not considered filled unless it has:

- real implementation
- observable runtime effect
- refusal or error behavior when relevant
- recovery, release, or cleanup behavior when relevant

## 2. Tissue

A tissue is a coherent family of cells that serve one local OS function.

Examples:

- process tissue:
  `spawn`, `stop`, `wait`, `inspect`, `status`
- VM tissue:
  `map`, `unmap`, `protect`, `inspect`
- VFS tissue:
  `lookup`, `open`, `read`, `write`, `stat`
- bus tissue:
  `endpoint`, `attach`, `detach`, `publish`, `receive`, `watch`
- network tissue:
  `link`, `admin`, `watch`, `event`, `queue observation`
- compat loader tissue:
  `manifest`, `launch`, `observe`, `refuse`, `relaunch`, `cleanup`

A tissue is filled only when the entire local behavioral family works together end-to-end.

## 3. Organ

An organ is a subsystem.

For `ngos`, the main organs are:

- boot
- platform
- kernel process model
- scheduler
- VM
- VFS
- IPC or bus
- networking
- storage
- graphics
- audio
- input
- compat runtime
- user runtime transport

An organ is not filled because one path works.

It is filled only when all relevant tissues of that subsystem are implemented, integrated, observable, and verifiable.

## 4. Apparatus

An apparatus is a working OS function made from several organs.

Examples:

- execution apparatus:
  `process + scheduler + VM + VFS + user-runtime`
- communication apparatus:
  `bus + event queues + networking + resource contracts`
- session apparatus:
  `process + compat loader + ABI + graphics/audio/input`
- boot apparatus:
  `boot-x86_64 + platform-x86_64 + kernel-core + user-runtime + userland-native`

An apparatus is the first level where the system starts behaving like a living OS flow instead of isolated subsystem proof.

## 5. Organism

The organism is the whole operating system on the real truth path:

- `boot-x86_64`
- `platform-x86_64`
- `kernel-core`
- `user-runtime`
- `userland-native`

The first accepted full-system truth surface is:

- `QEMU`

The final destination is:

- physical hardware

`host-runtime` is an accelerator and validation instrument only.
It is not the organism.

## Current Mapping In NGOS

### Cells already present in meaningful form

- process lifecycle cells
- VM operation cells
- VFS file cells
- bus cells
- shell networking cells
- compat loader and ABI observation cells
- graphics, audio, and input submission cells

### Tissues already being filled well

- `ngos-shell-bus`
- `ngos-shell-network`
- `ngos-shell-proc`
- `ngos-shell-proof`
- `ngos-shell-game`
- `ngos-shell-compat-abi`
- `game-compat-runtime`

These are mostly host-side and userland-side tissues that have been decomposed into smaller semantic owners.

### Organs that are still not closed

- boot on the real path
- platform on the real path
- scheduler as a complete subsystem on the real path
- VM as a complete subsystem
- VFS as a complete subsystem
- networking as a complete subsystem
- storage as a complete subsystem
- process execution apparatus on the real path

## Mandatory Fill Order

The required order from this point forward is:

1. continue shrinking semantic concentration in `userland-native`
2. finish filling tissues into their rightful subsystem owners
3. choose one organ and close it as a real subsystem
4. connect that organ into its apparatus
5. push that apparatus onto the real execution path
6. prove it on `QEMU`
7. only then treat that apparatus as strategically strong

## Strategic OS Order

The priority order for the operating system itself is:

1. boot and platform foundation
2. process plus scheduler plus VM
3. VFS plus process execution
4. IPC or bus plus eventing
5. networking
6. storage
7. graphics, audio, and input
8. compat runtime as a subordinate vertical

This order exists because the first three groups are vital organs of the OS.
Without them, later surfaces remain secondary or synthetic.

## Practical Interpretation

When a contributor or LLM asks "what do we fill next?", the question must be answered in this order:

1. Which cell family is still incomplete inside the current tissue?
2. Is the tissue already complete enough to move to organ-level closure?
3. If the tissue is complete, which organ is the current priority organ?
4. If the organ is complete on host-side proof, has it been pushed to the real path and to `QEMU`?

If the answer to the last question is no, the organ is not yet closed.

## Law For LLM Continuation

Any LLM continuing work in this repository must follow this interpretation:

- do not treat chat history as the only place where strategy lives
- use this document as the canonical fill-order model
- do not jump to a larger organ while the currently chosen organ remains locally fragmented
- do not stop at host-side proof if the corresponding real OS path is still open
- do not mistake tissue closure for organ closure
- do not mistake organ closure on host-side proof for organism closure

## Immediate Repository Direction

Right now the repository is strong in tissue decomposition and still weak in real-path organ closure.

That means the immediate direction is:

1. keep moving residual semantic families out of central membranes
2. pick a vital organ, not another cosmetic tissue
3. close that organ end-to-end
4. push it to `QEMU`

The best current organ candidates are:

- boot plus platform
- process plus scheduler plus VM
- VFS plus process execution

These are the first real basins that matter for `ngos` as an operating system.
