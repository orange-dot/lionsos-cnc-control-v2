# Hardware Modules Workspace

This folder curates the small module inventory copied from
`/home/dev/sel4/lionsos-cnc-control/sensors` into buckets that matter for the
`lionsos-cnc-control-v2` line.

The target hosts are:

- Raspberry Pi 3B Pi-side experiments
- the current MCU in this repo: `atmega328p`

## Buckets

- `shared-input/`
  - input-side modules that can reasonably be used from both `rpi3b` and the
    current MCU
  - this includes direct GPIO-style modules and comparator boards where the Pi
    can use a digital threshold output and the MCU can use digital or analog
    paths as needed
- `shared-output/`
  - output-side modules that are simple enough to drive from both `rpi3b` and
    the current MCU
- `mcu-only/`
  - modules whose main value is analog-first and therefore a better fit for the
    `atmega328p` ADC than for a stock `rpi3b`
- `pi-only/`
  - currently empty
  - nothing in the source inventory looked honestly `rpi3b`-only relative to
    the current MCU

## Included In `shared-input/`

- joystick module: `sensor-01`
- sound sensor modules: `sensor-02`, `sensor-10`
- rotary encoder: `sensor-04`
- optical emitter/receiver: `sensor-05`
- infrared sensor: `sensor-06`
- flame sensor: `sensor-08`
- clear optical module: `sensor-09`
- thermistor module: `sensor-11`
- cylindrical tilt switch: `sensor-12`
- reflective infrared sensor: `sensor-13`
- slot photo interrupter: `sensor-15`
- glass tilt switch: `sensor-16`
- reed switch: `sensor-17`
- ball tilt switch: `sensor-20`

## Included In `shared-output/`

- laser transmitter: `sensor-03`
- four-pin LED: `sensor-07`
- active buzzer: `sensor-14`
- RGB LED: `sensor-18`
- two-color LED: `sensor-21`
- magic light cup: `sensor-22`
- SMD RGB LED: `sensor-23`

## Included In `mcu-only/`

- photoresistor light sensor: `sensor-19`

## Why `mcu-only/`

This module looks most useful through the MCU's ADC path:

- the photoresistor module appears to be an analog-first light sensor breakout

The Pi can still use external conditioning around it, but that would not be the
honest primary classification here.

## Why `sensor-01` Moved To `shared-input/`

The joystick is still analog-first, but unlike the photoresistor it also has a
clear digital push switch and an obvious operator-input role for both sides.

That makes it reasonable to keep in `shared-input/` with one caveat:

- the MCU is the better host if you want the full analog axis behavior
- the Pi classification is more limited unless an external ADC or thresholding
  path is added

## Confidence Notes

Two copied modules still need bench verification before we build around them:

- `sensor-05` optical emitter/receiver
  - broad label, medium confidence
- `sensor-09` clear optical module
  - exact function is uncertain, low confidence

They are still placed in `shared-input/` because their visible pinout and
breakout style suggest a simple GPIO-style electrical interface rather than a
platform-specific bus dependency.

## Comparator-board Caveat

Some `shared-input/` modules are not equal across the two hosts.

Examples:

- sound sensor modules: `sensor-02`, `sensor-10`
- flame sensor: `sensor-08`
- thermistor module: `sensor-11`

On the Pi side, these are best treated as digital threshold inputs when the
board exposes `DO` or `D0`.

On the MCU side, they remain more flexible because the AVR can also use analog
inputs where available.

## Electrical Note

For `rpi3b`, treat every module as `3.3V`-sensitive until proven otherwise.
If a module is powered from `5V` and can drive its signal line to `5V`, do not
connect it directly to a Pi GPIO without level shifting or a known-safe
`3.3V` operating mode.
