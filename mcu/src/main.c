/* SPDX-License-Identifier: MIT */

#include <avr/interrupt.h>
#include <avr/io.h>
#include <util/delay.h>

#include <cnc_mcu_v2/config.h>
#include <cnc_mcu_v2/hal_gpio.h>
#include <cnc_mcu_v2/pins.h>
#include <cnc_mcu_v2/protocol.h>
#include <cnc_mcu_v2/stepper.h>
#include <cnc_mcu_v2/tick.h>
#include <cnc_mcu_v2/uart.h>

extern uint8_t cnc_uart_rx_available(void);
extern uint8_t cnc_uart_rx_get(cnc_frame_t *out);

static uint8_t g_machine_state = CNC_STATE_IDLE;
static int16_t g_pos_x;
static int16_t g_pos_y;
static int16_t g_pos_z;
static uint8_t g_rsp_seq;
static uint32_t g_last_hb_ms;
static uint8_t g_encoder_pos;
static uint8_t g_buzzer_state;

/* ------------------------------------------------------------------ */
/* GPIO and timer initialisation                                       */
/* ------------------------------------------------------------------ */

static void gpio_init(void)
{
    /* Step + direction outputs */
    PIN_X_STEP_DDR |= (1 << PIN_X_STEP_BIT);
    PIN_Y_STEP_DDR |= (1 << PIN_Y_STEP_BIT);
    PIN_Z_STEP_DDR |= (1 << PIN_Z_STEP_BIT);
    PIN_X_DIR_DDR  |= (1 << PIN_X_DIR_BIT);
    PIN_Y_DIR_DDR  |= (1 << PIN_Y_DIR_BIT);
    PIN_Z_DIR_DDR  |= (1 << PIN_Z_DIR_BIT);

    /* Enable output + spindle output */
    PIN_ENABLE_DDR  |= (1 << PIN_ENABLE_BIT);
    PIN_SPINDLE_DDR |= (1 << PIN_SPINDLE_BIT);

    /* Limit inputs with pull-ups */
    PIN_LIMIT_DDR  &= ~PIN_LIMIT_MASK;
    PIN_LIMIT_PORT |= PIN_LIMIT_MASK;

    /* Timer2 fast PWM for spindle (OC2A), initially off */
    TCCR2A = (1 << WGM21) | (1 << WGM20);
    TCCR2B = (1 << CS22);
    OCR2A = 0;

    /* Start with steppers disabled */
    cnc_stepper_disable();
}

/* ------------------------------------------------------------------ */
/* Frame helpers                                                       */
/* ------------------------------------------------------------------ */

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

    /* Accumulate encoder clicks into wrapping position */
    g_encoder_pos = (uint8_t)(g_encoder_pos + (uint8_t)hal_encoder_delta());

    cnc_pack_heartbeat_ext(&frame, g_rsp_seq++, cnc_tick_ms() / 1000u,
                           g_encoder_pos, 0, 0, 0, g_buzzer_state);
    send_frame(&frame);
}

static void send_limit_event(uint8_t latched)
{
    cnc_frame_t f;
    uint8_t axis = 0;

    if (latched & (1 << PIN_X_LIMIT_BIT)) axis = 0;
    else if (latched & (1 << PIN_Y_LIMIT_BIT)) axis = 1;
    else if (latched & (1 << PIN_Z_LIMIT_BIT)) axis = 2;

    cnc_frame_init_rsp(&f, g_rsp_seq++, CNC_RSP_LIMIT_HIT);
    f.payload[0] = axis;
    f.payload[1] = 0;  /* direction unknown at this level */
    cnc_frame_seal(&f);
    send_frame(&f);
}

/* ------------------------------------------------------------------ */
/* Command handlers                                                    */
/* ------------------------------------------------------------------ */

static void handle_linear_move(const cnc_frame_t *cmd)
{
    cnc_pay_linear_move_t move;

    if (g_machine_state != CNC_STATE_IDLE) {
        send_nak(cmd->seq, CNC_ERR_BAD_STATE);
        return;
    }

    cnc_unpack_move(cmd, &move);
    g_machine_state = CNC_STATE_MOVING;
    cnc_stepper_enable();

    uint8_t hit = cnc_line_to(&g_pos_x, &g_pos_y, &g_pos_z,
                              move.x, move.y, move.z);

    if (hit) {
        g_machine_state = CNC_STATE_FAULT;
        send_limit_event(cnc_limit_latched());
        send_nak(cmd->seq, CNC_ERR_LIMIT);
    } else {
        g_machine_state = CNC_STATE_IDLE;
        send_ack(cmd->seq);
    }
}

static void handle_spindle(const cnc_frame_t *cmd)
{
    cnc_pay_spindle_t s;

    if (g_machine_state == CNC_STATE_ESTOP) {
        send_nak(cmd->seq, CNC_ERR_BAD_STATE);
        return;
    }

    cnc_memcpy(&s, cmd->payload, sizeof(s));
    if (s.on_off)
        cnc_spindle_set((uint8_t)(s.speed & 0xFFu));
    else
        cnc_spindle_set(0);

    send_ack(cmd->seq);
}

static void handle_estop(const cnc_frame_t *cmd)
{
    cnc_motion_stop();
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
    cnc_pay_home_t h;

    if (g_machine_state != CNC_STATE_IDLE && g_machine_state != CNC_STATE_FAULT) {
        send_nak(cmd->seq, CNC_ERR_BAD_STATE);
        return;
    }

    cnc_memcpy(&h, cmd->payload, sizeof(h));
    g_machine_state = CNC_STATE_HOMING;
    cnc_stepper_enable();

    uint8_t result = 0;
    if ((h.axis_mask & 0x01u) && !result) {
        result = cnc_home_axis(0);
        if (!result) g_pos_x = 0;
    }
    if ((h.axis_mask & 0x02u) && !result) {
        result = cnc_home_axis(1);
        if (!result) g_pos_y = 0;
    }
    if ((h.axis_mask & 0x04u) && !result) {
        result = cnc_home_axis(2);
        if (!result) g_pos_z = 0;
    }

    if (result) {
        g_machine_state = CNC_STATE_FAULT;
        send_nak(cmd->seq, CNC_ERR_LIMIT);
    } else {
        g_machine_state = CNC_STATE_IDLE;
        send_ack(cmd->seq);
    }
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

/* ------------------------------------------------------------------ */
/* Dispatcher                                                          */
/* ------------------------------------------------------------------ */

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

/* ------------------------------------------------------------------ */
/* Main                                                                */
/* ------------------------------------------------------------------ */

int main(void)
{
    gpio_init();
    hal_buzzer_init();
    hal_rgb_init();
    hal_encoder_init();
    cnc_tick_init();
    cnc_uart_init(CNC_MCU_V2_BAUD);
    sei();

    for (;;) {
        cnc_frame_t cmd;

        if (cnc_uart_rx_available() && cnc_uart_rx_get(&cmd)) {
            dispatch(&cmd);
        }

        uint32_t now = cnc_tick_ms();
        if ((now - g_last_hb_ms) >= CNC_MCU_V2_HEARTBEAT_PERIOD_MS) {
            g_last_hb_ms = now;
            send_heartbeat();
        }
    }
}
