<!--
     Copyright 2026

     SPDX-License-Identifier: MIT
-->

# Channel And Region Model

## Notification Edges

- `CNC_V2_CH_JOB_TO_STATE`: `job_ingress -> state_core`
- `CNC_V2_CH_STATE_TO_PLANNER`: `state_core -> planner`
- `CNC_V2_CH_PLANNER_TO_SAFETY`: `planner -> safety_coordinator`
- `CNC_V2_CH_SAFETY_TO_XPORT`: `safety_coordinator -> mcu_transport`
- `CNC_V2_CH_XPORT_TO_STATE`: `mcu_transport -> state_core`
- `CNC_V2_CH_STATE_TO_STORE`: `state_core -> session_store`
- `CNC_V2_CH_STATE_TO_OBS`: `state_core -> observability`
- `CNC_V2_CH_PLANNER_TO_OBS`: `planner -> observability`
- `CNC_V2_CH_SAFETY_TO_OBS`: `safety_coordinator -> observability`
- `CNC_V2_CH_XPORT_TO_OBS`: `mcu_transport -> observability`
- `CNC_V2_CH_STORE_TO_OBS`: `session_store -> observability`

## Shared Regions

- `job_submit`: single writer `job_ingress`, reader `state_core`
- `state_snapshot`: single writer `state_core`, reader `observability`
- `planner_request`: single writer `state_core`, reader `planner`
- `motion_candidate`: single writer `planner`, reader `safety_coordinator`
- `planner_status`: single writer `planner`, reader `observability`
- `safety_decision`: single writer `safety_coordinator`, reader `mcu_transport`
- `safety_status`: single writer `safety_coordinator`, reader `observability`
- `transport_status`: single writer `mcu_transport`, readers `state_core`, `observability`
- `store_request`: single writer `state_core`, reader `session_store`
- `store_status`: single writer `session_store`, reader `observability`
- `observability_report`: single writer `observability`

## sDDF Ownership

- `mcu_transport` is the only app PD with serial-client ownership to the MCU boundary
- `state_core` and `mcu_transport` are the only app PDs with timer-client ownership
- no app PD maps UART MMIO directly
