<!--
     Copyright 2026

     SPDX-License-Identifier: MIT
-->

# CNC Control V2 Backlog Workspace

`docs/backlog/` is the canonical home for the implementation backlog of the
`lionsos-cnc-control-v2` line.

The purpose of this folder is not to replace the architectural documents in
`docs/`, but to extract narrowly defined, reviewable backlog slices from them.

## Source Of Truth

The architectural source of truth remains:

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

Backlog documents must not silently change those boundaries. If they uncover a
gap or contradiction, they must surface it as a work item or a human-owned
decision.

## Canonical Documents

- [B00 Implementation Master](B00_IMPLEMENTATION_MASTER.md)
- [B10 Runtime Wiring And Contract Freeze](B10_RUNTIME_WIRING_AND_CONTRACT_FREEZE.md)
- [B20 Minimal Bring-Up And Smoke Acceptance](B20_MINIMAL_BRINGUP_AND_SMOKE_ACCEPTANCE.md)
- [B30 MCU Transport And Executor Hardening](B30_MCU_TRANSPORT_AND_EXECUTOR_HARDENING.md)
- [Backlog Template](BACKLOG_TEMPLATE.md)

## Naming Rules

- large backlog documents use the `Bxx_` prefix
- future smaller AI-ready slice documents use `Sxx_` or `Bxx-Syy_`
- work item identifiers inside backlog documents use the format `B10-001`,
  `B20-004`, `B30-007`

## Status Vocabulary

Use only the following statuses:

- `planned`
- `active`
- `blocked`
- `partial`
- `done`

## Extraction Rule

- implementation should not start directly from the master backlog
- every concrete coding step should be extracted from one child backlog as a
  small AI-ready slice
- each slice must have a goal, in scope, out of scope, stable boundaries,
  dependencies, invariants, deliverables, and acceptance

## Current Active Tracks

The current backlog program is intentionally narrow:

- `B10` closes out and canonizes the already existing contract/wiring runtime
  cut
- `B20` freezes the first canonical bring-up and smoke acceptance path
- `B30` hardens the Pi/MCU wire semantics without widening the authority graph

The sequence is canonical:

1. `B10`
2. `B20`
3. `B30`
