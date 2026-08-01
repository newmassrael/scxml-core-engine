// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.5 + 6.3.1: `<donedata>` survives a late completion — C11 AOT path.
//
// The sibling `test_donedata_local_invoke.c` pins the payload shapes on a
// child whose initial configuration is already its top-level `<final>`.
// That child is done before its first macrostep, so the lift and the raise
// sit in the same call and the fixture cannot see a child that finishes
// later.
//
// §6.3.1 raises `done.invoke.<id>` whenever the child reaches a final state,
// and §5.5 puts that final state's `<donedata>` on the event. A backend that
// lifts the stash only where the child finalises during start-up satisfies
// the sibling and still hands the parent an empty `_event.data` for every
// child that answers an event first.
//
// Here the child opens the exchange with `ready`, the parent answers over
// `<send target="#_inv_late">`, and the child reaches `settled` two
// macrosteps in. The payload and the guard are copied from the sibling's
// `inv_param` phase, so a shape the sibling already proves green cannot be
// what fails here — only the timing differs.
//
// Fixture: integration_resources/donedata_late_completion/donedata_late_completion.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(donedata_late_completion ...)`
// in `backends/c/tests/CMakeLists.txt`. The build itself is the §6.2.6
// freshness invariant — there is no committed tree for the c11 backend.

#include <stdint.h>
#include <stdio.h>

#include "donedata_late_completion_sm.h"

int main(void) {
    donedata_late_completion_t sm;
    donedata_late_completion_init(&sm);

    // The child opens the verdict exchange itself (`ready` from its own
    // onentry) and the parent drives it to `<final>` with a directed send,
    // so the run is decided by the queues alone — no `<send delay>`, no
    // scheduler, no polling.
    donedata_late_completion_run(&sm);

    int rc = donedata_late_completion_in_state(&sm, DONEDATA_LATE_COMPLETION_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr,
                "donedata_late_completion: FAIL — the parent's "
                "done.invoke.inv_late guard did not see `_event.data.result "
                "=== 42`, so the child's `<donedata>` was dropped on a "
                "completion that happened after the invoke was started. W3C "
                "SCXML 6.3.1 raises done.invoke.<id> wherever the child "
                "reaches its final state and 5.5 puts that state's donedata "
                "on the event; neither is scoped to children that finalise "
                "during start-up. "
                "Diagnostic: in_PASS=%d in_FAIL=%d in_phase=%d\n",
                donedata_late_completion_in_state(&sm, DONEDATA_LATE_COMPLETION_STATE_PASS),
                donedata_late_completion_in_state(&sm, DONEDATA_LATE_COMPLETION_STATE_FAIL),
                donedata_late_completion_in_state(&sm, DONEDATA_LATE_COMPLETION_STATE_PHASE));
    }
    donedata_late_completion_destroy(&sm);
    return rc;
}
