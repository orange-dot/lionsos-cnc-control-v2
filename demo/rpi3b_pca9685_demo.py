import time
from machine import I2C


PCA9685_ADDR = 0x40
MODE1 = 0x00
MODE2 = 0x01
PRESCALE = 0xFE
LED0_ON_L = 0x06

SERVO_HZ = 50
SERVO_PERIOD_US = 20000
PULSE_MIN_US = 1000
PULSE_CENTER_US = 1500
PULSE_MAX_US = 2000

CHANNEL_YAW = 0
CHANNEL_PITCH = 14
CHANNEL_ROLL = 15

HAMMER_DELAY_MS = 20
HAMMER_LONG_DELAY_MS = 120
ACK_LOG_EVERY = 20

write_count = 0
write_failures = 0


def log(msg):
    print("GIMBAL|INFO:", msg)


def log_error(msg):
    print("GIMBAL|ERROR:", msg)


def clamp(value, lo, hi):
    if value < lo:
        return lo
    if value > hi:
        return hi
    return value


def reg_write(i2c, reg, value):
    return i2c_write(i2c, bytes((reg, value & 0xFF)), "reg 0x%02x" % reg)


def write_pwm(i2c, channel, on_count, off_count):
    base = LED0_ON_L + (channel * 4)
    payload = bytearray(5)
    payload[0] = base
    payload[1] = on_count & 0xFF
    payload[2] = (on_count >> 8) & 0x0F
    payload[3] = off_count & 0xFF
    payload[4] = (off_count >> 8) & 0x0F
    return i2c_write(i2c, payload, "pwm ch%d" % channel)


def i2c_write(i2c, payload, label):
    global write_count, write_failures

    try:
        i2c.writeto(PCA9685_ADDR, payload)
        write_count += 1
        if write_count <= 8 or (write_count % ACK_LOG_EVERY) == 0:
            log("i2c ack %s count=%d" % (label, write_count))
        return True
    except Exception as exc:
        write_failures += 1
        log_error("i2c fail %s count=%d failures=%d exc=%s" % (
            label,
            write_count,
            write_failures,
            exc,
        ))
        return False


def pulse_us_to_counts(pulse_us):
    return int((pulse_us * 4096) / SERVO_PERIOD_US)


def normalised_to_pulse_us(value):
    value = clamp(value, -1000, 1000)
    if value < 0:
        span = PULSE_CENTER_US - PULSE_MIN_US
        return PULSE_CENTER_US + int((value * span) / 1000)
    span = PULSE_MAX_US - PULSE_CENTER_US
    return PULSE_CENTER_US + int((value * span) / 1000)


def set_axis(i2c, channel, value):
    pulse_us = normalised_to_pulse_us(value)
    write_pwm(i2c, channel, 0, pulse_us_to_counts(pulse_us))


def set_pose(i2c, yaw, pitch, roll):
    set_axis(i2c, CHANNEL_YAW, yaw)
    set_axis(i2c, CHANNEL_PITCH, pitch)
    set_axis(i2c, CHANNEL_ROLL, roll)


def configure_pca9685(i2c):
    prescale = int((25000000 / (4096 * SERVO_HZ)) - 1)

    reg_write(i2c, MODE1, 0x10)
    reg_write(i2c, PRESCALE, prescale)
    reg_write(i2c, MODE2, 0x04)
    reg_write(i2c, MODE1, 0x20)
    time.sleep_ms(5)
    reg_write(i2c, MODE1, 0xA0)


def hammer_cycle(i2c, phase):
    if phase:
        reg_write(i2c, MODE2, 0x04)
        set_pose(i2c, 320, -260, 220)
        set_axis(i2c, CHANNEL_YAW, -320)
        set_axis(i2c, CHANNEL_PITCH, 260)
        set_axis(i2c, CHANNEL_ROLL, -220)
    else:
        reg_write(i2c, MODE2, 0x04)
        set_pose(i2c, -320, 260, -220)
        set_axis(i2c, CHANNEL_YAW, 320)
        set_axis(i2c, CHANNEL_PITCH, -260)
        set_axis(i2c, CHANNEL_ROLL, 220)


def main():
    log("i2c hammer init")
    try:
        i2c = I2C(1, freq=400000)
        log("i2c bus ready addr=0x40 freq=400000")
        configure_pca9685(i2c)
        log("pca9685 hammer ready")
    except Exception as exc:
        print("GIMBAL|ERROR: init failed:", exc)
        while True:
            time.sleep_ms(1000)

    phase = 0
    while True:
        hammer_cycle(i2c, phase)
        time.sleep_ms(HAMMER_DELAY_MS)
        phase = 1 - phase
        hammer_cycle(i2c, phase)
        if (write_count % ACK_LOG_EVERY) == 0:
            log("i2c heartbeat writes=%d failures=%d phase=%d" % (
                write_count,
                write_failures,
                phase,
            ))
        time.sleep_ms(HAMMER_LONG_DELAY_MS)


main()
