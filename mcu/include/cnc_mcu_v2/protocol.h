/*
 * MCU-side protocol wrapper for CNC Control V2.
 *
 * SPDX-License-Identifier: MIT
 */

#pragma once

#include <stdint.h>

#include <cnc_v2/wire_protocol.h>

typedef cnc_frame_rx_fsm_t cnc_rx_fsm_t;

static inline void cnc_rx_init(cnc_rx_fsm_t *fsm)
{
    cnc_frame_rx_reset(fsm);
}

static inline uint8_t cnc_rx_feed(cnc_rx_fsm_t *fsm, uint8_t byte)
{
    return cnc_frame_rx_feed_cmd(fsm, byte);
}
