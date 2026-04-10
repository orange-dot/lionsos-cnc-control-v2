<!--
     Copyright 2026

     SPDX-License-Identifier: MIT
-->

# B10 Runtime Wiring And Contract Freeze

## Purpose

This backlog canonizes the currently existing runtime cut:

- named app graph
- channels and shared regions
- typed contracts
- config blob layouts
- generator and `.mk` wiring

The backlog boundary is intentionally narrow:

- the goal is to close out what is already actually frozen
- the goal is not yet to introduce new features, persistence, or transport
  hardening

## Status

This backlog is currently **done**.

## Reality Check

The following pieces are already settled today and are runtime-consistent:

- `docs/TOPOLOGY.md` and `docs/CHANNEL_REGION_MODEL.md` define the same named
  edge set
- `include/cnc_v2/control_ipc.h`, `runtime_topology.h`, and `config.h`
  already freeze payload, channel, and config-blob identities
- `meta.py` already serializes config blobs that match those layouts
- `lionsos_cnc_v2.mk` already wires the same artifacts into real buildable
  images
- `build/` and `build-rpi3b/` already show that the first runtime cut exists
  for both board targets

`README.md` still lags behind that state, but that is a documentation gap, not
a contract blocker.

## Definition Of Done

This backlog is done when all of the following are true at the same time:

- each named edge has a clearly identified:
  - single producer
  - single consumer
  - channel or notify direction
  - shared region
  - typed payload contract
- config blob layouts in `meta.py` and `include/cnc_v2/config.h` are 1:1
- no app protection domain receives broader authority than the topology claims
- the two board paths (`qemu_virt_aarch64`, `rpi3b`) use the same canonical
  service graph

## Canonical Artifacts

- `docs/TOPOLOGY.md`
- `docs/CHANNEL_REGION_MODEL.md`
- `include/cnc_v2/config.h`
- `include/cnc_v2/control_ipc.h`
- `include/cnc_v2/runtime_topology.h`
- `include/cnc_v2/wire_protocol.h`
- `meta.py`
- `lionsos_cnc_v2.mk`
- `build/report.txt`
- `build-rpi3b/report.txt`

## Dependencies

- there are no previous backlog dependencies
- this backlog is the entry gate for `B20` and `B30`

## Alignment Note

The following matrix is the canonical `B10` close-out record for the first
runtime cut.

| Edge | Producer | Consumer | Delivery | Channel | Region | Payload |
| --- | --- | --- | --- | --- | --- | --- |
| `job_ingress -> state_core` | `job_ingress` | `state_core` | notify plus shared mailbox | `CNC_V2_CH_JOB_TO_STATE` | `job_submit` | `cnc_v2_job_submit_t` |
| `state_core -> planner` | `state_core` | `planner` | notify plus shared mailbox | `CNC_V2_CH_STATE_TO_PLANNER` | `planner_request` | `cnc_v2_planner_request_t` |
| `planner -> safety_coordinator` | `planner` | `safety_coordinator` | notify plus shared candidate | `CNC_V2_CH_PLANNER_TO_SAFETY` | `motion_candidate` | `cnc_v2_motion_candidate_t` |
| `safety_coordinator -> mcu_transport` | `safety_coordinator` | `mcu_transport` | notify plus shared decision | `CNC_V2_CH_SAFETY_TO_XPORT` | `safety_decision` | `cnc_v2_safety_decision_t` |
| `mcu_transport -> state_core` | `mcu_transport` | `state_core` | notify plus shared status | `CNC_V2_CH_XPORT_TO_STATE` | `transport_status` | `cnc_v2_transport_status_t` |
| `state_core -> session_store` | `state_core` | `session_store` | notify plus shared request | `CNC_V2_CH_STATE_TO_STORE` | `store_request` | `cnc_v2_store_request_t` |
| `state_core -> observability` | `state_core` | `observability` | notify plus shared snapshot | `CNC_V2_CH_STATE_TO_OBS` | `state_snapshot` | `cnc_v2_state_snapshot_t` |
| `planner -> observability` | `planner` | `observability` | notify plus shared status | `CNC_V2_CH_PLANNER_TO_OBS` | `planner_status` | `cnc_v2_planner_status_t` |
| `safety_coordinator -> observability` | `safety_coordinator` | `observability` | notify plus shared status | `CNC_V2_CH_SAFETY_TO_OBS` | `safety_status` | `cnc_v2_safety_status_t` |
| `mcu_transport -> observability` | `mcu_transport` | `observability` | notify plus shared status | `CNC_V2_CH_XPORT_TO_OBS` | `transport_status` | `cnc_v2_transport_status_t` |
| `session_store -> observability` | `session_store` | `observability` | notify plus shared status | `CNC_V2_CH_STORE_TO_OBS` | `store_status` | `cnc_v2_store_status_t` |

`observability_report` remains a producer-owned output region for
`observability`, not an additional cross-protection-domain command or feedback
edge.

## Work Items

### B10-001 Inventory canonical edge and region matrix

Task:

- close out one `edge -> channel -> region -> payload` matrix

Acceptance:

- the implementer no longer has to guess which artifact carries which payload

Status:

- done

### B10-002 Freeze config blob layout contract

Task:

- confirm that the `meta.py` `struct.pack` layouts match the packed structs in
  `include/cnc_v2/config.h`

Acceptance:

- the config-blob ABI is 1:1 reviewable from the code and generator

Status:

- done

### B10-003 Freeze shared-memory publish/read contract

Task:

- confirm that the generation-based publish/read helpers remain the canonical
  path for shared regions

Acceptance:

- producer/consumer shared-memory semantics are not left to local
  interpretation

Status:

- done

### B10-004 Freeze first board set and build graph

Task:

- confirm that the first runtime cut has the same service graph on
  `qemu_virt_aarch64` and `rpi3b`

Acceptance:

- the build reports for both paths do not carry a different runtime graph

Status:

- done

### B10-005 Publish canonical close-out note

Task:

- keep this backlog as the canonical record of what is already frozen before
  further bring-up and hardening work

Acceptance:

- `B20` and `B30` can start without reopening the basic channel/region model

Status:

- done

## Out Of Scope

- retransmit or bounded retry policy
- `qemu` or board smoke acceptance
- persistence behind the `session_store` stub
- host UX, job format extensions, or planner feature creep

## Human-Owned Decisions

- changes to the canonical service graph or authority model after this close-out
- introducing new app protection domains, new region families, or a wider
  Pi/MCU boundary
