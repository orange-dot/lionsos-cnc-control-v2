/* SPDX-License-Identifier: MIT */

#pragma once

#ifndef F_CPU
#error "F_CPU must be defined"
#endif

#define CNC_MCU_V2_BAUD 115200UL
#define CNC_MCU_V2_CMD_BUF_SIZE 4u
#define CNC_MCU_V2_HEARTBEAT_PERIOD_MS 1000u

/* Step pulse width (microseconds) */
#define CNC_MCU_V2_STEP_PULSE_US  10

/* Inter-step delay (microseconds) — sets max step rate */
#define CNC_MCU_V2_STEP_DELAY_US  500

/* Homing back-off distance (steps) after limit switch contact */
#define CNC_MCU_V2_HOME_BACKOFF   200
