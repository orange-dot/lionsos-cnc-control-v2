/* SPDX-License-Identifier: MIT */

#include <avr/interrupt.h>
#include <avr/io.h>

#include <cnc_mcu_v2/hal_gpio.h>
#include <cnc_mcu_v2/tick.h>

static volatile uint32_t g_tick_ms;

/*
 * Timer0 CTC mode, prescaler 64, OCR0A = 249.
 * 16 MHz / 64 / 250 = 1000 Hz → 1ms tick.
 */
void cnc_tick_init(void)
{
    TCCR0A = (1 << WGM01);          /* CTC mode */
    TCCR0B = (1 << CS01) | (1 << CS00);  /* prescaler 64 */
    OCR0A = 249u;
    TIMSK0 |= (1 << OCIE0A);        /* enable compare match A interrupt */
}

uint32_t cnc_tick_ms(void)
{
    uint32_t val;
    uint8_t sreg = SREG;

    cli();
    val = g_tick_ms;
    SREG = sreg;

    return val;
}

ISR(TIMER0_COMPA_vect)
{
    g_tick_ms++;
    hal_rgb_tick();
}
