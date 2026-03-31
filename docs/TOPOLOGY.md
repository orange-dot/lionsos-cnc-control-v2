<!--
     Copyright 2026

     SPDX-License-Identifier: MIT
-->

# Topology

## Pi-Side App Graph

- `job_ingress -> state_core`
- `state_core -> planner`
- `planner -> safety_coordinator`
- `safety_coordinator -> mcu_transport`
- `mcu_transport -> state_core`
- `state_core -> session_store`
- `state_core -> observability`
- `planner -> observability`
- `safety_coordinator -> observability`
- `mcu_transport -> observability`
- `session_store -> observability`

## sDDF Subsystems

- serial driver + virt tx/rx
- timer driver

## Forbidden Shortcuts

- no `job_ingress -> mcu_transport`
- no `planner -> mcu_transport`
- no `observability -> control` command path
- no app-side direct UART MMIO ownership
