# Hardware Play Ideas

This folder turns the curated inventory in
`/home/dev/sel4/lionsos-cnc-control-v2/hardware-modules` into small practical
play ideas for:

- `rpi3b` only
- `atmega328p` MCU only
- mixed `rpi3b + atmega328p`

The goal is not a full hardware spec. The goal is to give a few concrete
module combinations that are easy to wire and fun to explore.

## Files

- `STARTER_PROJECTS.md`
  - three recommended first projects
- `PI_ONLY.md`
  - combinations that make sense with Raspberry Pi 3B by itself
- `MCU_ONLY.md`
  - combinations that make sense with the current AVR MCU by itself
- `PI_MCU_COMBOS.md`
  - mixed split-host combinations

## Constraints

- `rpi3b` GPIO is `3.3V` sensitive
- the current MCU is `atmega328p`, so analog-first modules fit naturally there
- comparator modules with `DO` or `D0` are good shared inputs
- truly analog-first modules should stay on the MCU unless extra Pi-side ADC or
  conditioning is introduced

## Inventory Source

See:

- `/home/dev/sel4/lionsos-cnc-control-v2/hardware-modules/shared-input`
- `/home/dev/sel4/lionsos-cnc-control-v2/hardware-modules/shared-output`
- `/home/dev/sel4/lionsos-cnc-control-v2/hardware-modules/mcu-only`
