# Pi + MCU Combinations

These combinations deliberately split work between the two hosts instead of
forcing one side to do everything.

The pattern is:

- MCU owns analog-heavy or timing-local sensing
- Pi owns richer policy, UI, logging, and mode selection

## Combo A: Joystick on MCU, Feedback on Pi

MCU modules:

- joystick

Pi modules:

- RGB LED
- active buzzer

Minimal split:

- MCU reads joystick axes and switch
- MCU sends normalized state over the existing Pi/MCU boundary
- Pi drives LED and buzzer behavior

Demo:

- joystick direction picks color family
- joystick magnitude picks blink speed or tone rate
- switch toggles scenes

Why it is good:

- very clear separation of responsibilities
- uses each host where it is strongest

## Combo B: Analog Environment on MCU, Scene Logic on Pi

MCU modules:

- photoresistor or thermistor

Pi modules:

- laser transmitter
- buzzer
- RGB LED or two-color LED

Minimal split:

- MCU measures analog value and sends bucketed or raw state
- Pi maps that into output scenes and alert logic

Demo:

- darkness enables laser and low glow mode
- brightness disables laser and uses a different LED scene
- temperature threshold adds buzzer warnings

Why it is good:

- naturally scales from threshold logic to richer policies

## Combo C: Event Fusion

MCU modules:

- joystick or thermistor

Pi modules:

- reed switch or tilt switch
- RGB LED or buzzer

Minimal split:

- MCU provides continuous or analog-heavy state
- Pi provides discrete event triggers
- Pi combines both into one mode machine

Demo:

- joystick selects profile
- reed or tilt event arms or disarms the active profile
- LED and buzzer reflect fused state

Why it is good:

- starts to look like a real control plane instead of a single sensor demo

## Split-host Rule Of Thumb

Use the MCU side for:

- joystick analog axes
- photoresistor as a true analog sensor
- thermistor as a true analog sensor

Use the Pi side for:

- LEDs
- buzzer patterns
- logging and mode handling
- digital-only sensors when you want fast iteration on application logic
