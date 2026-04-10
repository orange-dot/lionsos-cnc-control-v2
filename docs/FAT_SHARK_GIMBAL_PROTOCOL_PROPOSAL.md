<!--
     Copyright 2026

     SPDX-License-Identifier: MIT
-->

# Fat-Shark Gimbal Protocol Proposal

## Status

This document is a branch-specific protocol proposal for `fat-shark`.

It is not yet the implemented system truth.

Its purpose is to define the smallest honest replacement for the current
`CNC`-named Pi-to-AVR command semantics while preserving the already useful
UART framing shape and Pi/MCU boundary split.

## Problem

The current tree still describes the MCU executor as a `CNC` machine:

- `LINEAR_MOVE`
- `HOME`
- `SPINDLE`
- `POSITION`
- `machine_state`

That model is no longer honest for the `fat-shark` direction.

The physical executor is still the AVR board, but the controlled plant is now a
three-channel pan-tilt-roll style gimbal path driven through the existing
three-axis MCU board outputs.

So the boundary does not need to move, but the command language does.

## Boundary Decision

The boundary for `fat-shark` stays:

- Pi side owns policy, safety, desired pose, timeout policy, and observability
- AVR side owns electrical output generation, watchdog expiry, neutral/failsafe
  posture, and the final low-level actuation path

The Pi must not claim direct motor or servo ownership.

The AVR must not pretend it measures true pose unless there is real sensor
feedback.

That means the protocol must describe:

- desired setpoints
- executor posture
- executor faults
- watchdog/failsafe state

It must not describe:

- fake CNC coordinates
- fake homing semantics
- fake measured position when only commanded output is known

## Design Goals

- keep the current fixed-width frame discipline
- keep CRC and sequence numbering
- keep one narrow Pi-to-AVR command path
- make the wire contract honest for a three-axis gimbal executor
- avoid committing to feedback semantics that do not exist yet
- leave room for later closed-loop or sensor-backed evolution

## Kept From The Existing Protocol

The following transport-level properties remain good and should be preserved:

- `12`-byte fixed frame
- `0x55` command sentinel
- `0xAA` response sentinel
- `8`-byte payload
- `CRC-8`
- sequence byte
- explicit `ACK`, `NAK`, `STATUS`, and `HEARTBEAT` classes

Keeping those properties reduces churn in:

- AVR UART ISR/FSM code
- Pi-side serial transport code
- test capture workflow
- debugability on real hardware

## Replaced Semantic Model

The `fat-shark` branch should replace:

- `LINEAR_MOVE` with `SET_POSE`
- `HOME` with `CENTER`
- `POSITION` with `POSE_STATUS`
- `machine_state` with `executor_state`

`ESTOP`, `STATUS_QUERY`, `ACK`, `NAK`, and `HEARTBEAT` remain conceptually
valid, but their payload meaning changes to the gimbal domain.

## Axis Model

The executor controls three independent output channels:

- `PAN`
- `TILT`
- `ROLL`

The proposal assumes there is no trustworthy measured pose yet.

So protocol setpoints and status values refer to:

- requested normalized axis targets
- last applied normalized axis targets

They do not claim:

- encoder position
- IMU-stabilized pose
- mechanically verified axis angle

## Units

Axis commands use normalized signed setpoints:

- `-1000` = full negative travel intent
- `0` = neutral / centered intent
- `+1000` = full positive travel intent

Why normalized units:

- the Pi side stays independent of exact pulse widths
- the AVR keeps ownership of electrical pulse mapping
- trims, inversion, deadband, and pulse limits stay local to the executor

The AVR maps normalized values into board-specific output timing or duty.

## Command Frame Layout

All frames remain:

```text
sentinel | seq | type | payload[8] | crc
```

## Command Types

### `GMB_CMD_SET_POSE = 0x01`

Purpose:

- set one or more axis targets
- arm or refresh the executor watchdog window

Payload layout:

```c
typedef struct {
    int16_t pan;
    int16_t tilt;
    int16_t roll;
    uint8_t valid_mask;
    uint8_t ttl_20ms;
} gmb_pay_set_pose_t;
```

Meaning:

