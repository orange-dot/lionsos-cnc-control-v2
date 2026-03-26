/*
 * Synthetic headless job ingress for LionsOS CNC Control V2.
 *
 * SPDX-License-Identifier: MIT
 */

#include <microkit.h>

#include <cnc_v2/config.h>
#include <cnc_v2/control_ipc.h>

__attribute__((__section__(".job_ingress_config"))) cnc_v2_job_ingress_config_t config;

static volatile cnc_v2_job_submit_t *job_submit_page(void)
{
    return (volatile cnc_v2_job_submit_t *)(uintptr_t)config.job_submit_vaddr;
}

void init(void)
{
    cnc_v2_job_submit_t job = {0};

    if (!cnc_v2_config_magic_valid(config.magic)) {
        microkit_dbg_puts("job_ingress: bad config magic\n");
        return;
    }

    job.job_id = 1u;
    job.target_x = 1200;
    job.target_y = 600;
    job.target_z = 0;
    job.feedrate = 500u;
    job.kind = CNC_V2_JOB_KIND_MOVE;
    cnc_v2_copy_cstring(job.name, CNC_V2_JOB_NAME_MAX, "boot-demo-job");

    job.generation = CNC_V2_GENERATION_INIT;
    cnc_v2_publish_job_submit(job_submit_page(), &job);
    microkit_notify(config.state_core_ch);
}

void notified(microkit_channel ch)
{
    (void)ch;
}
