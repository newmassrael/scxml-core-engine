// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test560 — C11 AOT runner.
//
// W3C SCXML 5.10 + 6.2: `<send event="foo"><param name="aParam" expr="1"/></send>`
// with a single literal-1 param, no datamodel block at all. The receiving
// transition's cond `_event.data.aParam == 1` reads the param off the
// promoted lua table on `_event.data` — fixture exists to verify the C11
// send-param arm works without a top-level `<datamodel>`, only the
// `_event.data.aParam` reader path forces `needs_script_engine`.
//
// W3C SCXML 5.10 + 6.2: <send event="foo"><param name="aParam"
// expr="1"/></send> without a top-level <datamodel> — the cond
// `_event.data.aParam == 1` is what forces needs_script_engine, the
// param-expr static literal 1 still crosses as JSON on the event's `data[]`
// so _event.data.aParam is the number 1 at the receiver

#include <stdio.h>

#include "../require_started.h"
#include "test560_sm.h"

int main(void) {
    test560_t sm;
    REQUIRE_STARTED(test560_init(&sm), TEST560_START_OK);
    test560_run(&sm);

    int rc = test560_ended_in(&sm, TEST560_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test560: FAIL — active = 0x%08x\n", (unsigned)test560_active_states(&sm));
    }
    test560_destroy(&sm);
    return rc;
}