- `pan`, `tilt`, `roll` are normalized target values in `[-1000, 1000]`
- `valid_mask bit 0` = apply `pan`
- `valid_mask bit 1` = apply `tilt`
- `valid_mask bit 2` = apply `roll`
- `ttl_20ms` is the executor hold window in `20 ms` units

Rules:

- `ttl_20ms == 0` is invalid and must be `NAK`ed
- unset axes in `valid_mask` keep their previous applied value
- a successful `SET_POSE` re-arms the local AVR watchdog

### `GMB_CMD_CENTER = 0x02`

Purpose:

- drive all three axes to neutral output

Payload:

- all zero

Rules:

- executor applies neutral output on all channels
- executor transitions to `CENTERED`
- executor may keep watchdog armed or clear it; that policy must be fixed in
  implementation docs, but the first cut should prefer clearing the active
  pose hold

### `GMB_CMD_ESTOP = 0x03`

Purpose:

- enter fail-closed emergency state immediately

Payload:

- all zero

Rules:

- executor stops normal pose tracking
- executor forces configured safe output posture
- executor latches `ESTOP` until an explicit recovery command exists

For the first cut, there is no wire-level `CLEAR_ESTOP`.

Recovery can remain power-cycle or reset until intentionally designed.

### `GMB_CMD_STATUS_QUERY = 0x04`

Purpose:

- request immediate executor status publication

Payload:

- all zero

Rules:

- does not change pose state
- does not refresh pose TTL

## Response Types

### `GMB_RSP_POSE_STATUS = 0x81`

Purpose:

- publish executor-owned applied targets and posture

Payload layout:

```c
typedef struct {
    int16_t applied_pan;
    int16_t applied_tilt;
    int16_t applied_roll;
    uint8_t state;
    uint8_t fault_bits;
} gmb_pay_pose_status_t;
```

Important:

- these are last applied targets
- they are not guaranteed measured physical pose

### `GMB_RSP_CMD_ACK = 0x82`

Purpose:

- confirm command acceptance

Payload layout:

```c
typedef struct {
    uint8_t acked_seq;
    uint8_t state;
    uint8_t fault_bits;
    uint8_t applied_mask;
    uint8_t reserved[4];
} gmb_pay_cmd_ack_t;
```

Meaning:

- `acked_seq` confirms the accepted command
- `state` is the resulting executor state after command application
- `fault_bits` gives current latched fault summary
- `applied_mask` tells which axes were actually updated

### `GMB_RSP_CMD_NAK = 0x83`

Purpose:

- reject a command with explicit reason

Payload layout:

```c
typedef struct {
    uint8_t naked_seq;
    uint8_t error;
    uint8_t detail;
    uint8_t state;
    uint8_t fault_bits;
    uint8_t reserved[3];
} gmb_pay_cmd_nak_t;
```

Meaning:

- `error` is the stable class
- `detail` is a small reason code or offending axis bit

### `GMB_RSP_HEARTBEAT = 0x84`

Purpose:

- periodic liveness from the AVR executor

Payload layout:

```c
typedef struct {
    uint32_t uptime_s;
    uint8_t state;
    uint8_t fault_bits;
    uint8_t last_cmd_seq;
    uint8_t ttl_remaining_20ms;
} gmb_pay_heartbeat_t;
```

Meaning:

- `state` and `fault_bits` provide cheap recurring health evidence
- `last_cmd_seq` is the last accepted command sequence
- `ttl_remaining_20ms` exposes whether the watchdog window is draining

## Executor State Model

```c
enum gmb_executor_state {
    GMB_STATE_BOOT = 0u,
    GMB_STATE_CENTERED = 1u,
    GMB_STATE_HOLDING = 2u,
    GMB_STATE_FAILSAFE = 3u,
    GMB_STATE_ESTOP = 4u,
    GMB_STATE_FAULT = 5u,
};
```

Meaning:

- `BOOT`
  initial startup before outputs are fully established
- `CENTERED`
  neutral output posture on all three channels
- `HOLDING`
  actively holding one or more non-neutral targets
- `FAILSAFE`
  watchdog expired, executor returned to safe posture
- `ESTOP`
  explicit emergency stop latched
- `FAULT`
  executor-local configuration or output fault

## Error Classes

