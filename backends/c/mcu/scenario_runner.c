/* SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The runner of a scenario table (sce_mcu_scenario.h): the program of every
 * image the `c11-mcu` gate builds from a machine. It starts the machine, makes
 * each step's act and asks its judge, and stops at the first step that does not
 * hold, writing which and why through the log seam (sce/log.h), the one way out
 * the image has. The exit status is the verdict: 0 when every step held, 1 at a
 * mismatch, 2 when the machine did not start or a step's act was refused.
 */

#include <sce/log.h>

#include "sce_mcu_scenario.h"

static void sce_mcu_say(sce_log_level_t level, sce_mcu_text_t label, sce_mcu_text_t text) {
    _sce_log_write(level, label.text, label.len, text.text, text.len);
}

int sce_mcu_replay(const sce_mcu_scenario_t *scenario) {
    static const sce_mcu_text_t not_started = SCE_MCU_TEXT("the machine did not start");
    static const sce_mcu_text_t refused = SCE_MCU_TEXT("the step's act was refused");
    if (!scenario->start(scenario->machine)) {
        sce_mcu_say(SCE_LOG_ERROR, scenario->name, not_started);
        return 2;
    }
    for (size_t at = 0u; at < scenario->steps_len; ++at) {
        const sce_mcu_step_t *step = &scenario->steps[at];
        if (step->act != NULL && !step->act(scenario->machine)) {
            sce_mcu_say(SCE_LOG_ERROR, step->label, refused);
            return 2;
        }
        const sce_mcu_text_t *failed = step->judge(scenario->machine);
        if (failed != NULL) {
            sce_mcu_say(SCE_LOG_ERROR, scenario->name, *failed);
            return 1;
        }
    }
    sce_mcu_say(SCE_LOG_INFO, scenario->name, scenario->summary);
    if (scenario->unjudged.len > 0u) {
        sce_mcu_say(SCE_LOG_INFO, scenario->name, scenario->unjudged);
    }
    return 0;
}

int sce_mcu_main(void) {
    return sce_mcu_replay(&sce_mcu_scenario);
}
