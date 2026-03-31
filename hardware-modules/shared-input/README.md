# Shared-input Bucket

This bucket contains input modules that can plausibly be used from both
`rpi3b` and the current `atmega328p` MCU.

It includes two subtypes:

- direct digital/GPIO-style inputs
- comparator-style boards where the Pi can consume the digital threshold output
  and the MCU can additionally use analog inputs when useful

`sensor-05` and `sensor-09` remain here with caution because their electrical
shape looks simple enough, but their exact function still needs bench
verification.
