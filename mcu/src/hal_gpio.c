/* SPDX-License-Identifier: MIT */

#include <avr/interrupt.h>
#include <avr/io.h>

#include <cnc_mcu_v2/hal_gpio.h>
#include <cnc_mcu_v2/module_pins.h>

/* ------------------------------------------------------------------ */
/* Buzzer                                                              */
/* ------------------------------------------------------------------ */

void hal_buzzer_init(void)
{
    PIN_BUZZER_DDR |= (1 << PIN_BUZZER_BIT);
    PIN_BUZZER_PORT &= ~(1 << PIN_BUZZER_BIT);
}

void hal_buzzer_on(void)
{
    PIN_BUZZER_PORT |= (1 << PIN_BUZZER_BIT);
}

void hal_buzzer_off(void)
{
    PIN_BUZZER_PORT &= ~(1 << PIN_BUZZER_BIT);
}

void hal_buzzer_toggle(void)
{
    PIN_BUZZER_PORT ^= (1 << PIN_BUZZER_BIT);
}

/* ------------------------------------------------------------------ */
/* RGB LED — software PWM (32-step, ~31 Hz at 1 kHz tick)             */
/* ------------------------------------------------------------------ */

/*
 * Duty values are 0-31.  The caller provides 0-255 which we scale
 * down by >> 3 so the public API stays familiar.
 */
static volatile uint8_t g_rgb_r;
static volatile uint8_t g_rgb_g;
static volatile uint8_t g_rgb_b;
static uint8_t g_pwm_counter;

void hal_rgb_init(void)
{
    PIN_RGB_R_DDR |= (1 << PIN_RGB_R_BIT);
    PIN_RGB_G_DDR |= (1 << PIN_RGB_G_BIT);
    PIN_RGB_B_DDR |= (1 << PIN_RGB_B_BIT);
    PIN_RGB_R_PORT &= ~(1 << PIN_RGB_R_BIT);
    PIN_RGB_G_PORT &= ~(1 << PIN_RGB_G_BIT);
    PIN_RGB_B_PORT &= ~(1 << PIN_RGB_B_BIT);
}

void hal_rgb_set(uint8_t r, uint8_t g, uint8_t b)
{
    g_rgb_r = r >> 3;
    g_rgb_g = g >> 3;
    g_rgb_b = b >> 3;
}

void hal_rgb_tick(void)
{
    g_pwm_counter = (g_pwm_counter + 1u) & 0x1Fu; /* 0-31 */

    uint8_t set = 0;
    uint8_t clr = 0;

    if (g_pwm_counter < g_rgb_r) set |= (1 << PIN_RGB_R_BIT);
    else                          clr |= (1 << PIN_RGB_R_BIT);

    if (g_pwm_counter < g_rgb_g) set |= (1 << PIN_RGB_G_BIT);
    else                          clr |= (1 << PIN_RGB_G_BIT);

    if (g_pwm_counter < g_rgb_b) set |= (1 << PIN_RGB_B_BIT);
    else                          clr |= (1 << PIN_RGB_B_BIT);

    PIN_RGB_R_PORT = (PIN_RGB_R_PORT & ~PIN_RGB_MASK) | set;
}

/* ------------------------------------------------------------------ */
/* Rotary encoder — PCINT-based click accumulator                      */
/* ------------------------------------------------------------------ */

static volatile int8_t g_encoder_accum;
static uint8_t g_encoder_last;

void hal_encoder_init(void)
{
    /* Input with pull-up */
    PIN_ENC_CLK_DDR &= ~(1 << PIN_ENC_CLK_BIT);
    PIN_ENC_CLK_PORT |= (1 << PIN_ENC_CLK_BIT);

    /* Read initial state */
    g_encoder_last = (PIN_ENC_CLK_PIN >> PIN_ENC_CLK_BIT) & 1u;

    /* Enable PCINT for encoder pin */
    PIN_ENC_CLK_PCMSK |= (1 << PIN_ENC_CLK_PCINT);
    PCICR |= (1 << PIN_ENC_CLK_PCIE);
}

int8_t hal_encoder_delta(void)
{
    uint8_t sreg = SREG;
    int8_t val;

    cli();
    val = g_encoder_accum;
    g_encoder_accum = 0;
    SREG = sreg;

    return val;
}

/*
 * PCINT1 ISR covers all PORTC pin-change interrupts (PCINT8-14).
 * Only the encoder pin is enabled in PCMSK1, so this fires only
 * for encoder transitions.
 *
 * Simple edge detection: count falling edges as one click.
 * For a proper quadrature encoder, a second pin (DT) and
 * direction logic would be needed here.
 */
ISR(PCINT1_vect)
{
    uint8_t current = (PIN_ENC_CLK_PIN >> PIN_ENC_CLK_BIT) & 1u;

    if (g_encoder_last && !current) {
        /* Falling edge — count as one click */
        g_encoder_accum++;
    }
    g_encoder_last = current;
}
