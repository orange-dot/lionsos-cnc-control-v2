# OWON VDS1022 + RPi3B UART Scope Session — 2026-03-21

## Goal

Capture UART output from RPi3B GPIO14/TX (pin 8) at 115200 baud using
OWON VDS1022 USB oscilloscope, decode it with owon-probe.

## Hardware Setup

- **Scope:** OWON VDS1022, V2.5, serial VDS10221827805, FPGA V1
- **Target:** Raspberry Pi 3 Model B+ (2017)
- **Probe:** Single probe on CH1 BNC input
- **Signal pin:** GPIO14/TX = pin 8 (inner column, 4th from top)
- **Ground:** Pin 9 or USB shield (various attempts)
- **SD image:** `build-rpi3b-scope/loader.img` with `CNC_UART_SCOPE_TEST=1`
  Later replaced with `build-gpio-scope/loader.img` (UART + GPIO square wave)

## Timeline of Issues Found and Fixed

### Issue 1: Probe on wrong BNC input (SOLVED)

Probe was connected to the **"Multi" BNC connector** on the VDS1022 instead
of **CH1**. Multi is not a scope channel — it's for external trigger or
multimeter. All captures showed swing=3 (pure noise, i8=[-1, +2]).

**Fix:** Moved probe BNC to CH1 input.

**Confirmation:** GPIO4 square wave captured successfully with swing=132,
period_num=236 after moving to CH1.

### Issue 2: Voltage range too sensitive (ALREADY FIXED in code)

`DEFAULT_VOLT_RANGE_INDEX=5` (200mV/div) is too sensitive for 3.3V UART.
A 3.3V signal at that range clips the ADC at both rails (u8=0 and 255),
destroying all signal information.

The code already had the fix: `CaptureRequest::uart_bench()` uses
`volt_range_index: 8` (2V/div = ~16V full screen), appropriate for 3.3V.
This was committed in a prior session.

- Range 5: voltage_gain=550, zero_offset=535
- Range 8: voltage_gain=725, zero_offset=536

### Issue 3: Capture didn't wait for signal (FIXED this session)

The original `capture_uart_bench()` called `capture_with_request()` once,
which grabbed whatever was in the scope buffer — usually noise from before
the Pi booted.

**First fix:** Added retry loop (60 attempts × 200ms) waiting for swing > 50.
This worked but was slow and still returned the first capture with signal,
which was often a transition rather than mid-burst data.

**Final fix:** Changed to rapid-fire 10-second scan (no sleep between
captures). Keeps the capture with the highest swing. Achieved 510 captures
in 10 seconds. Located in `DeviceSession::capture_uart_bench()` in
`crates/owon-device/src/lib.rs`.

### Issue 4: UART scope test not enabled on Pi (was already correct)

The SD image was verified to be `build-rpi3b-scope/loader.img` with
`CNC_UART_SCOPE_TEST=1`. Checksum confirmed:
`d189805656390e4bc3e98cab168c181c`.

## Changes Made

### helpers/owon-native/crates/owon-device/src/lib.rs

1. **`capture_uart_bench()`** — replaced single-shot capture with 10-second
   rapid-fire scan loop. Keeps the capture with highest i8 swing. Trace
   output shows attempt count, current swing, and best swing.

2. **`CaptureRequest`** — already had `volt_range_index` field. `uart_bench()`
   already used index 8.

3. **`CaptureDebugInfo`** — already had `volt_range_index` field.

4. **`DeviceSession`** — already stored `flash_bytes: Vec<u8>` and computed
   `CaptureProfile` on demand per request.

### microkit-cnc-control/cnc_rpi3b.system

Added GPIO MMIO mapping for scope test square wave:

```xml
<memory_region name="gpio_regs" size="0x1000" phys_addr="0x3f200000" />
```

Mapped into mcu_xport PD at vaddr 0x2500000.

### microkit-cnc-control/src/mcu_xport.c

