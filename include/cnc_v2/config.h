/*
 * Binary config blob layouts for LionsOS CNC Control V2 components.
 *
 * packed is load-bearing because meta.py serialises these blobs with
 * fixed struct.pack layouts.
 *
 * SPDX-License-Identifier: MIT
 */

#pragma once

#include <stdbool.h>
#include <stdint.h>

#define CNC_V2_CONFIG_MAGIC 0x32434E43u

static inline bool cnc_v2_config_magic_valid(uint32_t magic)
{
    return magic == CNC_V2_CONFIG_MAGIC;
}

typedef struct __attribute__((packed)) {
    uint32_t magic;
    uint32_t flags;
    uint64_t job_submit_vaddr;
    uint32_t state_core_ch;
} cnc_v2_job_ingress_config_t;

typedef struct __attribute__((packed)) {
    uint32_t magic;
    uint32_t flags;
    uint64_t job_submit_vaddr;
    uint64_t state_snapshot_vaddr;
    uint64_t planner_request_vaddr;
    uint64_t store_request_vaddr;
    uint64_t transport_status_vaddr;
    uint32_t planner_ch;
    uint32_t store_ch;
    uint32_t observability_ch;
} cnc_v2_state_core_config_t;

typedef struct __attribute__((packed)) {
    uint32_t magic;
    uint32_t flags;
    uint64_t planner_request_vaddr;
    uint64_t motion_candidate_vaddr;
    uint64_t planner_status_vaddr;
    uint32_t safety_ch;
    uint32_t observability_ch;
} cnc_v2_planner_config_t;

typedef struct __attribute__((packed)) {
    uint32_t magic;
    uint32_t flags;
    uint64_t motion_candidate_vaddr;
    uint64_t safety_decision_vaddr;
    uint64_t safety_status_vaddr;
    uint32_t transport_ch;
    uint32_t observability_ch;
} cnc_v2_safety_coordinator_config_t;

typedef struct __attribute__((packed)) {
    uint32_t magic;
    uint32_t flags;
    uint64_t safety_decision_vaddr;
    uint64_t transport_status_vaddr;
    uint32_t state_core_ch;
    uint32_t observability_ch;
} cnc_v2_mcu_transport_config_t;

typedef struct __attribute__((packed)) {
    uint32_t magic;
    uint32_t flags;
    uint64_t store_request_vaddr;
    uint64_t store_status_vaddr;
    uint32_t observability_ch;
} cnc_v2_session_store_config_t;

typedef struct __attribute__((packed)) {
    uint32_t magic;
    uint32_t flags;
    uint64_t state_snapshot_vaddr;
    uint64_t planner_status_vaddr;
    uint64_t safety_status_vaddr;
    uint64_t transport_status_vaddr;
    uint64_t store_status_vaddr;
    uint64_t report_vaddr;
} cnc_v2_observability_config_t;

_Static_assert(sizeof(cnc_v2_job_ingress_config_t) == 20, "job ingress config size");
_Static_assert(sizeof(cnc_v2_state_core_config_t) == 60, "state core config size");
_Static_assert(sizeof(cnc_v2_planner_config_t) == 40, "planner config size");
_Static_assert(sizeof(cnc_v2_safety_coordinator_config_t) == 40, "safety config size");
_Static_assert(sizeof(cnc_v2_mcu_transport_config_t) == 32, "transport config size");
_Static_assert(sizeof(cnc_v2_session_store_config_t) == 28, "session store config size");
_Static_assert(sizeof(cnc_v2_observability_config_t) == 56, "observability config size");
