# SPDX-License-Identifier: MIT
#
# LionsOS CNC Control V2 bare-metal MCU executor scaffold.

DEVICE ?= atmega328p
CLOCK = 16000000UL
PROGRAMMER ?= -c arduino -P /dev/ttyUSB0 -b 115200
export CCACHE_DISABLE ?= 1

SRCDIR = src
BUILDDIR = build
INCDIR = include
PI_INCDIR = ../include

SOURCES = $(wildcard $(SRCDIR)/*.c)
OBJECTS = $(patsubst $(SRCDIR)/%.c,$(BUILDDIR)/%.o,$(SOURCES))

COMPILE = avr-gcc -Wall -Wextra -Os -DF_CPU=$(CLOCK) -mmcu=$(DEVICE) \
	-I$(INCDIR) -I$(PI_INCDIR) -ffunction-sections -fdata-sections -std=c11

AVRDUDE = avrdude $(PROGRAMMER) -p $(DEVICE)
TARGET = cnc-executor-v2

.PHONY: all flash clean size

all: $(TARGET).hex size

$(BUILDDIR):
	mkdir -p $(BUILDDIR)

$(BUILDDIR)/%.o: $(SRCDIR)/%.c | $(BUILDDIR)
	$(COMPILE) -MMD -MP -c $< -o $@

$(BUILDDIR)/$(TARGET).elf: $(OBJECTS)
	$(COMPILE) -o $@ $(OBJECTS) -Wl,--gc-sections

$(TARGET).hex: $(BUILDDIR)/$(TARGET).elf
	avr-objcopy -j .text -j .data -O ihex $< $@

flash: $(TARGET).hex
	$(AVRDUDE) -U flash:w:$(TARGET).hex:i

clean:
	rm -rf $(BUILDDIR) $(TARGET).hex

size: $(BUILDDIR)/$(TARGET).elf
	@echo "--- Firmware size ---"
	@avr-size --format=berkeley $<
	@echo ""

-include $(OBJECTS:.o=.d)
