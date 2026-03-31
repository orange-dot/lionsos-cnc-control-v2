/*
 * Shared-memory contracts for LionsOS CNC Control V2.
 *
 * SPDX-License-Identifier: MIT
 */

#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include <cnc_v2/runtime_topology.h>
#include <cnc_v2/wire_protocol.h>

#define CNC_V2_SNAPSHOT_MAX_RETRIES 32u
#define CNC_V2_JOB_NAME_MAX 32u

enum cnc_v2_job_kind {
    CNC_V2_JOB_KIND_NONE = 0u,
    CNC_V2_JOB_KIND_MOVE = 1u,
    CNC_V2_JOB_KIND_HOME = 2u,
};

enum cnc_v2_lifecycle_state {
    CNC_V2_LIFECYCLE_BOOT = 0u,
    CNC_V2_LIFECYCLE_IDLE = 1u,
    CNC_V2_LIFECYCLE_JOB_ACCEPTED = 2u, /* NOTE: currently unused -- lifecycle transitions directly to PLANNED */
    CNC_V2_LIFECYCLE_PLANNED = 3u,
    CNC_V2_LIFECYCLE_SAFETY_REVIEWED = 4u,
    CNC_V2_LIFECYCLE_TRANSPORT_PENDING = 5u,
    CNC_V2_LIFECYCLE_MACHINE_UPDATED = 6u,
    CNC_V2_LIFECYCLE_DIVERGED = 7u,
};

enum cnc_v2_safety_result {
    CNC_V2_SAFETY_NONE = 0u,
    CNC_V2_SAFETY_APPROVED = 1u,
    CNC_V2_SAFETY_BLOCKED = 2u,
};

enum cnc_v2_transport_phase {
    CNC_V2_TRANSPORT_IDLE = 0u,
    CNC_V2_TRANSPORT_BLOCKED = 1u,
    CNC_V2_TRANSPORT_TX_SUBMITTED = 2u,
    CNC_V2_TRANSPORT_WAITING_RESPONSE = 3u,
    CNC_V2_TRANSPORT_RESPONSE_OK = 4u,
    CNC_V2_TRANSPORT_TIMEOUT = 5u,
};

enum cnc_v2_transport_health {
    CNC_V2_TRANSPORT_HEALTH_UNKNOWN = 0u,
    CNC_V2_TRANSPORT_HEALTH_NOMINAL = 1u,
    CNC_V2_TRANSPORT_HEALTH_DEGRADED = 2u,
};

enum cnc_v2_store_result {
    CNC_V2_STORE_NONE = 0u,
    CNC_V2_STORE_OK = 1u,
};

enum cnc_v2_transport_error {
    CNC_V2_TRANSPORT_ERR_NONE = 0u,
    CNC_V2_TRANSPORT_ERR_TX_QUEUE = 0x80u,
    CNC_V2_TRANSPORT_ERR_TIMEOUT = 0x81u,
    CNC_V2_TRANSPORT_ERR_BAD_CRC = 0x82u,
};

enum cnc_v2_health_bit {
    CNC_V2_HEALTH_HAVE_JOB = (1u << 0),
    CNC_V2_HEALTH_MACHINE_ACTIVE = (1u << 1),
    CNC_V2_HEALTH_SAFETY_BLOCKED = (1u << 2),
    CNC_V2_HEALTH_TRANSPORT_DEGRADED = (1u << 3),
    CNC_V2_HEALTH_STORE_READY = (1u << 4),
};

typedef struct {
    uint32_t generation;
    uint32_t job_id;
    int16_t target_x;
    int16_t target_y;
    int16_t target_z;
    uint16_t feedrate;
    uint8_t kind;
    uint8_t reserved[5];
    char name[CNC_V2_JOB_NAME_MAX];
} cnc_v2_job_submit_t;

typedef struct {
    uint32_t generation;
    uint32_t state_generation;
    uint32_t current_job_id;
    uint32_t planner_request_generation;
    uint32_t watchdog_ticks;
    uint8_t lifecycle;
    uint8_t machine_state;
    uint8_t last_error;
    uint8_t last_cmd_seq;
    uint8_t reserved[4];
} cnc_v2_state_snapshot_t;

typedef struct {
    uint32_t generation;
    uint32_t request_generation;
    uint32_t job_id;
    int16_t target_x;
    int16_t target_y;
    int16_t target_z;
    uint16_t feedrate;
    uint8_t kind;
    uint8_t reserved[5];
} cnc_v2_planner_request_t;

typedef struct {
    uint32_t generation;
    uint32_t plan_generation;
    uint32_t request_generation;
    uint8_t plan_kind;
    uint8_t reserved[3];
    cnc_frame_t frame;
} cnc_v2_motion_candidate_t;

typedef struct {
    uint32_t generation;
    uint32_t request_generation;
    uint32_t produced_frames;
    uint8_t last_cmd_seq;
    uint8_t last_kind;
    uint8_t reserved[2];
} cnc_v2_planner_status_t;

