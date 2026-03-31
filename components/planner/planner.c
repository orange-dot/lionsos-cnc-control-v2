/*
 * Minimal planner for LionsOS CNC Control V2.
 *
 * SPDX-License-Identifier: MIT
 */

#include <stdint.h>

#include <microkit.h>

#include <cnc_v2/config.h>
#include <cnc_v2/control_ipc.h>
#include <cnc_v2/wire_protocol.h>

__attribute__((__section__(".planner_config"))) cnc_v2_planner_config_t config;

static cnc_v2_planner_status_t g_status;
static uint32_t g_last_request_generation;
static uint8_t g_next_cmd_seq;

static volatile cnc_v2_planner_request_t *planner_request_page(void)
{
    return (volatile cnc_v2_planner_request_t *)(uintptr_t)config.planner_request_vaddr;
}

static volatile cnc_v2_motion_candidate_t *motion_candidate_page(void)
{
    return (volatile cnc_v2_motion_candidate_t *)(uintptr_t)config.motion_candidate_vaddr;
}

static volatile cnc_v2_planner_status_t *planner_status_page(void)
{
    return (volatile cnc_v2_planner_status_t *)(uintptr_t)config.planner_status_vaddr;
}

static void publish_status(void)
{
    cnc_v2_publish_planner_status(planner_status_page(), &g_status);
    microkit_notify(config.observability_ch);
}

void init(void)
{
    if (!cnc_v2_config_magic_valid(config.magic)) {
        microkit_dbg_puts("planner: bad config magic\n");
        return;
    }

    g_status.generation = CNC_V2_GENERATION_INIT;
    publish_status();
}

void notified(microkit_channel ch)
{
    cnc_v2_planner_request_t request;
    cnc_v2_motion_candidate_t candidate = {0};

    if (ch != CNC_V2_CH_STATE_TO_PLANNER) {
        return;
    }
    if (!cnc_v2_read_planner_request(planner_request_page(), &request)) {
        return;
    }
    if (request.generation == 0u || request.generation == g_last_request_generation) {
        return;
    }

    g_last_request_generation = request.generation;
    g_status.request_generation = request.request_generation;
    g_status.produced_frames += 1u;
    g_status.last_kind = request.kind;
    g_status.last_cmd_seq = g_next_cmd_seq;
    publish_status();

    candidate.plan_generation = g_status.produced_frames;
    candidate.request_generation = request.request_generation;
    candidate.plan_kind = request.kind;

    if (request.kind == CNC_V2_JOB_KIND_HOME) {
        cnc_frame_init_cmd(&candidate.frame, g_next_cmd_seq, CNC_CMD_HOME);
        candidate.frame.payload[0] = 0x07u;
        cnc_frame_seal(&candidate.frame);
    } else {
        cnc_pack_move(&candidate.frame,
                      g_next_cmd_seq,
                      request.target_x,
                      request.target_y,
                      request.target_z,
                      request.feedrate);
    }

    g_next_cmd_seq += 1u;
    cnc_v2_publish_motion_candidate(motion_candidate_page(), &candidate);
    microkit_notify(config.safety_ch);
    microkit_notify(config.observability_ch);
}
