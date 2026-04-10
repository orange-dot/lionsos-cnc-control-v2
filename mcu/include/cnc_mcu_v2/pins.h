/* SPDX-License-Identifier: MIT */

#pragma once

/*
 * CNC pin profile for the ATmega328P on the VGJ-12 V2.5 board.
 * Matches the v1 mcu wiring in lionsos-cnc-control.
 */

#include <avr/io.h>

/* --- Step outputs (Port D) --- */
#define PIN_X_STEP_DDR      DDRD
#define PIN_X_STEP_PORT     PORTD
#define PIN_X_STEP_BIT      PD2     /* D2 */

#define PIN_Y_STEP_DDR      DDRD
#define PIN_Y_STEP_PORT     PORTD
#define PIN_Y_STEP_BIT      PD3     /* D3 */

#define PIN_Z_STEP_DDR      DDRD
#define PIN_Z_STEP_PORT     PORTD
#define PIN_Z_STEP_BIT      PD4     /* D4 */

/* --- Direction outputs (Port D) --- */
#define PIN_X_DIR_DDR       DDRD
#define PIN_X_DIR_PORT      PORTD
#define PIN_X_DIR_BIT       PD5     /* D5 */

#define PIN_Y_DIR_DDR       DDRD
#define PIN_Y_DIR_PORT      PORTD
#define PIN_Y_DIR_BIT       PD6     /* D6 */

#define PIN_Z_DIR_DDR       DDRD
#define PIN_Z_DIR_PORT      PORTD
#define PIN_Z_DIR_BIT       PD7     /* D7 */

/* --- Stepper enable (Port B, active low) --- */
#define PIN_ENABLE_DDR      DDRB
#define PIN_ENABLE_PORT     PORTB
#define PIN_ENABLE_BIT      PB0     /* D8 */

/* --- Limit switch inputs (Port B, active low with pull-ups) --- */
#define PIN_X_LIMIT_BIT     PB1     /* D9 */
#define PIN_Y_LIMIT_BIT     PB2     /* D10 */
#define PIN_Z_LIMIT_BIT     PB4     /* D12 */
#define PIN_LIMIT_MASK      ((1 << PIN_X_LIMIT_BIT) | \
                             (1 << PIN_Y_LIMIT_BIT) | \
                             (1 << PIN_Z_LIMIT_BIT))
#define PIN_LIMIT_PIN       PINB
#define PIN_LIMIT_DDR       DDRB
#define PIN_LIMIT_PORT      PORTB

/* --- Spindle PWM (Port B, Timer2 OC2A) --- */
#define PIN_SPINDLE_DDR     DDRB
#define PIN_SPINDLE_PORT    PORTB
#define PIN_SPINDLE_BIT     PB3     /* D11 / OC2A */