typedef struct {
    uint32_t generation;
    uint32_t decision_generation;
    uint8_t result;
    uint8_t reason;
    uint8_t last_cmd_seq;
    uint8_t reserved;
    cnc_frame_t frame;
} cnc_v2_safety_decision_t;

typedef struct {
    uint32_t generation;
    uint32_t inspected_count;
    uint32_t approved_count;
    uint32_t blocked_count;
    uint8_t last_result;
    uint8_t last_reason;
    uint8_t last_cmd_seq;
    uint8_t reserved;
} cnc_v2_safety_status_t;

typedef struct {
    uint32_t generation;
    uint32_t tx_count;
    uint32_t rx_count;
    uint32_t timeout_count;
    uint8_t phase;
    uint8_t health;
    uint8_t last_error;
    uint8_t machine_state;
    uint8_t last_cmd_seq;
    uint8_t last_rsp_seq;
    uint8_t reserved[2];
    cnc_frame_t last_tx_frame;
    cnc_frame_t last_rx_frame;
} cnc_v2_transport_status_t;

typedef struct {
    uint32_t generation;
    uint32_t state_generation;
    uint32_t job_id;
    uint8_t lifecycle;
    uint8_t machine_state;
    uint8_t reserved[2];
} cnc_v2_store_request_t;

typedef struct {
    uint32_t generation;
    uint32_t persisted_generation;
    uint32_t persisted_job_id;
    uint32_t writes_completed;
    uint8_t result;
    uint8_t reserved[3];
} cnc_v2_store_status_t;

typedef struct {
    uint32_t generation;
    uint32_t events_seen;
    uint32_t state_generation;
    uint32_t planner_generation;
    uint32_t safety_generation;
    uint32_t transport_generation;
    uint32_t store_generation;
    uint32_t active_job_id;
    uint32_t health_bits;
    uint8_t lifecycle;
    uint8_t machine_state;
    uint8_t transport_phase;
    uint8_t safety_result;
    uint8_t store_result;
    uint8_t last_error;
    uint8_t reserved[2];
} cnc_v2_observability_report_t;

static inline uint32_t cnc_v2_gen_load(const volatile uint32_t *generation)
{
    return __atomic_load_n(generation, __ATOMIC_ACQUIRE);
}

static inline void cnc_v2_gen_store(volatile uint32_t *generation, uint32_t value)
{
    __atomic_store_n(generation, value, __ATOMIC_RELEASE);
}

static inline uint32_t cnc_v2_publish_begin(volatile uint32_t *generation)
{
    uint32_t start = cnc_v2_gen_load(generation);

    if ((start & 1u) == 0u) {
        start += 1u;
    } else {
        start += 2u;
    }
    cnc_v2_gen_store(generation, start);
    __atomic_signal_fence(__ATOMIC_ACQ_REL);
    return start;
}

static inline void cnc_v2_publish_end(volatile uint32_t *generation, uint32_t begin)
{
    __atomic_signal_fence(__ATOMIC_ACQ_REL);
    cnc_v2_gen_store(generation, begin + 1u);
}

static inline bool cnc_v2_snap_begin(const volatile uint32_t *generation, uint32_t *out)
{
    *out = cnc_v2_gen_load(generation);
    return ((*out & 1u) == 0u);
}

static inline bool cnc_v2_snap_accept(const volatile uint32_t *generation, uint32_t start)
{
    return cnc_v2_gen_load(generation) == start;
}

static inline void cnc_v2_copy_cstring(char *dst, uint32_t capacity, const char *src)
{
    uint32_t i;

    if (capacity == 0u) {
        return;
    }

    for (i = 0u; i + 1u < capacity && src[i] != '\0'; i++) {
        dst[i] = src[i];
    }
    dst[i] = '\0';
    for (i = i + 1u; i < capacity; i++) {
        dst[i] = '\0';
    }
}

#define CNC_V2_DEFINE_SNAPSHOT_IO(type, suffix) \
static inline void cnc_v2_publish_##suffix(volatile type *dst, const type *src) \
{ \
    uint32_t begin = cnc_v2_publish_begin(&dst->generation); \
    volatile uint8_t *dst_bytes = (volatile uint8_t *)dst; \
    const uint8_t *src_bytes = (const uint8_t *)src; \
    for (size_t i = sizeof(uint32_t); i < sizeof(type); i++) { \
        dst_bytes[i] = src_bytes[i]; \
    } \
    cnc_v2_publish_end(&dst->generation, begin); \
} \
static inline bool cnc_v2_read_##suffix(const volatile type *src, type *dst) \
{ \
    type tmp = {0}; \
    uint32_t start; \
    uint32_t retries = CNC_V2_SNAPSHOT_MAX_RETRIES; \
    do { \
        if (!cnc_v2_snap_begin(&src->generation, &start)) { \
            continue; \
        } \
        if (retries-- == 0u) { \
            return false; \
        } \
        __atomic_signal_fence(__ATOMIC_ACQ_REL); \
        const volatile uint8_t *src_bytes = (const volatile uint8_t *)src; \
        uint8_t *dst_bytes = (uint8_t *)&tmp; \
        for (size_t i = sizeof(uint32_t); i < sizeof(type); i++) { \
            dst_bytes[i] = src_bytes[i]; \
        } \
        __atomic_signal_fence(__ATOMIC_ACQ_REL); \
    } while (!cnc_v2_snap_accept(&src->generation, start)); \
    tmp.generation = start; \
    *dst = tmp; \
    return true; \
}

