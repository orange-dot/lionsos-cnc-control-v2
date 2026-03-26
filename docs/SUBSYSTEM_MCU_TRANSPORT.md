<!--
     Copyright 2026

     SPDX-License-Identifier: MIT
-->

# MCU Transport

`mcu_transport` is the only app-side component that can translate approved
control frames into raw bytes on the MCU wire.

## Responsibilities

- own the sDDF serial client connection
- convert `safety_decision` snapshots into serial TX
- track wire health, timeout posture, and latest response frame
- publish a single `transport_status` snapshot for the rest of the app

## Non-Responsibilities

- no job parsing
- no safety policy
- no authoritative session state
- no direct observability or store authority beyond its own status snapshot

## Current Skeleton

- TX path is real sDDF serial-client queue usage
- RX path is a minimal byte-stream FSM back into `cnc_frame_t`
- timeout handling is timer-driven and fail-visible
- no retransmit policy yet
