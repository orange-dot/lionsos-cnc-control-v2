/*
 * Authoritative control state owner for LionsOS CNC Control V2.
 *
 * SPDX-License-Identifier: MIT
 */

#include <stdbool.h>
#include <stdint.h>

#include <microkit.h>
#include <sddf/timer/client.h>
#include <sddf/timer/config.h>

#include <cnc_v2/config.h>
#include <cnc_v2/control_ipc.h>

__attribute__((__section__(".timer_client_config"))) timer_client_config_t timer_config;
__attribute__((__section__(".state_core_config"))) cnc_v2_state_core_config_t config;

static cnc_v2_state_snapshot_t g_state_snapshot;
static uint32_t g_last_job_generation;
static uint32_t g_last_transport_generation;

static volatile cnc_v2_job_submit_t *job_submit_page(void)
{
    return (volatile cnc_v2_job_submit_t *)(uintptr_t)config.job_submit_vaddr;
}

static volatile cnc_v2_state_snapshot_t *state_snapshot_page(void)
{
    return (volatile cnc_v2_state_snapshot_t *)(uintptr_t)config.state_snapshot_vaddr;
}

static volatile cnc_v2_planner_request_t *planner_request_page(void)
{
    return (volatile cnc_v2_planner_request_t *)(uintptr_t)config.planner_request_vaddr;
}

static volatile cnc_v2_store_request_t *store_request_page(void)
{
    return (volatile cnc_v2_store_request_t *)(uintptr_t)config.store_request_vaddr;
}

static volatile cnc_v2_transport_status_t *transport_status_page(void)
{
    return (volatile cnc_v2_transport_status_t *)(uintptr_t)config.transport_status_vaddr;
}

static void publish_state_snapshot(void)
{
    cnc_v2_publish_state_snapshot(state_snapshot_page(), &g_state_snapshot);
    microkit_notify(config.observability_ch);
}

static void process_job_submit(void)
{
    cnc_v2_job_submit_t job;
    cnc_v2_planner_request_t planner_request = {0};
    cnc_v2_store_request_t store_request = {0};

    if (!cnc_v2_read_job_submit(job_submit_page(), &job)) {
        return;
    }
    if (job.generation == 0u || job.generation == g_last_job_generation || job.job_id == 0u) {
        return;
    }

    g_last_job_generation = job.generation;
    g_state_snapshot.state_generation += 1u;
    g_state_snapshot.current_job_id = job.job_id;
    g_state_snapshot.planner_request_generation += 1u;
    g_state_snapshot.lifecycle = CNC_V2_LIFECYCLE_PLANNED;
    g_state_snapshot.machine_state = CNC_STATE_IDLE;
    g_state_snapshot.last_error = 0u;

    planner_request.request_generation = g_state_snapshot.planner_request_generation;
    planner_request.job_id = job.job_id;
    planner_request.target_x = job.target_x;
    planner_request.target_y = job.target_y;
    planner_request.target_z = job.target_z;
    planner_request.feedrate = job.feedrate;
    planner_request.kind = job.kind;

    store_request.state_generation = g_state_snapshot.state_generation;
    store_request.job_id = job.job_id;
    store_request.lifecycle = g_state_snapshot.lifecycle;
    store_request.machine_state = g_state_snapshot.machine_state;

    cnc_v2_publish_planner_request(planner_request_page(), &planner_request);
    cnc_v2_publish_store_request(store_request_page(), &store_request);
    publish_state_snapshot();

    microkit_notify(config.planner_ch);
    microkit_notify(config.store_ch);
}

static void process_transport_status(void)
{
    cnc_v2_transport_status_t transport_status;

    if (!cnc_v2_read_transport_status(transport_status_page(), &transport_status)) {
        return;
    }
    if (transport_status.generation == 0u || transport_status.generation == g_last_transport_generation) {
        return;
    }

    g_last_transport_generation = transport_status.generation;
    g_state_snapshot.state_generation += 1u;
    g_state_snapshot.last_cmd_seq = transport_status.last_cmd_seq;
    g_state_snapshot.machine_state = transport_status.machine_state;
    g_state_snapshot.last_error = transport_status.last_error;

    switch (transport_status.phase) {
    case CNC_V2_TRANSPORT_WAITING_RESPONSE:
    case CNC_V2_TRANSPORT_TX_SUBMITTED:
        g_state_snapshot.lifecycle = CNC_V2_LIFECYCLE_TRANSPORT_PENDING;
        break;
    case CNC_V2_TRANSPORT_RESPONSE_OK:
        g_state_snapshot.lifecycle = CNC_V2_LIFECYCLE_MACHINE_UPDATED;
        break;
    case CNC_V2_TRANSPORT_TIMEOUT:
        g_state_snapshot.lifecycle = CNC_V2_LIFECYCLE_DIVERGED;
        break;
    case CNC_V2_TRANSPORT_BLOCKED:
        g_state_snapshot.lifecycle = CNC_V2_LIFECYCLE_SAFETY_REVIEWED;
        break;
    default:
        break;
    }

    publish_state_snapshot();
}

void init(void)
{
    if (!cnc_v2_config_magic_valid(config.magic) || !timer_config_check_magic(&timer_config)) {
        microkit_dbg_puts("state_core: bad config magic\n");
        return;
    }

    g_state_snapshot.generation = CNC_V2_GENERATION_INIT;
    g_state_snapshot.lifecycle = CNC_V2_LIFECYCLE_IDLE;
    g_state_snapshot.machine_state = CNC_STATE_IDLE;
    publish_state_snapshot();

    process_job_submit();
    sddf_timer_set_timeout(timer_config.driver_id, CNC_V2_STATE_TICK_NS);
}

void notified(microkit_channel ch)
{
    if (ch == CNC_V2_CH_JOB_TO_STATE) {
        process_job_submit();
        return;
    }

    if (ch == CNC_V2_CH_XPORT_TO_STATE) {
        process_transport_status();
        return;
    }

    if (ch == timer_config.driver_id) {
        g_state_snapshot.watchdog_ticks += 1u;
        publish_state_snapshot();
        process_job_submit();
        sddf_timer_set_timeout(timer_config.driver_id, CNC_V2_STATE_TICK_NS);
    }
}
