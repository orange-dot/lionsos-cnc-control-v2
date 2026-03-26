/*
 * CNC V2 wire protocol shared between the Pi-side transport PD and the MCU.
 *
 * Fixed-width and portable C11 so the same header can be used by AArch64
 * LionsOS/Microkit components and the AVR bare-metal executor.
 *
 * SPDX-License-Identifier: MIT
 */

#pragma once

#include <stdbool.h>
#include <stdint.h>

#define CNC_FRAME_SIZE 12u
#define CNC_PAYLOAD_SIZE 8u
#define CNC_SENTINEL_CMD 0x55u
#define CNC_SENTINEL_RSP 0xAAu
#define CNC_WINDOW_SIZE 3u
#define CNC_CRC8_POLY 0x07u

enum cnc_cmd_type {
    CNC_CMD_LINEAR_MOVE = 0x01u,
    CNC_CMD_SPINDLE = 0x02u,
    CNC_CMD_ESTOP = 0x03u,
    CNC_CMD_STATUS_QUERY = 0x04u,
    CNC_CMD_HOME = 0x05u,
    CNC_CMD_SET_POSITION = 0x06u,
};

enum cnc_rsp_type {
    CNC_RSP_POSITION = 0x81u,
    CNC_RSP_LIMIT_HIT = 0x82u,
    CNC_RSP_CMD_ACK = 0x83u,
    CNC_RSP_CMD_NAK = 0x84u,
    CNC_RSP_HEARTBEAT = 0x85u,
};

enum cnc_machine_state {
    CNC_STATE_IDLE = 0x00u,
    CNC_STATE_MOVING = 0x01u,
    CNC_STATE_HOMING = 0x02u,
    CNC_STATE_FAULT = 0x03u,
    CNC_STATE_ESTOP = 0x04u,
};

enum cnc_error {
    CNC_ERR_BAD_CRC = 0x01u,
    CNC_ERR_BAD_TYPE = 0x02u,
    CNC_ERR_BAD_STATE = 0x03u,
    CNC_ERR_LIMIT = 0x04u,
    CNC_ERR_QUEUE_FULL = 0x05u,
};

typedef struct {
    uint8_t sentinel;
    uint8_t seq;
    uint8_t type;
    uint8_t payload[CNC_PAYLOAD_SIZE];
    uint8_t crc;
} cnc_frame_t;

_Static_assert(sizeof(cnc_frame_t) == CNC_FRAME_SIZE, "frame size mismatch");

typedef struct {
    int16_t x;
    int16_t y;
    int16_t z;
    uint16_t feedrate;
} cnc_pay_linear_move_t;

typedef struct {
    uint16_t speed;
    uint8_t on_off;
    uint8_t _pad[5];
} cnc_pay_spindle_t;

typedef struct {
    uint8_t axis_mask;
    uint8_t _pad[7];
} cnc_pay_home_t;

typedef struct {
    int16_t x;
    int16_t y;
    int16_t z;
    uint8_t _pad[2];
} cnc_pay_set_position_t;

typedef struct {
    int16_t x;
    int16_t y;
    int16_t z;
    uint8_t state;
    uint8_t _pad;
} cnc_pay_position_t;

typedef struct {
    uint8_t axis;
    uint8_t direction;
    uint8_t _pad[6];
} cnc_pay_limit_hit_t;

typedef struct {
    uint8_t acked_seq;
    uint8_t _pad[7];
} cnc_pay_cmd_ack_t;

typedef struct {
    uint8_t naked_seq;
    uint8_t error;
    uint8_t _pad[6];
} cnc_pay_cmd_nak_t;

typedef struct {
    uint32_t uptime_s;
    uint8_t _pad[4];
} cnc_pay_heartbeat_t;

_Static_assert(sizeof(cnc_pay_linear_move_t) == CNC_PAYLOAD_SIZE, "move payload size");
_Static_assert(sizeof(cnc_pay_spindle_t) == CNC_PAYLOAD_SIZE, "spindle payload size");
_Static_assert(sizeof(cnc_pay_home_t) == CNC_PAYLOAD_SIZE, "home payload size");
_Static_assert(sizeof(cnc_pay_set_position_t) == CNC_PAYLOAD_SIZE, "set position payload size");
_Static_assert(sizeof(cnc_pay_position_t) == CNC_PAYLOAD_SIZE, "position payload size");
_Static_assert(sizeof(cnc_pay_limit_hit_t) == CNC_PAYLOAD_SIZE, "limit payload size");
_Static_assert(sizeof(cnc_pay_cmd_ack_t) == CNC_PAYLOAD_SIZE, "ack payload size");
_Static_assert(sizeof(cnc_pay_cmd_nak_t) == CNC_PAYLOAD_SIZE, "nak payload size");
_Static_assert(sizeof(cnc_pay_heartbeat_t) == CNC_PAYLOAD_SIZE, "heartbeat payload size");

#define cnc_memcpy __builtin_memcpy

static inline uint8_t cnc_crc8(const uint8_t *data, uint8_t len)
{
    uint8_t crc = 0x00u;

    for (uint8_t i = 0; i < len; i++) {
        crc ^= data[i];
        for (uint8_t bit = 0; bit < 8u; bit++) {
            crc = (crc & 0x80u) ? (uint8_t)((crc << 1) ^ CNC_CRC8_POLY)
                                : (uint8_t)(crc << 1);
        }
    }

    return crc;
}

static inline void cnc_frame_seal(cnc_frame_t *frame)
{
    frame->crc = cnc_crc8((const uint8_t *)frame, CNC_FRAME_SIZE - 1u);
}