CNC_V2_DEFINE_SNAPSHOT_IO(cnc_v2_job_submit_t, job_submit)
CNC_V2_DEFINE_SNAPSHOT_IO(cnc_v2_state_snapshot_t, state_snapshot)
CNC_V2_DEFINE_SNAPSHOT_IO(cnc_v2_planner_request_t, planner_request)
CNC_V2_DEFINE_SNAPSHOT_IO(cnc_v2_motion_candidate_t, motion_candidate)
CNC_V2_DEFINE_SNAPSHOT_IO(cnc_v2_planner_status_t, planner_status)
CNC_V2_DEFINE_SNAPSHOT_IO(cnc_v2_safety_decision_t, safety_decision)
CNC_V2_DEFINE_SNAPSHOT_IO(cnc_v2_safety_status_t, safety_status)
CNC_V2_DEFINE_SNAPSHOT_IO(cnc_v2_transport_status_t, transport_status)
CNC_V2_DEFINE_SNAPSHOT_IO(cnc_v2_store_request_t, store_request)
CNC_V2_DEFINE_SNAPSHOT_IO(cnc_v2_store_status_t, store_status)
CNC_V2_DEFINE_SNAPSHOT_IO(cnc_v2_observability_report_t, observability_report)

_Static_assert(sizeof(cnc_v2_job_submit_t) <= CNC_V2_REGION_SIZE, "job submit fits in region");
_Static_assert(sizeof(cnc_v2_state_snapshot_t) <= CNC_V2_REGION_SIZE, "state snapshot fits in region");
_Static_assert(sizeof(cnc_v2_planner_request_t) <= CNC_V2_REGION_SIZE, "planner request fits in region");
_Static_assert(sizeof(cnc_v2_motion_candidate_t) <= CNC_V2_REGION_SIZE, "motion candidate fits in region");
_Static_assert(sizeof(cnc_v2_planner_status_t) <= CNC_V2_REGION_SIZE, "planner status fits in region");
_Static_assert(sizeof(cnc_v2_safety_decision_t) <= CNC_V2_REGION_SIZE, "safety decision fits in region");
_Static_assert(sizeof(cnc_v2_safety_status_t) <= CNC_V2_REGION_SIZE, "safety status fits in region");
_Static_assert(sizeof(cnc_v2_transport_status_t) <= CNC_V2_REGION_SIZE, "transport status fits in region");
_Static_assert(sizeof(cnc_v2_store_request_t) <= CNC_V2_REGION_SIZE, "store request fits in region");
_Static_assert(sizeof(cnc_v2_store_status_t) <= CNC_V2_REGION_SIZE, "store status fits in region");
_Static_assert(sizeof(cnc_v2_observability_report_t) <= CNC_V2_REGION_SIZE, "observability report fits in region");

_Static_assert(_Alignof(cnc_v2_job_submit_t) <= 4, "job submit alignment exceeds 4-byte shared-region guarantee");
_Static_assert(_Alignof(cnc_v2_state_snapshot_t) <= 4, "state snapshot alignment exceeds 4-byte shared-region guarantee");
_Static_assert(_Alignof(cnc_v2_planner_request_t) <= 4, "planner request alignment exceeds 4-byte shared-region guarantee");
_Static_assert(_Alignof(cnc_v2_motion_candidate_t) <= 4, "motion candidate alignment exceeds 4-byte shared-region guarantee");
_Static_assert(_Alignof(cnc_v2_planner_status_t) <= 4, "planner status alignment exceeds 4-byte shared-region guarantee");
_Static_assert(_Alignof(cnc_v2_safety_decision_t) <= 4, "safety decision alignment exceeds 4-byte shared-region guarantee");
_Static_assert(_Alignof(cnc_v2_safety_status_t) <= 4, "safety status alignment exceeds 4-byte shared-region guarantee");
_Static_assert(_Alignof(cnc_v2_transport_status_t) <= 4, "transport status alignment exceeds 4-byte shared-region guarantee");
_Static_assert(_Alignof(cnc_v2_store_request_t) <= 4, "store request alignment exceeds 4-byte shared-region guarantee");
_Static_assert(_Alignof(cnc_v2_store_status_t) <= 4, "store status alignment exceeds 4-byte shared-region guarantee");
_Static_assert(_Alignof(cnc_v2_observability_report_t) <= 4, "observability report alignment exceeds 4-byte shared-region guarantee");
