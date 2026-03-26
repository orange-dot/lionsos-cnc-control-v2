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

Ova linija je vec presla nivo "docs only" scaffolda:

- `include/cnc_v2/` vec zakljucava typed shared-memory i wire contracts
- `meta.py` + `lionsos_cnc_v2.mk` vec generisu buildable Pi-side image za
  `qemu_virt_aarch64` i `rpi3b`
- `components/` vec pokriva uski `job -> state -> planner -> safety ->
  transport -> store/observability` tok
- `mcu/` ostaje sinteticki bare-metal executor; transport hardening i stvarna
  persistence prica ostaju backlog work

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

Prvi kanonski bring-up path za ovu liniju je top-level Pi-side build za
`qemu_virt_aarch64/debug`:

```bash
LIONSOS=/path/to/lionsos MICROKIT_SDK=/path/to/microkit-sdk make
```

Ovaj command contract zamrzava:

- `MICROKIT_BOARD=qemu_virt_aarch64`
- `MICROKIT_CONFIG=debug`
- `BUILD_DIR=build`

Expected primary outputs za taj prvi target su:

- `build/lionsos_cnc_v2.img`
- `build/lionsos_cnc_v2.system`
- `build/report.txt`
- `build/qemu_virt_aarch64.dtb`
- app-side image-i:
  - `build/job_ingress.elf`
  - `build/state_core.elf`
  - `build/planner.elf`
  - `build/safety_coordinator.elf`
  - `build/mcu_transport.elf`
  - `build/session_store.elf`
  - `build/observability.elf`
- generated config/data artefakti za app-side i sDDF wiring, ukljucujuci
  `*.data` blobove za komponente, serial i timer klijente

Parity build za `rpi3b` ostaje:

```bash
LIONSOS=/path/to/lionsos MICROKIT_SDK=/path/to/microkit-sdk MICROKIT_BOARD=rpi3b BUILD_DIR=build-rpi3b make
```

Ovaj slice zamrzava build contract i generated artefakte. Stvarni `qemu`
boot/log i runtime smoke acceptance ostaju sledeci backlog korak.

MCU-side builds use the `mcu/` subtree directly.
