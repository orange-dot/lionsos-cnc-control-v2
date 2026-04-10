/* SPDX-License-Identifier: MIT */

#pragma once

#include <stdint.h>

/*
 * HAL for bench sensor/actuator modules (KY-series breakouts).
 * Buzzer, RGB LED (soft PWM), rotary encoder (PCINT).
 */

/* --- Buzzer (digital output) --- */
void hal_buzzer_init(void);
void hal_buzzer_on(void);
void hal_buzzer_off(void);
void hal_buzzer_toggle(void);

/* --- RGB LED (soft PWM, 32 levels per channel) --- */
void hal_rgb_init(void);
void hal_rgb_set(uint8_t r, uint8_t g, uint8_t b);

/*
 * Must be called from the Timer0 1ms ISR to drive soft PWM.
 * Runs the 32-step PWM counter internally.
 */
void hal_rgb_tick(void);

/* --- Rotary encoder (click accumulator via PCINT) --- */
void hal_encoder_init(void);

/*
 * Returns accumulated clicks since last call and resets the counter.
 * Positive = clockwise, negative = counter-clockwise.
 */
int8_t hal_encoder_delta(void);
