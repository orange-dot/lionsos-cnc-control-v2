# Shared-output Bucket

This bucket contains simple output modules that can plausibly be driven from
both `rpi3b` and the current `atmega328p` MCU.

The main rule is pragmatic:

- single-signal digital modules and LED-style modules belong here
- anything that would require a host-specific high-speed bus or unusual timing
  contract would not
