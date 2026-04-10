/* SPDX-License-Identifier: MIT */

#pragma once

#include <stdint.h>

/*
 * Timer0-based 1ms system tick.
 * Provides accurate monotonic millisecond counter independent of
 * the main loop cadence (which blocks during stepper motion).
 */

void cnc_tick_init(void);
uint32_t cnc_tick_ms(void);
