/*
 * sDDF serial client adapter for the CNC MCU wire boundary.
 *
 * SPDX-License-Identifier: MIT
 */

#include <stdbool.h>
#include <stdint.h>

#include <microkit.h>
#include <sddf/serial/config.h>
#include <sddf/serial/queue.h>
#include <sddf/timer/client.h>
#include <sddf/timer/config.h>

#include <cnc_v2/config.h>
#include <cnc_v2/control_ipc.h>

#ifndef CNC_V2_UART_SCOPE_TEST
#define CNC_V2_UART_SCOPE_TEST 0
#endif

#if CNC_V2_UART_SCOPE_TEST
#define CNC_V2_UART_SCOPE_PATTERN "LIONSOS-V2-UART-SCOPE 0123456789 ABCDEF\r\n"
#define CNC_V2_UART_SCOPE_KICK_NS 1000000ull
#define CNC_V2_UART_SCOPE_MAX_SEGMENTS 8u
#endif

__attribute__((__section__(".serial_client_config"))) serial_client_config_t serial_config;
__attribute__((__section__(".timer_client_config"))) timer_client_config_t timer_config;
__attribute__((__section__(".mcu_transport_config"))) cnc_v2_mcu_transport_config_t config;

serial_queue_handle_t serial_tx_queue_handle;
serial_queue_handle_t serial_rx_queue_handle;

static cnc_v2_transport_status_t g_status;
static cnc_frame_rx_fsm_t g_rsp_fsm;
static uint32_t g_last_decision_generation;
static bool g_serial_rx_enabled;
static bool g_waiting_for_response;

#if CNC_V2_UART_SCOPE_TEST
static uint32_t g_scope_pattern_offset;
#endif

static volatile cnc_v2_safety_decision_t *safety_decision_page(void)
{
    return (volatile cnc_v2_safety_decision_t *)(uintptr_t)config.safety_decision_vaddr;
}

static volatile cnc_v2_transport_status_t *transport_status_page(void)
{
    return (volatile cnc_v2_transport_status_t *)(uintptr_t)config.transport_status_vaddr;
}

static void publish_status(void)
{
    cnc_v2_publish_transport_status(transport_status_page(), &g_status);
    microkit_notify(config.state_core_ch);
    microkit_notify(config.observability_ch);
}

static bool send_frame_bytes(const cnc_frame_t *frame)
{
    uint32_t sent = serial_enqueue_batch(&serial_tx_queue_handle,
                                         CNC_FRAME_SIZE,
                                         (const char *)frame);
    if (sent != CNC_FRAME_SIZE) {
        microkit_notify(serial_config.tx.id);
        return false;
    }

    microkit_notify(serial_config.tx.id);
    return true;
}

#if CNC_V2_UART_SCOPE_TEST
static uint32_t send_scope_segment(void)
{
    static const char pattern[] = CNC_V2_UART_SCOPE_PATTERN;
    const uint32_t pattern_len = (uint32_t)(sizeof(pattern) - 1u);
    uint32_t remaining = pattern_len - g_scope_pattern_offset;
    uint32_t sent;

    sent = serial_enqueue_batch(&serial_tx_queue_handle,
                                remaining,
                                &pattern[g_scope_pattern_offset]);
    g_scope_pattern_offset += sent;
    if (g_scope_pattern_offset == pattern_len) {
        g_scope_pattern_offset = 0u;
    }

    return sent;
}

static void pump_scope_output(void)
{
    bool sent_any = false;

    for (uint32_t segment = 0u; segment < CNC_V2_UART_SCOPE_MAX_SEGMENTS; segment++) {
        uint32_t sent = send_scope_segment();

        if (sent == 0u) {
            break;
        }
        sent_any = true;
    }

    if (sent_any) {
        microkit_notify(serial_config.tx.id);
    }
}
#endif

static void record_response(const cnc_frame_t *frame)
{
    g_waiting_for_response = false;
    g_status.phase = CNC_V2_TRANSPORT_RESPONSE_OK;
    g_status.health = CNC_V2_TRANSPORT_HEALTH_NOMINAL;
    g_status.last_error = CNC_V2_TRANSPORT_ERR_NONE;
    g_status.last_rsp_seq = frame->seq;
    g_status.last_rx_frame = *frame;
    g_status.rx_count += 1u;

    if (frame->type == CNC_RSP_POSITION) {
        cnc_pay_position_t position;

        cnc_unpack_position(frame, &position);
        g_status.machine_state = position.state;
    } else if (frame->type == CNC_RSP_CMD_NAK) {
        g_status.last_error = frame->payload[1];
    }

    publish_status();
}

