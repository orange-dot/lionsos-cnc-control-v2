/* SPDX-License-Identifier: MIT */

#pragma once

#ifndef F_CPU
#error "F_CPU must be defined"
#endif

#define CNC_MCU_V2_BAUD 115200UL
#define CNC_MCU_V2_CMD_BUF_SIZE 4u
#define CNC_MCU_V2_HEARTBEAT_PERIOD_MS 1000u
