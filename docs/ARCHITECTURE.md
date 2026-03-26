<!--
     Copyright 2026

     SPDX-License-Identifier: MIT
-->

# Architecture

`lionsos-cnc-control-v2` is split into two asymmetrical halves:

- Pi-side LionsOS appliance
- MCU-side bare-metal executor

## Pi-Side Responsibilities

- ingress and session/job orchestration
- authoritative control state
- planning and safety coordination
- serial transport ownership through sDDF
- watchdog and cadence ownership through sDDF timer
- observability publication

## MCU-Side Responsibilities

- wire ingress and frame validation
- narrow dispatch of approved commands
- hard-RT execution ownership
- heartbeat and status publication
- fail-closed ESTOP/fault posture

## Design Rule

Use `seL4` concepts where they improve explicit ownership, protocol clarity,
and safety reasoning. Do not import abstractions that lengthen or destabilize
the execution path on the MCU.
