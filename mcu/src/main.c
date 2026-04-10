/* SPDX-License-Identifier: MIT */

#include <avr/interrupt.h>
#include <util/delay.h>

#include <cnc_mcu_v2/config.h>
#include <cnc_mcu_v2/protocol.h>
#include <cnc_mcu_v2/uart.h>

extern uint8_t cnc_uart_rx_available(void);
extern uint8_t cnc_uart_rx_get(cnc_frame_t *out);

static uint8_t g_machine_state = CNC_STATE_IDLE;
static int16_t g_pos_x;
static int16_t g_pos_y;
static int16_t g_pos_z;
static uint8_t g_rsp_seq;
static uint32_t g_uptime_ms;

static void send_frame(cnc_frame_t *frame)
{
    cnc_uart_send((const uint8_t *)frame, CNC_FRAME_SIZE);
}

static void send_ack(uint8_t acked_seq)
{
    cnc_frame_t frame;

    cnc_pack_ack(&frame, g_rsp_seq++, acked_seq);
    send_frame(&frame);
}

static void send_nak(uint8_t naked_seq, uint8_t error)
{
    cnc_frame_t frame;

    cnc_pack_nak(&frame, g_rsp_seq++, naked_seq, error);
    send_frame(&frame);
}

static void send_position(void)
{
    cnc_frame_t frame;

    cnc_pack_position(&frame, g_rsp_seq++, g_pos_x, g_pos_y, g_pos_z, g_machine_state);
    send_frame(&frame);
}

static void send_heartbeat(void)
{
    cnc_frame_t frame;

    cnc_pack_heartbeat(&frame, g_rsp_seq++, g_uptime_ms / 1000u);
    send_frame(&frame);
}

static void handle_linear_move(const cnc_frame_t *cmd)
{
    cnc_pay_linear_move_t move;

    cnc_unpack_move(cmd, &move);
    if (g_machine_state == CNC_STATE_ESTOP) {
        send_nak(cmd->seq, CNC_ERR_BAD_STATE);
        return;
    }

    g_machine_state = CNC_STATE_MOVING;
    g_pos_x = move.x;
    g_pos_y = move.y;
    g_pos_z = move.z;
    g_machine_state = CNC_STATE_IDLE;
    send_ack(cmd->seq);
}

static void handle_spindle(const cnc_frame_t *cmd)
{
    if (g_machine_state == CNC_STATE_ESTOP) {
        send_nak(cmd->seq, CNC_ERR_BAD_STATE);
        return;
    }
    send_ack(cmd->seq);
}

static void handle_estop(const cnc_frame_t *cmd)
{
    g_machine_state = CNC_STATE_ESTOP;
    send_ack(cmd->seq);
}

static void handle_status_query(const cnc_frame_t *cmd)
{
    (void)cmd;
    send_position();
}

static void handle_home(const cnc_frame_t *cmd)
{
    if (g_machine_state == CNC_STATE_ESTOP) {
        send_nak(cmd->seq, CNC_ERR_BAD_STATE);
        return;
    }

    g_machine_state = CNC_STATE_HOMING;
    g_pos_x = 0;
    g_pos_y = 0;
    g_pos_z = 0;
    g_machine_state = CNC_STATE_IDLE;
    send_ack(cmd->seq);
}

static void handle_set_position(const cnc_frame_t *cmd)
{
    cnc_pay_set_position_t payload;

    cnc_memcpy(&payload, cmd->payload, sizeof(payload));
    g_pos_x = payload.x;
    g_pos_y = payload.y;
    g_pos_z = payload.z;
    if (g_machine_state != CNC_STATE_ESTOP) {
        g_machine_state = CNC_STATE_IDLE;
    }
    send_ack(cmd->seq);
}

static void dispatch(const cnc_frame_t *cmd)
{
    switch (cmd->type) {
    case CNC_CMD_LINEAR_MOVE:
        handle_linear_move(cmd);
        break;
    case CNC_CMD_SPINDLE:
        handle_spindle(cmd);
        break;
    case CNC_CMD_ESTOP:
        handle_estop(cmd);
        break;
    case CNC_CMD_STATUS_QUERY:
        handle_status_query(cmd);
        break;
    case CNC_CMD_HOME:
        handle_home(cmd);
        break;
    case CNC_CMD_SET_POSITION:
        handle_set_position(cmd);
        break;
    default:
        send_nak(cmd->seq, CNC_ERR_BAD_TYPE);
        break;
    }
}

int main(void)
{
    cnc_uart_init(CNC_MCU_V2_BAUD);
    sei();

    for (;;) {
        cnc_frame_t cmd;

        if (cnc_uart_rx_available() && cnc_uart_rx_get(&cmd)) {
            dispatch(&cmd);
        }

        _delay_ms(1);
        g_uptime_ms += 1u;
        if ((g_uptime_ms % CNC_MCU_V2_HEARTBEAT_PERIOD_MS) == 0u) {
            send_heartbeat();
        }
    }
}
