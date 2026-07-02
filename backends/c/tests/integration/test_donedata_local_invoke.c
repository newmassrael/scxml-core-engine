// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.5 + 6.3.1 donedata surfacing — C11 AOT local-invoke path.
//
// Closes the W3C IRP coverage gap on the C11 backend: a repository
// grep over `resources/*/test*.txml` for fixtures combining
// `<donedata>` + `<invoke>` + `done.invoke.<id>._event.data` readback
// returned zero hits, so the c11 codegen's lifted `<donedata>` literal
// shape (commit `6eec3a95` — lua stash + JSON-quoted `_event.data`
// carry) is exercised by W3C IRP `<donedata>` tests
// (294/527/528/529/176/179/186/578/298) but not by the local-invoke
// completion contract. This fixture fills exactly that gap.
//
// Fixture: integration_resources/donedata_local_invoke/donedata_local_invoke.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(donedata_local_invoke ...)`
// in `backends/c/tests/CMakeLists.txt`. The build itself is the §6.2.6
// freshness invariant — there is no committed tree for the c11
// backend.

#include <stdint.h>
#include <stdio.h>

#include "donedata_local_invoke_sm.h"

int main(void) {
    donedata_local_invoke_t sm;
    donedata_local_invoke_init(&sm);

    // No `<send delay>` in this fixture — the macrostep loop in `_run`
    // drains the parent's two invokes synchronously (child SM reaches
    // its inline `<final>` during `_init`, parent's
    // `execute_pending_invokes` raises done.invoke onto the external
    // queue, the next drain dispatches it, repeat for inv_content).
    // No scheduler / polling needed.
    donedata_local_invoke_run(&sm);

    int rc = donedata_local_invoke_in_state(&sm, DONEDATA_LOCAL_INVOKE_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr,
                "donedata_local_invoke: FAIL — current state is not "
                "DONEDATA_LOCAL_INVOKE_STATE_PASS (donedata envelope "
                "round-trip regressed on the C11 AOT engine). "
                "Diagnostic: in_PASS=%d in_FAIL=%d in_phase_param=%d "
                "in_phase_content=%d\n",
                donedata_local_invoke_in_state(&sm, DONEDATA_LOCAL_INVOKE_STATE_PASS),
                donedata_local_invoke_in_state(&sm, DONEDATA_LOCAL_INVOKE_STATE_FAIL),
                donedata_local_invoke_in_state(&sm, DONEDATA_LOCAL_INVOKE_STATE_PHASE_PARAM),
                donedata_local_invoke_in_state(&sm, DONEDATA_LOCAL_INVOKE_STATE_PHASE_CONTENT));
    }
    donedata_local_invoke_destroy(&sm);
    return rc;
}
