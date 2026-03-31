# Pi-only Combinations

These combinations assume only `rpi3b` and modules that make sense without a
Pi-side ADC.

## Combo A: Encoder Desk Toy

Modules:

- rotary encoder
- RGB LED
- active buzzer

Minimal wiring:

- encoder signal pin to a Pi GPIO input
- encoder power to a Pi-safe supply
- RGB LED color pins to three Pi GPIO outputs
- buzzer signal pin to one Pi GPIO output
- common ground

Demo:

- rotate encoder to cycle color palette or mode index
- short beep on each detent or each accepted step
- press encoder switch to toggle blink mode

Why it is good:

- fully digital
- no fragile analog path
- immediate visible feedback

## Combo B: Magnetic Door Toy

Modules:

- reed switch
- two-color LED
- active buzzer

Minimal wiring:

- reed output to one Pi GPIO input
- two-color LED signal to one or two Pi GPIO outputs depending on module style
- buzzer signal to one Pi GPIO output
- common ground

Demo:

- magnet present means green and quiet
- magnet removed means red and chirp
- optional event counter in software

Why it is good:

- trivial logic
- great first interrupt or poll loop exercise

## Combo C: Beam / Object Trip

Modules:

- reflective infrared sensor
- slot photo interrupter or infrared sensor
- laser transmitter

Minimal wiring:

- each sensor output to one Pi GPIO input
- laser signal to one Pi GPIO output if gated, otherwise fixed power as needed
- common ground

Demo:

- count interruptions
- classify which sensor fired first
- blink software state on a terminal or optional LED output

Why it is good:

- feels more like a real machine edge detector
- still remains simple electrically

## Combo D: Tilt Lamp

Modules:

- glass tilt switch or ball tilt switch
- magic light cup

Minimal wiring:

- tilt switch output to one Pi GPIO input
- magic light cup signal to one Pi GPIO output
- common ground

Demo:

- one orientation steady
- other orientation blinking or breathing

Why it is good:

- tiny wiring footprint
- playful and physically obvious

## Pi-side Caution

Avoid treating these as analog projects on Pi without extra hardware:

- joystick axes
- photoresistor main value
- thermistor main value

Comparator boards can still be useful on Pi through `DO` or `D0`, but that is
threshold behavior, not full analog sensing.
