<!--
     Copyright 2026

     SPDX-License-Identifier: MIT
-->

# MCU Executor

The `mcu/` subtree is a bare-metal executor, not a Microkit or `seL4` node.

## Structure

- `uart.c`: ISR-fed RX FSM and blocking TX
- `main.c`: dispatcher, simple machine state, ACK/NAK/status/heartbeat
- shared wire ABI via `include/cnc_v2/wire_protocol.h`

## Intentional Constraints

- no internal task graph
- no dynamic allocation
- no logging on the protocol UART
- one narrow dispatch path from valid frame to state update and response

## Current Scope

- accepts the same 12-byte command frames as v1
- implements synthetic move/home/set-position/spindle/estop/status behavior
- publishes heartbeat periodically so Pi-side timeout and liveness logic have a concrete counterpart
