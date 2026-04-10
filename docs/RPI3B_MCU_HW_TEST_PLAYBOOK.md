<!--
     Copyright 2026

     SPDX-License-Identifier: MIT
-->

# RPI3B + MCU Hardware Test Playbook

## Purpose

This playbook defines the first practical board-side test path for
`lionsos-cnc-control-v2` on real `rpi3b + MCU` hardware.

It is intentionally narrow:

- prove that the real `rpi3b` image boots
- prove that Pi-side transport emits the expected boot-demo command
- prove that the MCU executor accepts that command and replies on the wire
- prove that periodic MCU heartbeat traffic exists on real hardware

This playbook does not by itself close the full `B20` backlog. It is a
practical test procedure and evidence checklist for the current runtime cut.

## Scope

In scope:

- `rpi3b` Pi-side image build
- MCU firmware build and flash
- real UART wiring between Pi and MCU
- one boot-demo job emitted by `job_ingress`
- one command/response exchange on the wire
- heartbeat observation after bring-up

Out of scope:

- retransmit or retry behavior
- persistence beyond the in-memory session-store stub
- host-side UX or operator tooling
- extended CNC job sets
- long-duration soak, jitter, or throughput characterization

## Hardware and Preconditions

Required:

- `Raspberry Pi 3 Model B`
- target MCU board supported by the current `mcu/` firmware flow
- stable power for both boards
- working `rpi3b` boot path for local `Microkit` images
- `MICROKIT_SDK` built for `rpi3b`
- serial/UART electrical path between Pi and MCU
- common ground between Pi and MCU

Strongly recommended:

- logic analyzer, oscilloscope, or UART sniffer
- existing serial console path you already use for `rpi3b` bring-up
- a way to reset the MCU immediately before the Pi boot

Assumptions:

- deployment of the built `build-rpi3b/lionsos_cnc_v2.img` onto the board is
  environment-local and already known-good in your setup
- this document does not re-specify Pi firmware, SD-card layout, or bootloader
  provisioning
- if you are using a typical 5V Arduino-class AVR board, the electrical level
  presented to the Pi RX pin must remain 3.3V-safe

## Wiring Notes

Minimum wiring:

- Pi TX -> MCU RX
- MCU TX -> Pi RX
- common ground

If your local `rpi3b` bring-up uses the standard header UART exposure, the
usual pins are:

- Pi TX: `GPIO14`, header `pin 8`
- Pi RX: `GPIO15`, header `pin 10`
- ground: `pin 6`, `pin 9`, or equivalent common ground

Use that only if it matches your existing local `rpi3b` UART routing.

Important:

- do not connect a 5V UART TX directly into the Pi RX pin
- if the MCU board is 5V-only on TX, use a proper level shifter or divider
- treat the protocol UART as binary traffic, not as a human-readable log port

## Preflight

Before the first board attempt, confirm:

- the repo subtree is at the intended commit
- `LIONSOS` points at the local LionsOS tree
- `MICROKIT_SDK` points at the `rpi3b` SDK
- the MCU programmer path is correct for your host
- the Pi and MCU share ground
- the MCU can be reset or power-cycled on demand

## Build Pi Image

Run from the subtree root:

```sh
LIONSOS=/path/to/lionsos \
MICROKIT_SDK=/path/to/microkit-sdk-rpi3b \
MICROKIT_BOARD=rpi3b \
BUILD_DIR=build-rpi3b \
make
```

Expected primary artifact:

- `build-rpi3b/lionsos_cnc_v2.img`

Expected supporting artifacts:

- `build-rpi3b/lionsos_cnc_v2.system`
- `build-rpi3b/report.txt`
- `build-rpi3b/rpi3b.dtb`
- app-side `.elf` images
- generated `.data` blobs for component, serial, and timer wiring

Possible non-blocking note:

- `meta.py` may print an informational line about missing `drivers/gpio`
  without blocking the image build

## Build and Flash MCU Firmware

Build:

```sh
make -C mcu
```

Expected artifacts:

- `mcu/build/cnc-executor-v2.elf`
- `mcu/cnc-executor-v2.hex`

Default flash path:

```sh
make -C mcu flash
```

If your programmer or serial device differs from the default, override
`PROGRAMMER`, for example:

```sh
make -C mcu flash \
  PROGRAMMER='-c arduino -P /dev/ttyUSB0 -b 115200'
```

## Recommended Capture Setup

For the first real hardware cut, wire-level evidence is better than trying to
infer health from logs.

