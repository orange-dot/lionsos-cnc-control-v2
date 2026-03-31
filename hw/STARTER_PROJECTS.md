# Starter Projects

These are the three best first projects if the goal is to start playing
quickly, not to prove out a final architecture.

## 1. Pi UI Toy

Host:

- `rpi3b`

Modules:

- rotary encoder
- RGB LED
- active buzzer

Why first:

- no analog dependency
- clear visible and audible feedback
- easy to turn into a tiny menu, mode selector, or alarm panel

What it does:

- encoder rotation changes mode or color
- press event confirms selection
- buzzer gives click or alert feedback

See:

- `PI_ONLY.md`

## 2. MCU Analog Toy

Host:

- `atmega328p`

Modules:

- joystick
- RGB LED

Why first:

- the MCU can read the joystick honestly as an analog input
- it gives immediate feedback with almost no protocol overhead

What it does:

- X and Y move color mix or brightness
- joystick switch toggles mode

See:

- `MCU_ONLY.md`

## 3. Split-host Demo

Hosts:

- joystick on MCU
- RGB LED and buzzer on Pi

Why first:

- clean separation of strengths
- MCU reads the analog control
- Pi owns higher-level behavior and presentation

What it does:

- MCU publishes joystick state
- Pi maps that into LED color, blink pattern, and tone events

See:

- `PI_MCU_COMBOS.md`
