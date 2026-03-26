<!--
     Copyright 2026

     SPDX-License-Identifier: MIT
-->

# RPI3B UART HW Evidence — 2026-03-26

## Purpose

Record the current Raspberry Pi 3B hardware evidence for the `fat-shark`
direction before the AVR/gimbal recut lands.

This note is intentionally narrow:

- current SD image already flashed
- no MCU attached on the Pi/MCU UART boundary
- only Pi TX visibility was tested

## Image Under Test

This hardware pass now has two Pi-side phases.

Initial image used earlier in the day:

- `/home/dev/sel4/lionsos-cnc-control-v2/build-rpi3b/lionsos_cnc_v2.img`

That initial image behaved as follows:

- `job_ingress` injects a synthetic boot job in
  [job_ingress.c](/home/dev/sel4/lionsos-cnc-control-v2/components/job_ingress/job_ingress.c#L19)
- that job becomes a single boot `LINEAR_MOVE` frame
- `mcu_transport` submits the frame on UART and then waits once for a response in
  [mcu_transport.c](/home/dev/sel4/lionsos-cnc-control-v2/components/mcu_transport/mcu_transport.c#L237)
- transport timeout is `250 ms` in
  [runtime_topology.h](/home/dev/sel4/lionsos-cnc-control-v2/include/cnc_v2/runtime_topology.h#L13)
- current MCU-side baud contract is `115200` in
  [config.h](/home/dev/sel4/lionsos-cnc-control-v2/mcu/include/cnc_mcu_v2/config.h#L9)

Expected boot-time command frame from the current image:

```text
55 00 01 b0 04 58 02 00 00 f4 01 8f
```

Later in the same session, a Pi-side bench scope mode was added and built as:

- `/home/dev/sel4/lionsos-cnc-control-v2/build-rpi3b-uart-scope/lionsos_cnc_v2.img`

That mode is controlled by:

- [Makefile](/home/dev/sel4/lionsos-cnc-control-v2/Makefile#L17)
- [lionsos_cnc_v2.mk](/home/dev/sel4/lionsos-cnc-control-v2/lionsos_cnc_v2.mk#L12)
- [mcu_transport.c](/home/dev/sel4/lionsos-cnc-control-v2/components/mcu_transport/mcu_transport.c#L19)

When `CNC_V2_UART_SCOPE_TEST=1`, `mcu_transport` continuously emits:

```text
LIONSOS-V2-UART-SCOPE 0123456789 ABCDEF\r\n
```

That scope image was copied onto the mounted SD card as:

- `/run/media/dev/RPICNC/loader.img`

Copy verification:

- local and SD `sha256` matched:
  `67bd70b0ef87a99b1e3237e35820954f6ba0b81c87f8235256964c9c331261ba`

## Measurement Setup

Bench capture setup used on 2026-03-26:

- scope: OWON VDS1022
- probe: `CH1` only
- signal point: Raspberry Pi 3B header `pin 8` = `GPIO14 / TXD`
- ground: Raspberry Pi 3B header `pin 6` = `GND`
- signal type: `3.3V TTL UART`

No MCU was attached during this pass.

## Captures

Raw bench artifacts now live in the vendored OWON helper pack:

- `helpers/owon-native/rpi3b-live-boot-meta.json`
- `helpers/owon-native/rpi3b-live-boot-debug.json`
- `helpers/owon-native/rpi3b-live-boot-ch1.bin`
- `helpers/owon-native/rpi3b-live-boot-2-meta.json`
- `helpers/owon-native/rpi3b-live-boot-2-debug.json`
- `helpers/owon-native/rpi3b-live-boot-2-ch1.bin`
- `helpers/owon-native/rpi3b-live-boot-3-meta.json`
- `helpers/owon-native/rpi3b-live-boot-3-debug.json`
- `helpers/owon-native/rpi3b-live-boot-3-ch1.bin`
- `helpers/owon-native/rpi3b-scope-continuous-3-meta.json`
- `helpers/owon-native/rpi3b-scope-continuous-3-debug.json`
- `helpers/owon-native/rpi3b-scope-continuous-3-ch1.bin`
- `helpers/owon-native/rpi3b-scope-vr7-meta.json`
- `helpers/owon-native/rpi3b-scope-vr7-debug.json`
- `helpers/owon-native/rpi3b-scope-vr7-ch1.bin`
- `helpers/owon-native/rpi3b-scope-vr6-meta.json`
- `helpers/owon-native/rpi3b-scope-vr6-debug.json`
- `helpers/owon-native/rpi3b-scope-vr6-ch1.bin`

Companion bench note:

- `helpers/owon-native/SCOPE_UART_SESSION_2026-03-26.md`

## What Was Confirmed

This session did confirm real hardware progress:

- the current `rpi3b` image produces measurable activity on the Pi TX pin
- the activity is repeatably capturable with the OWON bench path
- the system is no longer in a "maybe Pi TX never drives" state
- the new scope image produces sustained TX activity, so later bench work no
  longer depends on reboot timing
- the current best OWON CLI preset is now known

This is the strongest honest statement we can make from the session:

- current hardware evidence supports that the Pi-side image reaches the UART
  transport boundary and drives `GPIO14/TX`

## What Was Not Yet Confirmed

This session did **not** close the stronger claim:

- we do not yet have one clean decoded `115200 8N1` frame that can be trusted as
  the exact expected `55 00 01 ... 8f` boot packet

Observed decodes showed line activity but still included framing errors or
garbled bytes, even after the continuous scope image landed.

The best current bench result is from the continuous-image sweep with
`volt_range_index 7`:

- capture artifact set:
  `helpers/owon-native/rpi3b-scope-vr7-*`
- signed sample range:
  `-28..9`
- automatic decode:
  `17` frames with `3` framing errors

## Practical Conclusion

For the `fat-shark` branch, this note should be treated as:

- positive Pi-side hardware evidence
- not yet full UART protocol acceptance evidence

In other words:

- `Pi TX activity present`: yes
- `continuous Pi TX bench mode present`: yes
- `clean boot frame decode closed`: not yet
- `Pi/MCU wire boundary fully validated on hardware`: not yet

## Next Recommended Step

Keep this evidence as the baseline and continue with one of these next:

- keep using the continuous Pi-side scope image for bench work
- keep using the current best OWON preset:
  `volt_range_index 7`, `timebase_prescaler 40`,
  `pre_trigger 512`, `post_trigger 4588`,
  `holdoff 0x8002`, `edge_level 0xf600`, `freqref 0xfb`
- improve capture cleanliness and decoder confidence on that baseline before
  changing Pi-side UART behavior again
