<!--
     Copyright 2026

     SPDX-License-Identifier: MIT
-->

# CNC Control V2 Implementation Master Backlog

## Purpose

This document is the main backlog entrypoint for the first real implementation
program in `lionsos-cnc-control-v2`.

Its role is not to replace the architecture, but to:

- connect the existing runtime cut to the implementation sequence
- extract the canonical backlog tracks
- freeze dependencies and ordering
- prevent work from starting from contradictory runtime or acceptance
  assumptions

## Status

This backlog is currently **active**.

## Reality Check

The current state of the repo is stronger than the "appliance scaffold"
description in the short `README.md`:

- the Pi-side app graph is already named, generated, and buildable
- `meta.py` + `lionsos_cnc_v2.mk` already cover two board paths:
  `qemu_virt_aarch64` and `rpi3b`
- `include/cnc_v2/*.h` already freeze channels, regions, config blob layouts,
  generation helpers, and the wire ABI
- `components/` already carry the narrow
  `job -> plan -> safety -> transport -> observability` path
- `mcu/` already implements a synthetic bare-metal executor with `ACK/NAK/status`
  and heartbeat responses

The following pieces are still missing:

- one first frozen smoke acceptance path above the build contract
- a hardened transport/executor failure model, especially around timeout/retry
  and response classification

The backlog workspace has now been introduced specifically to keep those
remaining steps canonically tied to the actual state of the repo.

## Source Of Truth

The architectural input for this backlog remains:

- `README.md`
- `docs/ARCHITECTURE.md`
- `docs/TOPOLOGY.md`
- `docs/MCU_BOUNDARY.md`
- `docs/CHANNEL_REGION_MODEL.md`
- `docs/SUBSYSTEM_MCU_TRANSPORT.md`
- `docs/SUBSYSTEM_MCU_EXECUTOR.md`
- `include/cnc_v2/*.h`
- `meta.py`
- `lionsos_cnc_v2.mk`
- `Makefile`

## Track Snapshot

- [B10 Runtime Wiring And Contract Freeze](B10_RUNTIME_WIRING_AND_CONTRACT_FREEZE.md) -
  `done`
- [B20 Minimal Bring-Up And Smoke Acceptance](B20_MINIMAL_BRINGUP_AND_SMOKE_ACCEPTANCE.md) -
  `active`
- [B30 MCU Transport And Executor Hardening](B30_MCU_TRANSPORT_AND_EXECUTOR_HARDENING.md) -
  `planned`

## Canonical Sequence

The backlog close-out sequence is canonical:

1. `B10` first closes out what is already actually frozen in docs, contracts,
   `meta.py`, `.mk`, and build outputs
2. `B20` then freezes the first bring-up target, build path, and minimal smoke
   acceptance scenario
3. `B30` only then hardens the wire-level and MCU executor semantics without
   moving authority or topology boundaries

## Cross-Track Invariants

All three tracks must preserve the following at the same time:

- the Pi-side control plane remains `seL4 + Microkit + sDDF + LionsOS`
- the MCU remains a bare-metal executor, not a "second seL4 node"
- `mcu_transport` remains the only app protection domain with serial-client
  ownership
- `state_core` remains the authoritative owner of lifecycle and the canonical
  Pi-side machine-state model
- the forbidden shortcuts from `docs/TOPOLOGY.md` remain forbidden
- there is no app-side direct UART MMIO ownership
- session persistence and broader host UX must not be introduced through the
  transport backlog

## Blocker Table

Current blockers by track:

- `B10`
  - there are no active contract/topology blockers
  - the backlog remains the canonical close-out of already existing decisions
- `B20`
  - the first canonical bring-up target and build contract are now frozen
  - `README.md` does not yet carry the full smoke status of the repo
  - there is no backlog-extracted smoke scenario with explicit subsystem-level
    success signals
- `B30`
  - there is no retransmit or bounded retry policy
  - a valid RX frame currently ends the wait state automatically and sets
    `RESPONSE_OK` without full response classification
  - heartbeat and status/position responses are not yet backlog-locked as
    separate semantic classes

## Exit Criteria For This Master Backlog

The master backlog is closed when all of the following are true at the same
time:

- `B10`, `B20`, and `B30` have `done` status
- there is one canonical build and smoke path for the first runtime cut
- transport failure semantics are explicit and reviewable
- subsequent work can be split into smaller feature backlogs or AI-ready
  slices without returning to the basic runtime/acceptance dispute

## Slice Extraction Rule

This document is not implemented directly.

Before each coding step, it is necessary to:

1. choose one child backlog
2. extract a small slice brief
3. freeze the goal, in scope, out of scope, stable boundaries, and acceptance
4. only then hand implementation to the AI

## First Expected Slice Families

The first logical slice families expected to come out of this master backlog
are:

- a `B20` slice for smoke acceptance evidence
- a `B20` slice for the subsystem success-signal model and README alignment
- a `B30` slice for the response-class model
- a `B30` slice for the timeout/retry policy
- a `B30` slice for `state_core`/`observability` alignment after transport
  hardening