Recommended capture:

- one probe on Pi TX
- one probe on MCU TX
- shared ground on the capture device
- UART decode at `115200 8N1`

If you do not have a logic analyzer, a scope or USB-TTL sniffer is still
useful. Expect binary bytes, not readable text.

## Boot Order

The current transport path has a timeout but no retry policy yet.

That means board order matters:

1. Flash the MCU.
2. Power off both Pi and MCU.
3. Complete UART wiring and capture setup.
4. Reset or power on the MCU so the executor is already alive.
5. Immediately boot the Pi image.

For the cleanest first capture, reset the MCU just before Pi boot. That keeps
the early response sequence numbers deterministic.

If the MCU has been running for a while before the Pi boots, the class of
responses should still be useful, but response sequence and heartbeat uptime
bytes may differ from the exact signatures below.

## Test Sequence

The current image submits one synthetic boot-demo job automatically during
`job_ingress` init.

That job should travel through:

- `job_ingress`
- `state_core`
- `planner`
- `safety_coordinator`
- `mcu_transport`
- MCU executor

No manual operator command is required for this first test.

## Expected Wire Signatures

### Pi to MCU boot-demo command

The current boot-demo job is a `LINEAR_MOVE` with:

- `seq = 0`
- `x = 1200`
- `y = 600`
- `z = 0`
- `feedrate = 500`

Expected full 12-byte command frame:

```text
55 00 01 b0 04 58 02 00 00 f4 01 8f
```

Interpretation:

- `55` = command sentinel
- `00` = command sequence
- `01` = `CNC_CMD_LINEAR_MOVE`

### MCU to Pi ACK

If the MCU receives and accepts that command from a fresh reset, the first
expected response is:

```text
aa 00 83 00 00 00 00 00 00 00 00 3d
```

Interpretation:

- `aa` = response sentinel
- `00` = MCU response sequence
- `83` = `CNC_RSP_CMD_ACK`
- payload byte `0` = acked command sequence `0`

### MCU heartbeat after about one second

From a fresh MCU reset, the next useful liveness signal is typically:

```text
aa 01 85 01 00 00 00 00 00 00 00 57
```

Interpretation:

- `85` = `CNC_RSP_HEARTBEAT`
- payload `01 00 00 00` = uptime about `1` second

If the MCU was not freshly reset, heartbeat sequence and uptime are expected to
vary.

## Acceptance

Treat the board run as accepted when all of these are true:

- `build-rpi3b/lionsos_cnc_v2.img` builds reproducibly
- `mcu/cnc-executor-v2.hex` flashes successfully
- the real Pi image boots on `rpi3b`
- Pi TX emits the expected single boot-demo `LINEAR_MOVE` frame
- MCU TX returns one `ACK` for command sequence `0`
- MCU TX continues to emit heartbeat traffic after bring-up
- there is no obvious wiring-level corruption such as framing collapse or
  persistent garbage on both directions

Stronger acceptance:

- UART capture decodes the exact command and ACK bytes listed above
- the first heartbeat appears at roughly one second after MCU reset

Weaker but still useful acceptance if you lack full UART decode:

- one clear binary burst leaves the Pi shortly after boot
- one clear binary burst returns from the MCU in response
- later periodic MCU bursts continue at roughly one-second cadence

## Failure Interpretation

Use this quick triage:

- no Pi TX burst
  likely Pi image did not boot far enough, deployment is wrong, or transport
  was never reached
- Pi TX burst appears, but MCU TX stays silent
  likely MCU flash, power, wiring, level shifting, or RX path issue
- MCU TX shows heartbeat, but no ACK to the boot move
  MCU is alive, but command ingress or command handling is wrong
- MCU returns `NAK` instead of `ACK`
  likely bad command decode, corrupted frame, or MCU-side bad-state path
- Pi sends one command, then no retry happens
  expected current limitation; the transport does not yet implement retry
- binary garbage appears on a shared serial/log path
  expected if your local board setup still exposes protocol UART traffic on the
  same path you watch for logs

## Evidence to Save

For each real board session, keep:

- exact Pi build command
- exact MCU build and flash command
- image path deployed to the board
- note whether the MCU was freshly reset before boot
- photo or sketch of final wiring
- UART capture screenshot or byte dump if available
- observed command frame
- observed ACK or NAK frame
- whether heartbeat traffic was seen
- short note on pass/fail outcome

This is enough to make the first `rpi3b + MCU` board runs reviewable instead of
purely anecdotal.
