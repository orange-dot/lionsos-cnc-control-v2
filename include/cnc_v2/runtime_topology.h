/*
 * Shared topology constants for LionsOS CNC Control V2.
 *
 * SPDX-License-Identifier: MIT
 */

#pragma once

#include <stdint.h>

#define CNC_V2_REGION_SIZE 0x1000u
#define CNC_V2_STATE_TICK_NS 100000000ull
#define CNC_V2_TRANSPORT_TIMEOUT_NS 250000000ull

/*
 * Generation counter convention:
 * - 0 is reserved for "uninitialized" (never used as a stable value)
 * - All init() functions must set generation to CNC_V2_GENERATION_INIT
 * - This avoids ABA on uint32_t wraparound (0xFFFFFFFF + 1 = 0 = uninitialized)
 */
#define CNC_V2_GENERATION_INIT 2u

enum cnc_v2_channel {
    CNC_V2_CH_JOB_TO_STATE = 0u,
    CNC_V2_CH_STATE_TO_PLANNER = 1u,
    CNC_V2_CH_PLANNER_TO_SAFETY = 2u,
    CNC_V2_CH_SAFETY_TO_XPORT = 3u,
    CNC_V2_CH_XPORT_TO_STATE = 4u,
    CNC_V2_CH_STATE_TO_STORE = 5u,
    CNC_V2_CH_STATE_TO_OBS = 6u,
    CNC_V2_CH_PLANNER_TO_OBS = 7u,
    CNC_V2_CH_SAFETY_TO_OBS = 8u,
    CNC_V2_CH_XPORT_TO_OBS = 9u,
    CNC_V2_CH_STORE_TO_OBS = 10u,
};

enum cnc_v2_service_id {
    CNC_V2_SVC_JOB_INGRESS = 1u,
    CNC_V2_SVC_STATE_CORE = 2u,
    CNC_V2_SVC_PLANNER = 3u,
    CNC_V2_SVC_SAFETY = 4u,
    CNC_V2_SVC_MCU_TRANSPORT = 5u,
    CNC_V2_SVC_SESSION_STORE = 6u,
    CNC_V2_SVC_OBSERVABILITY = 7u,
};

enum cnc_v2_region_id {
    CNC_V2_REGION_JOB_SUBMIT = 1u,
    CNC_V2_REGION_STATE_SNAPSHOT = 2u,
    CNC_V2_REGION_PLANNER_REQUEST = 3u,
    CNC_V2_REGION_MOTION_CANDIDATE = 4u,
    CNC_V2_REGION_PLANNER_STATUS = 5u,
    CNC_V2_REGION_SAFETY_DECISION = 6u,
    CNC_V2_REGION_SAFETY_STATUS = 7u,
    CNC_V2_REGION_TRANSPORT_STATUS = 8u,
    CNC_V2_REGION_STORE_REQUEST = 9u,
    CNC_V2_REGION_STORE_STATUS = 10u,
    CNC_V2_REGION_OBSERVABILITY_REPORT = 11u,
};
