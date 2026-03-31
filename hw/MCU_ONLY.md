# MCU-only Combinations

These combinations assume the current bare-metal MCU in this repo:

- `atmega328p`

They favor modules where digital I/O is enough or where MCU analog input is the
natural fit.

## Combo A: Joystick Color Mixer

Modules:

- joystick
- RGB LED

Minimal wiring:

- joystick analog axes to MCU ADC pins
- joystick switch to one MCU digital input
- RGB LED channels to MCU outputs
- common ground

Demo:

- X and Y steer color balance or brightness
- switch toggles between color mode and blink mode

Why it is good:

- honest analog project
- tiny software surface
- immediate visual reward

## Combo B: Light-reactive Lamp

Modules:

- photoresistor light sensor
- RGB LED or two-color LED

Minimal wiring:

- photoresistor signal to MCU ADC input
- LED control pins to MCU outputs
- common ground

Demo:

- dark room raises brightness
- bright room lowers brightness
- optional hysteresis mode to avoid flicker

Why it is good:

- fits AVR ADC naturally
- turns into a useful threshold and smoothing exercise

## Combo C: Temperature Alarm

Modules:

- thermistor module
- active buzzer
- two-color LED

Minimal wiring:

- thermistor analog signal to MCU ADC input, or `DO` to digital if you only
  want threshold mode
- buzzer signal to one MCU output
- LED control to one or two MCU outputs
- common ground

Demo:

- normal temperature is green
- above threshold is red plus buzzer chirp
- switch between analog display mode and threshold-only mode in firmware

Why it is good:

- gives both analog and comparator-style options
- simple but not toy-like

## Combo D: Fire / Noise Alert

Modules:

- flame sensor or sound sensor
- buzzer
- LED

Minimal wiring:

- sensor `DO` to MCU digital input
- optional `A0` to MCU ADC if you want richer behavior
- buzzer and LED to outputs
- common ground

Demo:

- rising event latches alarm
- LED shows armed or triggered state
- buzzer pattern varies by source or intensity bucket

Why it is good:

- can stay digital and simple
- easy path toward richer analog processing later
