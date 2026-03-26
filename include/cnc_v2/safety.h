/*
 * Safety policy helpers for the V2 control line.
 *
 * SPDX-License-Identifier: MIT
 */

#pragma once

#include <stdbool.h>
#include <stdint.h>

#include <cnc_v2/wire_protocol.h>

typedef struct {
    int16_t x_min;
    int16_t x_max;
    int16_t y_min;
    int16_t y_max;
    int16_t z_min;
    int16_t z_max;
    uint16_t max_feedrate;
    uint16_t max_spindle;
} cnc_soft_limits_t;

#define CNC_DEFAULT_LIMITS ((cnc_soft_limits_t){ \
    .x_min = -16000, .x_max = 16000, \
    .y_min = -16000, .y_max = 16000, \
    .z_min = -16000, .z_max = 10000, \
    .max_feedrate = 2000, \
    .max_spindle = 255, \
})

static inline bool cnc_safety_check(const cnc_frame_t *frame,
                                    const cnc_soft_limits_t *limits,
                                    uint8_t *out_error)
{
    *out_error = 0u;

    switch (frame->type) {
    case CNC_CMD_LINEAR_MOVE: {
        cnc_pay_linear_move_t move;

        cnc_unpack_move(frame, &move);
        if (move.x < limits->x_min || move.x > limits->x_max ||
            move.y < limits->y_min || move.y > limits->y_max ||
            move.z < limits->z_min || move.z > limits->z_max) {
            *out_error = CNC_ERR_BAD_STATE;
            return false;
        }
        if (move.feedrate > limits->max_feedrate) {
            *out_error = CNC_ERR_BAD_STATE;
            return false;
        }
        return true;
    }

    case CNC_CMD_SPINDLE: {
        cnc_pay_spindle_t spindle;

        cnc_memcpy(&spindle, frame->payload, sizeof(spindle));
        if (spindle.speed > limits->max_spindle) {
            *out_error = CNC_ERR_BAD_STATE;
            return false;
        }
        return true;
    }

    case CNC_CMD_ESTOP:
    case CNC_CMD_STATUS_QUERY:
    case CNC_CMD_HOME:
    case CNC_CMD_SET_POSITION:
        return true;

    default:
        *out_error = CNC_ERR_BAD_TYPE;
        return false;
    }
}
