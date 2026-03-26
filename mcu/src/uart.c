/* SPDX-License-Identifier: MIT */

#include <avr/io.h>
#include <avr/interrupt.h>

#include <cnc_mcu_v2/config.h>
#include <cnc_mcu_v2/protocol.h>
#include <cnc_mcu_v2/uart.h>

static cnc_rx_fsm_t g_rx_fsm;
static cnc_frame_t g_rx_ring[CNC_MCU_V2_CMD_BUF_SIZE];
static volatile uint8_t g_rx_head;
static volatile uint8_t g_rx_tail;

ISR(USART_RX_vect)
{
    uint8_t byte = UDR0;

    if (!cnc_rx_feed(&g_rx_fsm, byte)) {
        return;
    }

    if (!cnc_frame_valid(&g_rx_fsm.frame)) {
        return;
    }

    {
        uint8_t next = (uint8_t)((g_rx_head + 1u) % CNC_MCU_V2_CMD_BUF_SIZE);
        if (next == g_rx_tail) {
            return;
        }

        g_rx_ring[g_rx_head] = g_rx_fsm.frame;
        g_rx_head = next;
    }
}

uint8_t cnc_uart_rx_available(void)
{
    return g_rx_head != g_rx_tail;
}

uint8_t cnc_uart_rx_get(cnc_frame_t *out)
{
    if (g_rx_head == g_rx_tail) {
        return 0u;
    }

    *out = g_rx_ring[g_rx_tail];
    g_rx_tail = (uint8_t)((g_rx_tail + 1u) % CNC_MCU_V2_CMD_BUF_SIZE);
    return 1u;
}

void cnc_uart_init(uint32_t baud)
{
    uint16_t ubrr = (uint16_t)(F_CPU / (8UL * baud) - 1UL);

    UBRR0H = (uint8_t)(ubrr >> 8);
    UBRR0L = (uint8_t)(ubrr);
    UCSR0A = (1 << U2X0);
    UCSR0B = (1 << RXEN0) | (1 << TXEN0) | (1 << RXCIE0);
    UCSR0C = (1 << UCSZ01) | (1 << UCSZ00);

    cnc_rx_init(&g_rx_fsm);
    g_rx_head = 0u;
    g_rx_tail = 0u;
}

void cnc_uart_putc(uint8_t c)
{
    while (!(UCSR0A & (1 << UDRE0))) {
    }
    UDR0 = c;
}

void cnc_uart_send(const uint8_t *data, uint8_t len)
{
    for (uint8_t i = 0u; i < len; i++) {
        cnc_uart_putc(data[i]);
    }
}
