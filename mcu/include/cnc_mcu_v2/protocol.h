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

/*
 * Extended heartbeat that piggybacks sensor telemetry into the 4
 * pad bytes of cnc_pay_heartbeat_t.  Wire-compatible — the frame
 * size and CRC coverage are unchanged, and old Pi firmware simply
 * ignores payload[4..7].
 *
 * Layout of payload[0..7]:
 *   [0..3] uptime_s  (uint32_t, little-endian)
 *   [4]    encoder_pos   — rotary encoder absolute position (wrapping)
 *   [5]    rgb_packed    — R[7:5] G[4:2] B[1:0] (3-3-2 bit)
 *   [6]    buzzer_on     — 0 or 1
 *   [7]    module_flags  — reserved for future expansion
 */
static inline void cnc_pack_heartbeat_ext(cnc_frame_t *frame, uint8_t seq,
                                          uint32_t uptime_s,
                                          uint8_t encoder_pos,
                                          uint8_t rgb_r, uint8_t rgb_g,
                                          uint8_t rgb_b, uint8_t buzzer_on)
{
    cnc_frame_init_rsp(frame, seq, CNC_RSP_HEARTBEAT);
    cnc_memcpy(frame->payload, &uptime_s, sizeof(uptime_s));
    frame->payload[4] = encoder_pos;
    frame->payload[5] = (uint8_t)((rgb_r & 0xE0u) | ((rgb_g >> 3) & 0x1Cu) | ((rgb_b >> 6) & 0x03u));
    frame->payload[6] = buzzer_on ? 1u : 0u;
    frame->payload[7] = 0u;
    cnc_frame_seal(frame);
}
