# OWON VDS1022 + RPi3B UART Scope Session — 2026-03-26

## Goal

Capture the current `lionsos-cnc-control-v2` Raspberry Pi 3B image on the Pi TX
line and confirm whether the hardware UART signal is present and decodable.

This session ended up spanning two Pi-side phases:

- initial stock `rpi3b` image with a short boot-time UART burst
- later bench-specific scope image with continuous UART output on the same TX line

## Hardware Setup

- Scope: OWON VDS1022
- Target: Raspberry Pi 3B
- Probe: `CH1` only
- Signal point: Raspberry Pi header `pin 8` = `GPIO14 / TXD`
- Ground: Raspberry Pi header `pin 6` = `GND`
- Probe ratio: `x10`
- `CH2`: unused

Important electrical assumption for this session:

- signal is `3.3V TTL UART`
- no MCU attached on the UART boundary

## Software Setup

Pi-side images used during the session:

- initial hardware image:
  `/home/dev/sel4/lionsos-cnc-control-v2/build-rpi3b/lionsos_cnc_v2.img`
- later scope image:
  `/home/dev/sel4/lionsos-cnc-control-v2/build-rpi3b-uart-scope/lionsos_cnc_v2.img`

Expected Pi-side behavior in the initial phase:

- one boot-time `12`-byte command frame on TX
- `115200 8N1`
- no retry if no MCU response arrives

Expected Pi-side behavior in the later scope phase:

- continuous ASCII pattern on TX
- same physical `GPIO14/TX` line
- no MCU required for sustained signal

OWON/native tooling used:

- `cargo run -p owon-ui`
- `cargo run -p owon-probe -- uart-bench-capture ...`
- `cargo run -p owon-probe -- decode-uart ...`

OWON/native tooling was also improved during this session:

- `owon-probe uart-bench-capture` now accepts bench tuning flags for:
  `--volt-range-index`, `--timebase-prescaler`, `--pre-trigger`,
  `--post-trigger`, `--holdoff`, `--edge-level`, `--freqref`,
  `--rollmode`, `--peakmode`
- those settings are now written into the capture debug JSON and shown in the UI

## Session Artifacts

Artifacts captured in this session:

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

## What Improved

This session is meaningful progress compared to the earlier state:

- the OWON scope is usable both from GUI and from the native CLI path
- we successfully armed repeated live captures from the terminal
- we captured repeatable activity on the Pi TX pin during boot
- this is no longer a "maybe the Pi never drives TX" situation
- we now have a Pi-side continuous UART scope image, so repeated bench work no
  longer depends on reboot timing
- we now have a tunable CLI capture path instead of only fixed bench defaults

## Observed Results

### Capture 1: `rpi3b-live-boot`

- capture completed successfully
- signed sample range: `-8..20`
- automatic decode at `115200` found `1` byte: `0xe8`
- result was not a clean protocol frame

Interpretation:

- real line activity likely present
- not enough to claim clean UART framing yet

### Capture 2: `rpi3b-live-boot-2`

- capture completed successfully
- signed sample range: `-128..18`
- automatic decode at `115200` found `0` frames
- threshold sweep at `0` produced multiple garbage bytes and many framing errors

Interpretation:

- stronger activity was captured than in attempt 1
- waveform still did not decode into a clean UART frame

### Capture 3: `rpi3b-live-boot-3`

- capture completed successfully
- signed sample range: `-21..5`
- automatic decode at `115200` found `11` frames with `7` framing errors
- threshold sweep still did not yield the expected clean boot frame bytes

Best automatic decode result:

```text
Bytes: 02 80 00 00 00 62 60 ff ff ff ff
```

Interpretation:

- there is definitely measurable switching activity on `GPIO14/TX`
- current capture quality is still not good enough for a trustworthy UART decode

### Phase 2: Continuous Scope Image

The Pi-side repo was then extended with a bench-specific transport mode that
continuously emits:

```text
LIONSOS-V2-UART-SCOPE 0123456789 ABCDEF\r\n
```

That scope image was built and flashed to SD, and then bench work continued
without depending on reboot timing.

Two first continuous-image CLI attempts hit transport-side OWON problems:

- one `USB bulk read failed: Overflow`
- one `USB bulk write failed: Operation timed out`

A later retry succeeded and produced `rpi3b-scope-continuous-3-*`.

### Capture 4: `rpi3b-scope-continuous-3`

- capture completed successfully on the continuous-image UART stream
- signed sample range: `-14..6`
- automatic decode at `115200` found `18` frames with `7` framing errors

Best automatic decode result:

```text
Bytes: 20 66 ee fe de ce ee ce 66 ce ff ff ff ff ff ff ff ff
```

Interpretation:

- continuous UART mode materially improved bench usability
- decode was still not clean enough to trust as literal ASCII

### Capture Sweep: `volt_range_index`

Bench sweep performed while the Pi stayed powered and transmitting:

- `volt_range_index 8`
  - usable baseline
  - weaker swing
- `volt_range_index 7`
  - best current result
  - signed sample range: `-28..9`
  - automatic decode found `17` frames with only `3` framing errors
- `volt_range_index 6`
  - too aggressive
  - signed sample range: `-128..25`
  - behavior strongly suggests clipping

Best current capture is `rpi3b-scope-vr7-*`.

Best automatic decode result for `vr7`:

```text
Bytes: 00 00 66 ee fc 9d ef df 9c b9 fe dd ff bf ff ff ff
```

The decode is still not semantically clean, but `vr7` is clearly the best
current bench preset.

## Current Conclusion

This is real progress.

We can now say all of the following:

- the current `rpi3b` hardware image reaches a point where TX activity is visible
- the OWON bench path is operational end-to-end
- the later scope image gives sustained TX activity without reboot timing games
- the remaining problem is signal cleanliness / capture quality / decode quality,
  not basic absence of Pi-side TX activity

What we still cannot claim:

- a clean decoded `115200 8N1` boot frame matching the expected
  `55 00 01 b0 04 58 02 00 00 f4 01 8f`

## Best Current Preset

Best current OWON CLI preset from this session:

```bash
cd /path/to/lionsos-cnc-control-v2/helpers/owon-native
cargo run -p owon-probe -- uart-bench-capture rpi3b-scope-vr7 ch1 \
  --volt-range-index 7 \
  --timebase-prescaler 40 \
  --pre-trigger 512 \
  --post-trigger 4588 \
  --holdoff 0x8002 \
  --edge-level 0xf600 \
  --freqref 0xfb
```

## Most Likely Next Step

Use the continuous Pi-side scope image and the `vr7` OWON preset as the new
baseline:

- keep `CH1` only
- keep the probe on `pin 8`
- keep ground on `pin 6`
- minimize probe ground loop length
- continue tuning from the `vr7` preset instead of changing range blindly
