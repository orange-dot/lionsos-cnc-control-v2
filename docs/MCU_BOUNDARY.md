<!--
     Copyright 2026

     SPDX-License-Identifier: MIT
-->

# MCU Boundary

The MCU is not treated as a Microkit PD or a second `seL4` node in this v2
line. It remains a bare-metal executor.

## What Carries Over From seL4 Thinking

- explicit ownership of resources
- narrow authority to convert frames into motion
- typed state machine transitions
- observable divergence and heartbeat posture

## What Does Not Carry Over

- internal IPC decomposition
- scheduler-like service layering
- generic OS abstractions on the execution path
- abstractions whose primary benefit is aesthetic rather than timing-safe
