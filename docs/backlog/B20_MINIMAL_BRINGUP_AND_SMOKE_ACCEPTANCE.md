<!--
     Copyright 2026

     SPDX-License-Identifier: MIT
-->

# B20 Minimal Bring-Up And Smoke Acceptance

## Purpose

This backlog freezes the first canonical bring-up and smoke acceptance path for
`lionsos-cnc-control-v2`.

The backlog boundary is intentionally narrow:

- the goal is one reviewable build and acceptance path
- the goal is not yet to do transport retransmit, persistence, or broader host
  tooling

## Status

This backlog is currently **active**.

## Reality Check

The following already exist today:

- `Makefile` as the top-level build entrypoint
- `lionsos_cnc_v2.mk` as the real build/wiring layer
- `meta.py` as the system-description generator
- `build/` and `build-rpi3b/` with already generated image/report artifacts
- the first canonical build contract is now frozen around
  `qemu_virt_aarch64/debug` with `BUILD_DIR=build`
- `rpi3b` remains the parity build target with `BUILD_DIR=build-rpi3b`
- a real app-side command path through `job_ingress`, `state_core`, `planner`,
  `safety_coordinator`, `mcu_transport`, `session_store`, `observability`

The following are still missing:

- one backlog-locked smoke scenario with named success signals
- a fully closed subsystem-level success-signal model
- a real runtime smoke close-out above the build-plus-artifact layer

## Chosen Defaults

The following defaults apply for the first acceptance cut:

- the first canonical bring-up target is `qemu_virt_aarch64` with
  `MICROKIT_CONFIG=debug`
- `rpi3b` remains the parity build target, but not the first mandatory run
  target
- build-plus-generated-artifact evidence is enough to close `B20-001`
- actual `qemu` boot/log evidence is deferred to `B20-002`
- the first smoke scenario covers one accepted job that passes through:
  `state_core -> planner -> safety_coordinator -> mcu_transport ->
  state_core -> observability`

## Definition Of Done

This backlog is done when all of the following are true at the same time:

- there is one canonical build path for the first bring-up target
- there is one minimal smoke scenario with clearly named success signals
- the success signals cover build artifacts, topology artifacts, and
  runtime-facing state/transport/observability effects
- `README.md` and the backlog no longer present different pictures of what the
  "current runtime cut" is

## Canonical Artifacts

- `README.md`
- `Makefile`
- `lionsos_cnc_v2.mk`
- `meta.py`
- `build/report.txt`
- `components/state_core/state_core.c`
- `components/planner/planner.c`
- `components/safety_coordinator/safety_coordinator.c`
- `components/mcu_transport/mcu_transport.c`
- `components/observability/observability.c`

## Dependencies

- `B10` must remain closed and not be reopened

## Acceptance Sequence

The first acceptance path for this backlog needs to settle:

1. which build command path is canonical for the first bring-up target
2. which generated artifacts must exist after the build
3. which minimal job scenario we run or simulate
4. which changes in `state_snapshot`, `transport_status`, and
   `observability_report` count as success
5. which failure signals immediately mean the bring-up cut is not closed

## Work Items

### B20-001 Freeze first bring-up target and build contract

Task:

- freeze `qemu_virt_aarch64` with `MICROKIT_CONFIG=debug` as the first
  canonical bring-up target
- freeze the top-level build command:
  `LIONSOS=/path/to/lionsos MICROKIT_SDK=/path/to/microkit-sdk make`
- freeze the parity build command for `rpi3b`:
  `LIONSOS=/path/to/lionsos MICROKIT_SDK=/path/to/microkit-sdk MICROKIT_BOARD=rpi3b BUILD_DIR=build-rpi3b make`
- freeze the checklist of primary output artifacts for the first target:
  `build/lionsos_cnc_v2.img`, `build/lionsos_cnc_v2.system`,
  `build/report.txt`, `build/qemu_virt_aarch64.dtb`, app-side `.elf` images,
  and generated `.data` wiring blobs

Acceptance:

- a new implementer does not have to guess whether to target
  `qemu_virt_aarch64` or `rpi3b` first, nor which output means the build
  succeeded
- `README.md` and the backlog use the same command contract and the same
  artifact checklist

Status:

- done

### B20-002 Publish minimal smoke scenario

Task:

- describe one narrow smoke scenario that proves the command path is not
  compile-only
- the scenario must cover an accepted job and downstream state/transport
  effects

Acceptance:

- there is one reviewable scenario that does not widen scope to full CNC
  functionality

Status:

- planned

### B20-003 Freeze subsystem-level success signals

Task:

- name what counts as a success signal for:
  - `state_core`
  - `mcu_transport`
  - `observability`
- clearly separate build-artifact success from runtime success

Acceptance:

- bring-up review no longer depends on an unspoken "it seems good enough"

Status:

- planned

### B20-004 Align README with canonical runtime cut

Task:

- extend the repo-level description so it documents the backlog workspace,
  current status, and the first acceptance path

Acceptance:

- `README.md` no longer lags behind the backlog-locked runtime reality

Status:

- partial

## Out Of Scope

- transport retransmit or advanced recovery policy
- session persistence behind the in-memory stub
- new job types, planner features, or host-side tooling
- introducing additional protection domains or changing the authority graph

## Human-Owned Decisions

- whether we close `B20-002` through actual `qemu` boot/log evidence, or
  through another reviewable smoke-evidence layer
- whether `rpi3b` runtime smoke must be part of the same backlog close-out or a
  separate follow-up cut
