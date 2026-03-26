<!--
     Copyright 2026

     SPDX-License-Identifier: MIT
-->

# B30 MCU Transport And Executor Hardening

## Purpose

This backlog hardens the Pi/MCU wire boundary without moving the authority
model or widening the runtime graph.

The backlog boundary is intentionally narrow:

- the goal is for `mcu_transport` and `mcu/` to get a clear failure and
  recovery model
- the goal is not for the MCU to become an OS-like subsystem or for
  planner/safety to gain new scope

## Status

This backlog is currently **planned**.

## Reality Check

The following already exist today:

- a real TX path through the sDDF serial client queue
- an RX byte-stream FSM back into `cnc_frame_t`
- a timeout path that degrades `transport_status`
- a synthetic MCU executor with `ACK/NAK/status/heartbeat` responses

The following are still missing:

- explicit response classification and correlation with the pending command
- a bounded retry or abandon policy
- clear separation of heartbeat/liveness signals from command-completion
  signals
- backlog-locked semantics for how `state_core` and `observability` interpret
  NAK, timeout, and unexpected valid responses

## Chosen Defaults

The following defaults apply for the first hardening cut:

- one in-flight command remains in the transport
- heartbeat does not close the pending-command wait
- a valid but unexpected response must not clear divergence or timeout on its
  own
- the MCU executor remains synthetic and without an internal task graph

## Definition Of Done

This backlog is done when all of the following are true at the same time:

- response classes (`ACK`, `NAK`, `POSITION`, `HEARTBEAT`, invalid/unexpected`)
  have clear semantics
- timeout/retry/abandon rules are explicit and reviewable
- `transport_status` can distinguish local queue fault, remote reject,
  timeout, and protocol fault
- `state_core` and `observability` do not treat all valid RX frames as the
  same "success"
- the MCU executor remains a narrow bare-metal dispatcher

## Canonical Artifacts

- `docs/SUBSYSTEM_MCU_TRANSPORT.md`
- `docs/SUBSYSTEM_MCU_EXECUTOR.md`
- `docs/MCU_BOUNDARY.md`
- `include/cnc_v2/control_ipc.h`
- `include/cnc_v2/wire_protocol.h`
- `components/mcu_transport/mcu_transport.c`
- `components/state_core/state_core.c`
- `components/observability/observability.c`
- `mcu/include/cnc_mcu_v2/protocol.h`
- `mcu/src/main.c`
- `mcu/src/uart.c`

## Dependencies

- `B10` must remain closed
- `B20` must settle the first canonical bring-up and smoke path before more
  aggressive wire-hardening work

## Work Items

### B30-001 Freeze response classification model

Task:

- settle which response types:
  - close the pending command
  - only carry liveness/status information
  - represent a protocol or correlation problem

Acceptance:

- `mcu_transport` no longer treats every valid response as the same
  `RESPONSE_OK`

Status:

- planned

### B30-002 Freeze timeout, retry and abandon policy

Task:

- settle whether the first hardening cut has:
  - zero retry
  - bounded retry
  - or explicit abandon after timeout
- the policy must stay small and reviewable

Acceptance:

- the implementer does not have to invent recovery behavior on a wire fault

Status:

- planned

### B30-003 Harden invalid and unexpected RX handling

Task:

- separate:
  - CRC fault
  - unexpected response class
  - out-of-sequence response
- determine which of those cases change health, lifecycle, or only evidence

Acceptance:

- protocol fault handling is explicit, not an implicit consequence of the
  current helper flow

Status:

- planned

### B30-004 Freeze heartbeat and status-query semantics

Task:

- settle how heartbeat and position/status responses affect:
  - liveness
  - pending command wait
  - machine state publication

Acceptance:

- heartbeat and status responses are no longer "just another valid frame"

Status:

- planned

### B30-005 Align transport consumers after hardening

Task:

- align the `state_core` lifecycle mapping and the `observability`
  health/report model with the new transport semantics

Acceptance:

- downstream consumers do not lose information about whether the problem is a
  remote reject, timeout, CRC fault, or a local queue problem

Status:

- planned

## Out Of Scope

- changes to planner or safety policy scope
- widening the wire-frame format beyond what is needed for the first hardening
  cut
- persistence, session semantics, or host UX
- introducing an MCU-side scheduler, task system, or dynamic allocation

## Human-Owned Decisions

- the exact retry budget for the first hardening cut
- whether the `POSITION` response may close a pending command only for
  `STATUS_QUERY` or also for other commands
- how remote `NAK` maps to the lifecycle/evidence model outside the narrow
  transport layer
