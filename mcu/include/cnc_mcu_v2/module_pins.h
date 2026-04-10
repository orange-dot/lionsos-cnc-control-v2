/* SPDX-License-Identifier: MIT */

#pragma once

/*
 * Pin assignments for bench sensor/actuator modules (KY-series breakouts).
 * These use the free PORTC pins not occupied by the CNC shield.
 *
 * See pins.h for the CNC shield pin allocation (PORTD + PORTB).
 */

#include <avr/io.h>

/* --- Buzzer (sensor-14): digital output on A0 --- */
#define PIN_BUZZER_DDR      DDRC
#define PIN_BUZZER_PORT     PORTC
#define PIN_BUZZER_BIT      PC0     /* A0 */

/* --- RGB LED (sensor-18): soft PWM outputs on A1-A3 --- */
#define PIN_RGB_R_DDR       DDRC
#define PIN_RGB_R_PORT      PORTC
#define PIN_RGB_R_BIT       PC1     /* A1 */

#define PIN_RGB_G_DDR       DDRC
#define PIN_RGB_G_PORT      PORTC
#define PIN_RGB_G_BIT       PC2     /* A2 */

#define PIN_RGB_B_DDR       DDRC
#define PIN_RGB_B_PORT      PORTC
#define PIN_RGB_B_BIT       PC3     /* A3 */

#define PIN_RGB_MASK        ((1 << PIN_RGB_R_BIT) | \
                             (1 << PIN_RGB_G_BIT) | \
                             (1 << PIN_RGB_B_BIT))

/* --- Rotary encoder (sensor-04): PCINT input on A4 --- */
#define PIN_ENC_CLK_DDR     DDRC
#define PIN_ENC_CLK_PORT    PORTC
#define PIN_ENC_CLK_PIN     PINC
#define PIN_ENC_CLK_BIT     PC4     /* A4 / PCINT12 */
#define PIN_ENC_CLK_PCINT   PCINT12
#define PIN_ENC_CLK_PCMSK   PCMSK1
#define PIN_ENC_CLK_PCIE    PCIE1

/* --- Reserved: A5 (PC5) for encoder DT or spare --- */

/* --- Debug LED on D13 (PB5) --- */
#define PIN_DBG_LED_DDR     DDRB
#define PIN_DBG_LED_PORT    PORTB
#define PIN_DBG_LED_BIT     PB5     /* D13 */
