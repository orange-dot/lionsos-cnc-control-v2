/* SPDX-License-Identifier: MIT */

#pragma once

#include <stdint.h>

/*
 * Stepper motion and safety primitives.
 * Bresenham 3-axis line, limit switch sensing, spindle PWM.
 * Adapted from lionsos-cnc-control v1.
 */

/* Enable/disable stepper drivers (active-low enable pin). */
void cnc_stepper_enable(void);
void cnc_stepper_disable(void);

/*
 * Bresenham line move from current position to (tx, ty, tz).
 * Updates *cx, *cy, *cz to reflect final position.
 * Returns 0 on success, 1 if aborted by limit switch.
 */
uint8_t cnc_line_to(int16_t *cx, int16_t *cy, int16_t *cz,
                    int16_t tx, int16_t ty, int16_t tz);

/* Set spindle PWM duty (0 = off, 1-255 = on). */
void cnc_spindle_set(uint8_t duty);

/* Stop all motion and disable spindle. */
void cnc_motion_stop(void);

/*
 * Read limit switch state.
 * Returns nonzero bitmask if any limit is active.
 * Calls cnc_motion_stop() on hit and latches the result.
 */
uint8_t cnc_limit_hit(void);

/* Return the latched limit mask from the last limit event. */
uint8_t cnc_limit_latched(void);

/* Clear the latched limit state (after homing or reset). */
void cnc_limit_clear(void);

/*
 * Home a single axis toward its limit switch, then back off.
 * axis: 0=X, 1=Y, 2=Z.
 * Returns 0 on success, 1 on unexpected limit (wrong axis).
 */
uint8_t cnc_home_axis(uint8_t axis);
