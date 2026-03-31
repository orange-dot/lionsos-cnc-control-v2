# MCU-only Bucket

This bucket is reserved for modules whose honest primary value depends on the
current MCU more than on `rpi3b`.

Right now that means analog-first modules where the `atmega328p` ADC is the
obvious fit and the Pi would need extra external help.
