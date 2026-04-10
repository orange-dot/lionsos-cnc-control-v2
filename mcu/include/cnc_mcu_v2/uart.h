/* SPDX-License-Identifier: MIT */

#pragma once

#include <stdint.h>

void cnc_uart_init(uint32_t baud);
void cnc_uart_putc(uint8_t c);
void cnc_uart_send(const uint8_t *data, uint8_t len);
