/*
 * Structured read-mostly observability sink for LionsOS CNC Control V2.
 *
 * SPDX-License-Identifier: MIT
 */

#include <stdint.h>

#include <microkit.h>

#include <cnc_v2/config.h>
#include <cnc_v2/control_ipc.h>

__attribute__((__section__(".observability_config"))) cnc_v2_observability_config_t config;

static cnc_v2_observability_report_t g_report;

static volatile cnc_v2_state_snapshot_t *state_snapshot_page(void)
{
    return (volatile cnc_v2_state_snapshot_t *)(uintptr_t)config.state_snapshot_vaddr;
}

static volatile cnc_v2_planner_status_t *planner_status_page(void)
{
    return (volatile cnc_v2_planner_status_t *)(uintptr_t)config.planner_status_vaddr;
}

static volatile cnc_v2_safety_status_t *safety_status_page(void)
{
    return (volatile cnc_v2_safety_status_t *)(uintptr_t)config.safety_status_vaddr;
}

static volatile cnc_v2_transport_status_t *transport_status_page(void)
{
    return (volatile cnc_v2_transport_status_t *)(uintptr_t)config.transport_status_vaddr;
}

static volatile cnc_v2_store_status_t *store_status_page(void)
{
    return (volatile cnc_v2_store_status_t *)(uintptr_t)config.store_status_vaddr;
}

static volatile cnc_v2_observability_report_t *report_page(void)
{
    return (volatile cnc_v2_observability_report_t *)(uintptr_t)config.report_vaddr;
}

static void rebuild_report(void)
{
    cnc_v2_state_snapshot_t state_snapshot;
    cnc_v2_planner_status_t planner_status;
    cnc_v2_safety_status_t safety_status;
    cnc_v2_transport_status_t transport_status;
    cnc_v2_store_status_t store_status;
    uint32_t health_bits = 0u;

    if (cnc_v2_read_state_snapshot(state_snapshot_page(), &state_snapshot)) {
        g_report.state_generation = state_snapshot.state_generation;
        g_report.active_job_id = state_snapshot.current_job_id;
        g_report.lifecycle = state_snapshot.lifecycle;
        g_report.machine_state = state_snapshot.machine_state;
        g_report.last_error = state_snapshot.last_error;
    }

    if (cnc_v2_read_planner_status(planner_status_page(), &planner_status)) {
        g_report.planner_generation = planner_status.generation;
    }

    if (cnc_v2_read_safety_status(safety_status_page(), &safety_status)) {
        g_report.safety_generation = safety_status.generation;
        g_report.safety_result = safety_status.last_result;
    }

    /*
     * Transport status intentionally overrides state_snapshot for machine_state
     * and last_error -- transport is closer to MCU ground truth.
     */
    if (cnc_v2_read_transport_status(transport_status_page(), &transport_status)) {
        g_report.transport_generation = transport_status.generation;
        g_report.transport_phase = transport_status.phase;
        g_report.machine_state = transport_status.machine_state;
        g_report.last_error = transport_status.last_error;
    }

    if (cnc_v2_read_store_status(store_status_page(), &store_status)) {
        g_report.store_generation = store_status.generation;
        g_report.store_result = store_status.result;
    }

    if (g_report.active_job_id != 0u) {
        health_bits |= CNC_V2_HEALTH_HAVE_JOB;
    }
    if (g_report.machine_state == CNC_STATE_MOVING || g_report.machine_state == CNC_STATE_HOMING) {
        health_bits |= CNC_V2_HEALTH_MACHINE_ACTIVE;
    }
    if (g_report.safety_result == CNC_V2_SAFETY_BLOCKED) {
        health_bits |= CNC_V2_HEALTH_SAFETY_BLOCKED;
    }
    if (g_report.transport_phase == CNC_V2_TRANSPORT_TIMEOUT) {
        health_bits |= CNC_V2_HEALTH_TRANSPORT_DEGRADED;
    }
    if (g_report.store_result == CNC_V2_STORE_OK) {
        health_bits |= CNC_V2_HEALTH_STORE_READY;
    }

    g_report.health_bits = health_bits;
    g_report.events_seen += 1u;
    cnc_v2_publish_observability_report(report_page(), &g_report);
}

void init(void)
{
    if (!cnc_v2_config_magic_valid(config.magic)) {
        microkit_dbg_puts("observability: bad config magic\n");
        return;
    }

    g_report.generation = CNC_V2_GENERATION_INIT;
    rebuild_report();
}

void notified(microkit_channel ch)
{
    (void)ch;
    rebuild_report();
}