static void drain_rx_queue(void)
{
    char byte;

    if (!g_serial_rx_enabled) {
        return;
    }

    while (serial_dequeue(&serial_rx_queue_handle, &byte) == 0) {
        if (!cnc_frame_rx_feed_rsp(&g_rsp_fsm, (uint8_t)byte)) {
            continue;
        }

        if (!cnc_frame_valid(&g_rsp_fsm.frame)) {
            g_status.health = CNC_V2_TRANSPORT_HEALTH_DEGRADED;
            g_status.last_error = CNC_V2_TRANSPORT_ERR_BAD_CRC;
            publish_status();
            continue;
        }

        record_response(&g_rsp_fsm.frame);
    }
}

void init(void)
{
    if (!cnc_v2_config_magic_valid(config.magic) ||
        !serial_config_check_magic(&serial_config) ||
        !timer_config_check_magic(&timer_config)) {
        microkit_dbg_puts("mcu_transport: bad config magic\n");
        return;
    }

    g_serial_rx_enabled = (serial_config.rx.queue.vaddr != 0);
    serial_queue_init(&serial_tx_queue_handle,
                      serial_config.tx.queue.vaddr,
                      serial_config.tx.data.size,
                      serial_config.tx.data.vaddr);
    if (g_serial_rx_enabled) {
        serial_queue_init(&serial_rx_queue_handle,
                          serial_config.rx.queue.vaddr,
                          serial_config.rx.data.size,
                          serial_config.rx.data.vaddr);
    }

    cnc_frame_rx_reset(&g_rsp_fsm);
    g_status.generation = CNC_V2_GENERATION_INIT;
    g_status.phase = CNC_V2_TRANSPORT_IDLE;
    g_status.health = CNC_V2_TRANSPORT_HEALTH_UNKNOWN;
    g_status.machine_state = CNC_STATE_IDLE;
    publish_status();

#if CNC_V2_UART_SCOPE_TEST
    g_status.phase = CNC_V2_TRANSPORT_TX_SUBMITTED;
    g_status.health = CNC_V2_TRANSPORT_HEALTH_NOMINAL;
    publish_status();
    pump_scope_output();
    sddf_timer_set_timeout(timer_config.driver_id, CNC_V2_UART_SCOPE_KICK_NS);
#endif
}

void notified(microkit_channel ch)
{
    cnc_v2_safety_decision_t decision;

    if (ch == serial_config.rx.id) {
        drain_rx_queue();
        return;
    }

    if (ch == serial_config.tx.id) {
#if CNC_V2_UART_SCOPE_TEST
        pump_scope_output();
#endif
        return;
    }

    if (ch == timer_config.driver_id) {
#if CNC_V2_UART_SCOPE_TEST
        pump_scope_output();
        sddf_timer_set_timeout(timer_config.driver_id, CNC_V2_UART_SCOPE_KICK_NS);
        return;
#endif
        if (g_waiting_for_response) {
            g_waiting_for_response = false;
            g_status.timeout_count += 1u;
            g_status.phase = CNC_V2_TRANSPORT_TIMEOUT;
            g_status.health = CNC_V2_TRANSPORT_HEALTH_DEGRADED;
            g_status.last_error = CNC_V2_TRANSPORT_ERR_TIMEOUT;
            publish_status();
        }
        return;
    }

    if (ch != CNC_V2_CH_SAFETY_TO_XPORT) {
        return;
    }
#if CNC_V2_UART_SCOPE_TEST
    return;
#endif
    if (!cnc_v2_read_safety_decision(safety_decision_page(), &decision)) {
        return;
    }
    if (decision.generation == 0u || decision.generation == g_last_decision_generation) {
        return;
    }

    g_last_decision_generation = decision.generation;
    g_status.last_cmd_seq = decision.last_cmd_seq;

    if (decision.result != CNC_V2_SAFETY_APPROVED) {
        g_status.phase = CNC_V2_TRANSPORT_BLOCKED;
        g_status.last_error = decision.reason;
        publish_status();
        return;
    }

    g_status.last_tx_frame = decision.frame;
    g_status.tx_count += 1u;
    g_status.phase = CNC_V2_TRANSPORT_TX_SUBMITTED;
    g_status.health = CNC_V2_TRANSPORT_HEALTH_NOMINAL;
    g_status.last_error = CNC_V2_TRANSPORT_ERR_NONE;

    if (!send_frame_bytes(&decision.frame)) {
        g_status.health = CNC_V2_TRANSPORT_HEALTH_DEGRADED;
        g_status.last_error = CNC_V2_TRANSPORT_ERR_TX_QUEUE;
        publish_status();
        return;
    }

    g_waiting_for_response = true;
    g_status.phase = CNC_V2_TRANSPORT_WAITING_RESPONSE;
    publish_status();
    sddf_timer_set_timeout(timer_config.driver_id, CNC_V2_TRANSPORT_TIMEOUT_NS);
}