static inline bool cnc_frame_valid(const cnc_frame_t *frame)
{
    return frame->crc == cnc_crc8((const uint8_t *)frame, CNC_FRAME_SIZE - 1u);
}

static inline void cnc_frame_init_cmd(cnc_frame_t *frame, uint8_t seq, uint8_t type)
{
    frame->sentinel = CNC_SENTINEL_CMD;
    frame->seq = seq;
    frame->type = type;
    for (uint8_t i = 0; i < CNC_PAYLOAD_SIZE; i++) {
        frame->payload[i] = 0u;
    }
}

static inline void cnc_frame_init_rsp(cnc_frame_t *frame, uint8_t seq, uint8_t type)
{
    frame->sentinel = CNC_SENTINEL_RSP;
    frame->seq = seq;
    frame->type = type;
    for (uint8_t i = 0; i < CNC_PAYLOAD_SIZE; i++) {
        frame->payload[i] = 0u;
    }
}

static inline void cnc_pack_move(cnc_frame_t *frame, uint8_t seq,
                                 int16_t x, int16_t y, int16_t z,
                                 uint16_t feedrate)
{
    cnc_pay_linear_move_t payload = { x, y, z, feedrate };

    cnc_frame_init_cmd(frame, seq, CNC_CMD_LINEAR_MOVE);
    cnc_memcpy(frame->payload, &payload, CNC_PAYLOAD_SIZE);
    cnc_frame_seal(frame);
}

static inline void cnc_pack_spindle(cnc_frame_t *frame, uint8_t seq,
                                    uint16_t speed, uint8_t on_off)
{
    cnc_pay_spindle_t payload = { speed, on_off, {0} };

    cnc_frame_init_cmd(frame, seq, CNC_CMD_SPINDLE);
    cnc_memcpy(frame->payload, &payload, CNC_PAYLOAD_SIZE);
    cnc_frame_seal(frame);
}

static inline void cnc_unpack_move(const cnc_frame_t *frame, cnc_pay_linear_move_t *payload)
{
    cnc_memcpy(payload, frame->payload, sizeof(*payload));
}

static inline void cnc_pack_position(cnc_frame_t *frame, uint8_t seq,
                                     int16_t x, int16_t y, int16_t z,
                                     uint8_t state)
{
    cnc_pay_position_t payload = { x, y, z, state, 0u };

    cnc_frame_init_rsp(frame, seq, CNC_RSP_POSITION);
    cnc_memcpy(frame->payload, &payload, CNC_PAYLOAD_SIZE);
    cnc_frame_seal(frame);
}

static inline void cnc_unpack_position(const cnc_frame_t *frame, cnc_pay_position_t *payload)
{
    cnc_memcpy(payload, frame->payload, sizeof(*payload));
}

static inline void cnc_pack_ack(cnc_frame_t *frame, uint8_t seq, uint8_t acked_seq)
{
    cnc_frame_init_rsp(frame, seq, CNC_RSP_CMD_ACK);
    frame->payload[0] = acked_seq;
    cnc_frame_seal(frame);
}

static inline void cnc_pack_nak(cnc_frame_t *frame, uint8_t seq,
                                uint8_t naked_seq, uint8_t error)
{
    cnc_frame_init_rsp(frame, seq, CNC_RSP_CMD_NAK);
    frame->payload[0] = naked_seq;
    frame->payload[1] = error;
    cnc_frame_seal(frame);
}

static inline void cnc_pack_heartbeat(cnc_frame_t *frame, uint8_t seq, uint32_t uptime_s)
{
    cnc_frame_init_rsp(frame, seq, CNC_RSP_HEARTBEAT);
    cnc_memcpy(frame->payload, &uptime_s, sizeof(uptime_s));
    cnc_frame_seal(frame);
}

typedef struct {
    uint8_t state;
    uint8_t pos;
    cnc_frame_t frame;
} cnc_frame_rx_fsm_t;

enum cnc_frame_rx_state {
    CNC_FRAME_RX_WAIT_SENTINEL = 0u,
    CNC_FRAME_RX_ACCUMULATE = 1u,
};

static inline void cnc_frame_rx_reset(cnc_frame_rx_fsm_t *fsm)
{
    fsm->state = CNC_FRAME_RX_WAIT_SENTINEL;
    fsm->pos = 0u;
}

static inline uint8_t cnc_frame_rx_feed(cnc_frame_rx_fsm_t *fsm,
                                        uint8_t expected_sentinel,
                                        uint8_t byte)
{
    uint8_t *raw = (uint8_t *)&fsm->frame;

    switch (fsm->state) {
    case CNC_FRAME_RX_WAIT_SENTINEL:
        if (byte == expected_sentinel) {
            raw[0] = byte;
            fsm->pos = 1u;
            fsm->state = CNC_FRAME_RX_ACCUMULATE;
        }
        return 0u;

    case CNC_FRAME_RX_ACCUMULATE:
        raw[fsm->pos++] = byte;
        if (fsm->pos >= CNC_FRAME_SIZE) {
            cnc_frame_rx_reset(fsm);
            return 1u;
        }
        return 0u;

    default:
        cnc_frame_rx_reset(fsm);
        return 0u;
    }
}

static inline uint8_t cnc_frame_rx_feed_cmd(cnc_frame_rx_fsm_t *fsm, uint8_t byte)
{
    return cnc_frame_rx_feed(fsm, CNC_SENTINEL_CMD, byte);
}

static inline uint8_t cnc_frame_rx_feed_rsp(cnc_frame_rx_fsm_t *fsm, uint8_t byte)
{
    return cnc_frame_rx_feed(fsm, CNC_SENTINEL_RSP, byte);
}
