/*
 * In-memory session store stub for LionsOS CNC Control V2.
 *
 * SPDX-License-Identifier: MIT
 */

#include <stdint.h>

#include <microkit.h>

#include <cnc_v2/config.h>
#include <cnc_v2/control_ipc.h>

__attribute__((__section__(".session_store_config"))) cnc_v2_session_store_config_t config;

static cnc_v2_store_status_t g_status;
static uint32_t g_last_request_generation;

static volatile cnc_v2_store_request_t *store_request_page(void)
{
    return (volatile cnc_v2_store_request_t *)(uintptr_t)config.store_request_vaddr;
}

static volatile cnc_v2_store_status_t *store_status_page(void)
{
    return (volatile cnc_v2_store_status_t *)(uintptr_t)config.store_status_vaddr;
}

static void publish_status(void)
{
    cnc_v2_publish_store_status(store_status_page(), &g_status);
    microkit_notify(config.observability_ch);
}

void init(void)
{
    if (!cnc_v2_config_magic_valid(config.magic)) {
        microkit_dbg_puts("session_store: bad config magic\n");
        return;
    }

    g_status.generation = CNC_V2_GENERATION_INIT;
    publish_status();
}

void notified(microkit_channel ch)
{
    cnc_v2_store_request_t request;

    if (ch != CNC_V2_CH_STATE_TO_STORE) {
        return;
    }
    if (!cnc_v2_read_store_request(store_request_page(), &request)) {
        return;
    }
    if (request.generation == 0u || request.generation == g_last_request_generation) {
        return;
    }

    g_last_request_generation = request.generation;
    g_status.persisted_generation = request.state_generation;
    g_status.persisted_job_id = request.job_id;
    g_status.writes_completed += 1u;
    g_status.result = CNC_V2_STORE_OK;
    publish_status();
}
