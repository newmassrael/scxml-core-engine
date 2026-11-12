// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test577 — C11 AOT runner.
//
// W3C SCXML C.2 (test577): `<send event="test"
// type="http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"/>` with no
// `target`/`targetexpr` raises `error.communication` on the internal queue.
// cpp `actions/send.jinja2:337-343` emits a runtime
// `SendHelper::requiresTargetAttribute` check (which always returns true for
// the BasicHTTP URI per `sce/include/common/SendHelper.h:275-277`); C11
// collapses the check to a codegen-time decision per T3 inline lock-in — the
// literal `send_type` and the absence of `target`/`targetexpr` are both
// decidable at codegen time, so the new BasicHTTP-no-target arm in
// `state_machine.c.jinja2` emits `error.communication` raise + return
// directly (mirrors the test159 invalid-target arm). The W3C 5.10 mandate to
// halt subsequent executable content is satisfied by the `return` from the
// enclosing `execute_entry_actions`. test577's preceding `<send
// event="event1"/>` (W3C SCXMLEventProcessor default, line 11) lands in the
// external queue; the BasicHTTP failed-target send raises
// `error.communication` to the internal queue; internal-priority dequeue
// dispatches `error.communication` first → s0→pass. The fixture has no
// datamodel, no `cond`, no `expr`, and a send with no target posts nothing —
// so its machine requires neither Lua nor the HTTP client (parity with the
// datamodel-less `test355`), and its registry harness is `simple`.

#include <stdio.h>

#include "../require_started.h"
#include "test577_sm.h"

int main(void) {
    test577_t sm;
    REQUIRE_STARTED(test577_init(&sm), TEST577_START_OK);
    test577_run(&sm);

    int rc = test577_ended_in(&sm, TEST577_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test577: FAIL — active = 0x%08x\n", (unsigned)test577_active_states(&sm));
    }
    test577_destroy(&sm);
    return rc;
}