Added GPIO4 (pin 7) square wave generator inside `#if CNC_UART_SCOPE_TEST`:

- GPIO register definitions (GPFSEL0, GPSET0, GPCLR0)
- `gpio_regs` uintptr_t (patched by microkit tool)
- Sets GPIO4 as output, toggles in infinite loop (~100 kHz square wave)
- Runs AFTER the UART burst completes
- Purpose: continuous signal for probe/scope connectivity testing

Build: `build-gpio-scope/loader.img` (checksum `9019b7f0abde7263e6f3008affa55975`)

## Capture Results Summary

### Before CH1 fix (probe on Multi)

All captures identical:
- swing=3, i8=[-1, +2] or [-12, +2]
- period_num=0, frequency_hz=0
- Pure ADC noise, no signal

### After CH1 fix — GPIO4 square wave (pin 7)

```
attempt=3 swing=132 min_i8=-101 max_i8=31 period_num=236
```

Clear square wave detected. Confirmed scope + probe working.

### After CH1 fix — UART TX (pin 8)

Best single capture:
```
swing=97 min_i8=-78 max_i8=19 period_num=10
```

Histogram showed two clusters:
- HIGH: centered at i8=+5 (~2500 samples) — UART idle (3.3V)
- LOW:  centered at i8=-70 (~2500 samples) — UART active (0V)

Decoded: 1 frame, byte=0xFF, stop=ok. Only caught one UART transition.

### 10-second rapid-fire scan (510 captures)

Best capture:
```
swing=218 min_i8=-106 max_i8=112 period_num=10
```

But histogram showed uniform distribution across entire range — this looks
like the GPIO4 square wave bleeding into the capture, not clean UART data.
Decoded only 1 frame (0xFF).

## Current Status / Open Questions

1. **Scope and probe confirmed working** on CH1 with GPIO4 square wave.

2. **UART signal seen on pin 8** with correct two-cluster histogram in
   earlier captures (swing=97), but only 1 decoded frame.

3. **10-second scan best capture (swing=218)** shows uniform distribution
   that looks more like GPIO square wave than UART. Possible causes:
   - Probe physically closer to pin 7 (GPIO4) than pin 8 (GPIO14)
   - Crosstalk from GPIO4 square wave on adjacent pin
   - The UART burst may have already finished and scope captured GPIO instead

4. **Potential next steps:**
   - Remove GPIO4 infinite loop from image (only keep UART burst)
   - Or: separate test — UART-only image without GPIO square wave
   - Try longer UART burst or continuous UART output
   - Verify pin 8 contact with multimeter continuity check
   - Try probe on pin 8 with GPIO square wave disabled

## Build Commands

### owon-probe (scope driver)
```bash
cd /path/to/lionsos-cnc-control-v2/helpers/owon-native
cargo build -p owon-probe
OWON_CAPTURE_TRACE=1 cargo run -p owon-probe -- bringup
OWON_CAPTURE_TRACE=1 cargo run -p owon-probe -- uart-bench-capture <prefix> ch1
cargo run -p owon-probe -- decode-uart <meta.json> ch1 [baud] [--threshold N] [--invert]
```

### microkit-cnc-control (RPi image)
```bash
cd /home/dev/sel4/public-code/microkit-cnc-control
make BUILD_DIR=build-gpio-scope \
  MICROKIT_SDK=/home/dev/sel4/microkit/release/microkit-sdk-2.1.0-dev-rpi3b \
  MICROKIT_BOARD=rpi3b \
  MICROKIT_CONFIG=debug \
  TARGET_TRIPLE=aarch64-linux-gnu \
  CNC_UART_SCOPE_TEST=1
```

### Flash to SD
```bash
# SD mounts at /run/media/dev/RPICNC (or /mnt after manual mount)
cp build-gpio-scope/loader.img /run/media/dev/RPICNC/loader.img && sync
```
