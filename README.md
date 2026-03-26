<!--
     Copyright 2026

     SPDX-License-Identifier: MIT
-->

# LionsOS CNC Control V2

`lionsos-cnc-control-v2` is a LionsOS-native, Microkit-built CNC control
appliance scaffold.

It is intentionally separate from the existing `lionsos-cnc-control` v1 line.

## Architectural Posture

- Pi-side control plane uses `seL4 + Microkit + sDDF + LionsOS`
- MCU-side execution remains bare-metal and hard-RT
- the Pi/MCU boundary is preserved and treated as a typed protocol contract
- sDDF serial and timer subsystems replace direct app-side UART ownership

## Pi-Side Components

- `job_ingress`
- `state_core`
- `planner`
- `safety_coordinator`
- `mcu_transport`
- `session_store`
- `observability`

## MCU-Side Executor

The `mcu/` subtree contains a v2 executor scaffold that remains bare-metal, but
uses seL4-shaped concepts where they help:

- narrow authority boundaries
- explicit ownership of wire ingress, dispatch, fault state, and telemetry
- typed protocol and heartbeat/status publication

## Documents

- [Architecture](docs/ARCHITECTURE.md)
- [Topology](docs/TOPOLOGY.md)
- [MCU Boundary](docs/MCU_BOUNDARY.md)
- [Channel And Region Model](docs/CHANNEL_REGION_MODEL.md)
- [MCU Transport](docs/SUBSYSTEM_MCU_TRANSPORT.md)
- [MCU Executor](docs/SUBSYSTEM_MCU_EXECUTOR.md)
- [Fat-Shark Gimbal Protocol Proposal](docs/FAT_SHARK_GIMBAL_PROTOCOL_PROPOSAL.md)
- [RPI3B + MCU HW Test Playbook](docs/RPI3B_MCU_HW_TEST_PLAYBOOK.md)
- [RPI3B UART HW Evidence 2026-03-26](docs/RPI3B_UART_HW_EVIDENCE_2026-03-26.md)
- [Helpers Workspace](helpers/README.md)
- [OWON Native Helper Pack](helpers/owon-native/README.md)
- [Backlog Workspace](docs/backlog/README.md)

## Current Status

This line has already moved beyond the "docs only" scaffold stage:

- `include/cnc_v2/` already freezes the typed shared-memory and wire contracts
- `meta.py` + `lionsos_cnc_v2.mk` already generate a buildable Pi-side image
  for `qemu_virt_aarch64` and `rpi3b`
- `components/` already cover the narrow
  `job -> state -> planner -> safety -> transport -> store/observability` path
- `mcu/` remains a synthetic bare-metal executor; transport hardening and the
  real persistence story remain backlog work

## Build Inputs

Pi-side builds expect:

- `LIONSOS=/path/to/lionsos`
- `MICROKIT_SDK=/path/to/microkit-sdk`

Canonical defaults are:

- `MICROKIT_BOARD=qemu_virt_aarch64`
- `MICROKIT_CONFIG=debug`
- `BUILD_DIR=build`

Parity build target remains:

- `MICROKIT_BOARD=rpi3b`
- `BUILD_DIR=build-rpi3b`

## Canonical Bring-Up

The first canonical bring-up path for this line is the top-level Pi-side build
for `qemu_virt_aarch64/debug`:

```bash
LIONSOS=/path/to/lionsos MICROKIT_SDK=/path/to/microkit-sdk make
```

This command contract freezes:

- `MICROKIT_BOARD=qemu_virt_aarch64`
- `MICROKIT_CONFIG=debug`
- `BUILD_DIR=build`

The expected primary outputs for that first target are:

- `build/lionsos_cnc_v2.img`
- `build/lionsos_cnc_v2.system`
- `build/report.txt`
- `build/qemu_virt_aarch64.dtb`
- app-side images:
  - `build/job_ingress.elf`
  - `build/state_core.elf`
  - `build/planner.elf`
  - `build/safety_coordinator.elf`
  - `build/mcu_transport.elf`
  - `build/session_store.elf`
  - `build/observability.elf`
- generated config/data artifacts for app-side and sDDF wiring, including
  component, serial, and timer client `*.data` blobs

The parity build for `rpi3b` remains:

```bash
LIONSOS=/path/to/lionsos MICROKIT_SDK=/path/to/microkit-sdk MICROKIT_BOARD=rpi3b BUILD_DIR=build-rpi3b make
```

This slice freezes the build contract and generated artifacts. Actual `qemu`
boot/log and runtime smoke acceptance remain the next backlog step.

MCU-side builds use the `mcu/` subtree directly.
