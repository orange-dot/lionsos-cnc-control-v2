/*
 * Safety coordinator for LionsOS CNC Control V2.
 *
 * SPDX-License-Identifier: MIT
 */

#include <stdint.h>

#include <microkit.h>

#include <cnc_v2/config.h>
#include <cnc_v2/control_ipc.h>
#include <cnc_v2/safety.h>

__attribute__((__section__(".safety_coordinator_config"))) cnc_v2_safety_coordinator_config_t config;

static const cnc_soft_limits_t g_limits = CNC_DEFAULT_LIMITS;
static cnc_v2_safety_status_t g_status;
static uint32_t g_last_motion_generation;

static volatile cnc_v2_motion_candidate_t *motion_candidate_page(void)
{
    return (volatile cnc_v2_motion_candidate_t *)(uintptr_t)config.motion_candidate_vaddr;
}

static volatile cnc_v2_safety_decision_t *safety_decision_page(void)
{
    return (volatile cnc_v2_safety_decision_t *)(uintptr_t)config.safety_decision_vaddr;
}

static volatile cnc_v2_safety_status_t *safety_status_page(void)
{
    return (volatile cnc_v2_safety_status_t *)(uintptr_t)config.safety_status_vaddr;
}

static void publish_status(void)
{
    cnc_v2_publish_safety_status(safety_status_page(), &g_status);
    microkit_notify(config.observability_ch);
}

void init(void)
{
    if (!cnc_v2_config_magic_valid(config.magic)) {
        microkit_dbg_puts("safety_coordinator: bad config magic\n");
        return;
    }

    g_status.generation = CNC_V2_GENERATION_INIT;
    publish_status();
}

void notified(microkit_channel ch)
{
    cnc_v2_motion_candidate_t candidate;
    cnc_v2_safety_decision_t decision = {0};
    uint8_t reason = 0u;

    if (ch != CNC_V2_CH_PLANNER_TO_SAFETY) {
        return;
    }
    if (!cnc_v2_read_motion_candidate(motion_candidate_page(), &candidate)) {
        return;
    }
    if (candidate.generation == 0u || candidate.generation == g_last_motion_generation) {
        return;
    }

    g_last_motion_generation = candidate.generation;
    g_status.inspected_count += 1u;
    g_status.last_cmd_seq = candidate.frame.seq;

    decision.decision_generation = g_status.inspected_count;
    decision.last_cmd_seq = candidate.frame.seq;
    decision.frame = candidate.frame;

    if (cnc_safety_check(&candidate.frame, &g_limits, &reason)) {
        decision.result = CNC_V2_SAFETY_APPROVED;
        g_status.last_result = CNC_V2_SAFETY_APPROVED;
        g_status.last_reason = 0u;
        g_status.approved_count += 1u;
    } else {
        decision.result = CNC_V2_SAFETY_BLOCKED;
        decision.reason = reason;
        g_status.last_result = CNC_V2_SAFETY_BLOCKED;
        g_status.last_reason = reason;
        g_status.blocked_count += 1u;
    }

    cnc_v2_publish_safety_decision(safety_decision_page(), &decision);
    publish_status();
    microkit_notify(config.transport_ch);
}
