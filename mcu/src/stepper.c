/* SPDX-License-Identifier: MIT */
/*
 * Stepper motion primitives — adapted from lionsos-cnc-control v1.
 * Bresenham line algorithm, limit switch safety, spindle PWM.
 */

#include <avr/io.h>
#include <util/delay.h>

#include <cnc_mcu_v2/config.h>
#include <cnc_mcu_v2/pins.h>
#include <cnc_mcu_v2/stepper.h>

/* ------------------------------------------------------------------ */
/* State                                                               */
/* ------------------------------------------------------------------ */

static uint8_t g_limit_latched;

/* ------------------------------------------------------------------ */
/* Helpers                                                             */
/* ------------------------------------------------------------------ */

static int16_t iabs(int16_t v) { return v < 0 ? -v : v; }

static void pulse_x(void)
{
    PIN_X_STEP_PORT |= (1 << PIN_X_STEP_BIT);
    _delay_us(CNC_MCU_V2_STEP_PULSE_US);
    PIN_X_STEP_PORT &= ~(1 << PIN_X_STEP_BIT);
}

static void pulse_y(void)
{
    PIN_Y_STEP_PORT |= (1 << PIN_Y_STEP_BIT);
    _delay_us(CNC_MCU_V2_STEP_PULSE_US);
    PIN_Y_STEP_PORT &= ~(1 << PIN_Y_STEP_BIT);
}

static void pulse_z(void)
{
    PIN_Z_STEP_PORT |= (1 << PIN_Z_STEP_BIT);
    _delay_us(CNC_MCU_V2_STEP_PULSE_US);
    PIN_Z_STEP_PORT &= ~(1 << PIN_Z_STEP_BIT);
}

/* ------------------------------------------------------------------ */
/* Public API                                                          */
/* ------------------------------------------------------------------ */

void cnc_stepper_enable(void)
{
    PIN_ENABLE_PORT &= ~(1 << PIN_ENABLE_BIT);  /* active low */
    _delay_ms(50);
}

void cnc_stepper_disable(void)
{
    PIN_ENABLE_PORT |= (1 << PIN_ENABLE_BIT);
}

void cnc_spindle_set(uint8_t duty)
{
    if (duty == 0) {
        TCCR2A &= ~(1 << COM2A1);
        PIN_SPINDLE_PORT &= ~(1 << PIN_SPINDLE_BIT);
    } else {
        OCR2A = duty;
        TCCR2A |= (1 << COM2A1);
    }
}

void cnc_motion_stop(void)
{
    cnc_spindle_set(0);
    PIN_X_STEP_PORT &= ~(1 << PIN_X_STEP_BIT);
    PIN_Y_STEP_PORT &= ~(1 << PIN_Y_STEP_BIT);
    PIN_Z_STEP_PORT &= ~(1 << PIN_Z_STEP_BIT);
    cnc_stepper_disable();
}

uint8_t cnc_limit_hit(void)
{
    uint8_t mask = (uint8_t)((~PIN_LIMIT_PIN) & PIN_LIMIT_MASK);
    if (mask != 0) {
        g_limit_latched = mask;
        cnc_motion_stop();
        return mask;
    }
    return 0;
}

uint8_t cnc_limit_latched(void) { return g_limit_latched; }
void    cnc_limit_clear(void)   { g_limit_latched = 0; }

/* ------------------------------------------------------------------ */
/* 3-axis Bresenham                                                    */
/* ------------------------------------------------------------------ */

uint8_t cnc_line_to(int16_t *cx, int16_t *cy, int16_t *cz,
                    int16_t tx, int16_t ty, int16_t tz)
{
    int16_t dx = iabs(tx - *cx);
    int16_t dy = iabs(ty - *cy);
    int16_t dz = iabs(tz - *cz);
    int8_t sx = *cx < tx ? 1 : -1;
    int8_t sy = *cy < ty ? 1 : -1;
    int8_t sz = *cz < tz ? 1 : -1;

    /* Set direction pins */
    if (sx > 0) PIN_X_DIR_PORT &= ~(1 << PIN_X_DIR_BIT);
    else        PIN_X_DIR_PORT |=  (1 << PIN_X_DIR_BIT);
    if (sy > 0) PIN_Y_DIR_PORT &= ~(1 << PIN_Y_DIR_BIT);
    else        PIN_Y_DIR_PORT |=  (1 << PIN_Y_DIR_BIT);
    if (sz > 0) PIN_Z_DIR_PORT &= ~(1 << PIN_Z_DIR_BIT);
    else        PIN_Z_DIR_PORT |=  (1 << PIN_Z_DIR_BIT);
    _delay_us(5);

    /* Dominant axis drives the loop */
    int16_t dominant = dx;
    if (dy > dominant) dominant = dy;
    if (dz > dominant) dominant = dz;

    int16_t err_x = dominant / 2;
    int16_t err_y = dominant / 2;
    int16_t err_z = dominant / 2;

    for (int16_t i = 0; i < dominant; i++) {
        if (cnc_limit_hit()) return 1;

        err_x -= dx;
        if (err_x < 0) { err_x += dominant; pulse_x(); *cx += sx; }

        err_y -= dy;
        if (err_y < 0) { err_y += dominant; pulse_y(); *cy += sy; }

        err_z -= dz;
        if (err_z < 0) { err_z += dominant; pulse_z(); *cz += sz; }

        _delay_us(CNC_MCU_V2_STEP_DELAY_US);
    }

    return 0;
}

/* ------------------------------------------------------------------ */
/* Homing                                                              */
/* ------------------------------------------------------------------ */

uint8_t cnc_home_axis(uint8_t axis)
{
    uint8_t limit_bit;
    void (*pulse_fn)(void);
    volatile uint8_t *dir_port;
    uint8_t dir_bit;

    switch (axis) {
    case 0: /* X */
        limit_bit = PIN_X_LIMIT_BIT;
        pulse_fn = pulse_x;
        dir_port = &PIN_X_DIR_PORT;
        dir_bit = PIN_X_DIR_BIT;
        break;
    case 1: /* Y */
        limit_bit = PIN_Y_LIMIT_BIT;
        pulse_fn = pulse_y;
        dir_port = &PIN_Y_DIR_PORT;
        dir_bit = PIN_Y_DIR_BIT;
        break;
    case 2: /* Z */
        limit_bit = PIN_Z_LIMIT_BIT;
        pulse_fn = pulse_z;
        dir_port = &PIN_Z_DIR_PORT;
        dir_bit = PIN_Z_DIR_BIT;
        break;
    default:
        return 1;
    }

    /* Move toward limit (negative direction) */
    *dir_port |= (1 << dir_bit);
    _delay_us(5);

    while (!((~PIN_LIMIT_PIN) & (1 << limit_bit))) {
        pulse_fn();
        _delay_us(CNC_MCU_V2_STEP_DELAY_US);
    }

    /* Back off */
    *dir_port &= ~(1 << dir_bit);
    _delay_us(5);

    for (uint16_t i = 0; i < CNC_MCU_V2_HOME_BACKOFF; i++) {
        pulse_fn();
        _delay_us(CNC_MCU_V2_STEP_DELAY_US);
    }

    cnc_limit_clear();
    return 0;
}