```c
enum gmb_error {
    GMB_ERR_BAD_CRC = 0x01u,
    GMB_ERR_BAD_TYPE = 0x02u,
    GMB_ERR_BAD_STATE = 0x03u,
    GMB_ERR_BAD_RANGE = 0x04u,
    GMB_ERR_BAD_MASK = 0x05u,
    GMB_ERR_BAD_TTL = 0x06u,
    GMB_ERR_ESTOP_LATCHED = 0x07u,
};
```

Notes:

- `BAD_RANGE` means one or more axis values exceeded allowed normalized range
- `BAD_MASK` means no valid axis bit was set for `SET_POSE`
- `BAD_TTL` means zero or unsupported TTL
- `ESTOP_LATCHED` means the executor refuses normal commands until recovery

## Fault Bits

```c
enum gmb_fault_bit {
    GMB_FAULT_NONE = 0u,
    GMB_FAULT_WATCHDOG = (1u << 0),
    GMB_FAULT_ESTOP = (1u << 1),
    GMB_FAULT_RANGE_CLAMP = (1u << 2),
    GMB_FAULT_OUTPUT_CONFIG = (1u << 3),
};
```

Intent:

- `WATCHDOG`
  executor entered failsafe because pose refresh expired
- `ESTOP`
  executor is estop latched
- `RANGE_CLAMP`
  executor had to clamp an otherwise accepted target
- `OUTPUT_CONFIG`
  timer/output setup is broken or unavailable

For the first cut, prefer `NAK` over silent clamp for out-of-range commands.

That means `RANGE_CLAMP` may remain unused until a later soft-clamp policy is
introduced.

## Watchdog Policy

The AVR executor owns the final timeout action.

The first honest policy should be:

- `SET_POSE` arms or refreshes watchdog
- if watchdog expires, executor drives neutral output
- watchdog expiry transitions state to `FAILSAFE`
- watchdog expiry sets `GMB_FAULT_WATCHDOG`
- heartbeat continues while in failsafe

The Pi side should treat:

- `FAILSAFE`
- `ESTOP`
- repeated `NAK`

as materially different outcomes.

## Why This Proposal Is Better Than Reusing `CNC`

This proposal keeps the good transport mechanics while deleting false domain
claims.

It removes misleading concepts:

- no fake `HOME`
- no fake spindle
- no fake measured `POSITION`
- no fake machine coordinates

And it replaces them with domain-honest facts:

- desired pose
- applied pose target
- executor state
- watchdog posture
- estop posture

## Pi-Side Mapping

The Pi-side logical path can remain structurally similar:

- ingress
- state owner
- safety/policy gate
- transport
- observability

But the semantic names should move toward:

- `pose_request`
- `pose_command`
- `executor_status`
- `failsafe`

and away from:

- `planner_request`
- `motion_candidate`
- `machine_updated`

The current branch can stage that rename incrementally, but the wire contract
should not keep lying meanwhile.

## AVR-Side Mapping

The AVR executor implementation should own:

- three output channels
- normalized-to-output mapping
- neutral values
- inversion and trims
- watchdog expiration
- estop latch behavior

The AVR executor should not claim:

- true axis angle
- stabilized pose
- closed-loop correction

unless sensors are actually added later.

## Minimal First Slice

The smallest implementation slice that matches this proposal is:

1. new shared wire header for gimbal semantics
2. AVR executor that supports:
   - `SET_POSE`
   - `CENTER`
   - `ESTOP`
   - `STATUS_QUERY`
   - `ACK`
   - `NAK`
   - `POSE_STATUS`
   - `HEARTBEAT`
3. watchdog-to-neutral timeout on AVR
4. Pi-side transport updated for the new frame types
5. one synthetic Pi-side boot pose command for smoke bring-up

## Deferred Work

Explicitly deferred:

- sensor-backed pose feedback
- closed-loop stabilization
- estop clear/recovery command
- per-axis trim configuration over the wire
- multi-command queueing
- retries beyond the current narrow transport model

## Recommended Next Artifacts

After this proposal, the next useful branch-local artifacts are:

- `docs/FAT_SHARK_EXECUTOR_BOUNDARY.md`
- `docs/backlog/B40_FAT_SHARK_GIMBAL_RE_CUT.md`
- shared header replacement for `wire_protocol.h`
- AVR `main.c` recut around the new command set

