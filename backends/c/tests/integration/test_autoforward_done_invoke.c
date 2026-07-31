// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4 autoforward carries `done.invoke.<id>` — C11 AOT path.
//
// Appendix D's `mainEventLoop` forwards every event it dequeues from the
// external queue to each `autoforward` child without testing the event's
// name; the sole exclusion is the cancel event, expressed as control flow.
// §6.4.2 places `done.invoke.<id>` on the external queue of the invoking
// session, so a sibling child that is still running must receive it.
//
// Fixture: integration_resources/autoforward_done_invoke/autoforward_done_invoke.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(autoforward_done_invoke ...)`
// in `backends/c/tests/CMakeLists.txt`. The build itself is the §6.2.6
// freshness invariant — there is no committed tree for the c11 backend.

#include <stdint.h>
#include <stdio.h>

#include "autoforward_done_invoke_sm.h"

int main(void) {
    autoforward_done_invoke_t sm;
    autoforward_done_invoke_init(&sm);

    // No `<send delay>` in this fixture: `inv_short` reaches `<final>` during
    // its own `_init`, raising `done.invoke.inv_short` onto the parent's
    // external queue while `inv_watch` is still running. The parent's
    // targetless transition then sends `probe` to itself, which orders the
    // two events the watcher can see without any wall-clock delay.
    autoforward_done_invoke_run(&sm);

    int rc = autoforward_done_invoke_in_state(&sm, AUTOFORWARD_DONE_INVOKE_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr,
                "autoforward_done_invoke: FAIL — the watcher saw only `probe`, "
                "so `done.invoke.inv_short` was withheld from a live "
                "`autoforward` child. W3C Appendix D `mainEventLoop` forwards "
                "every event dequeued from the external queue and excludes "
                "only the cancel event, and 6.4.2 places `done.invoke.<id>` on "
                "that queue: `_forward_to_autoforward_children` must emit a "
                "switch arm for every event, not skip the `done.`/`error.` "
                "names. Diagnostic: in_PASS=%d in_FAIL=%d in_phase=%d\n",
                autoforward_done_invoke_in_state(&sm, AUTOFORWARD_DONE_INVOKE_STATE_PASS),
                autoforward_done_invoke_in_state(&sm, AUTOFORWARD_DONE_INVOKE_STATE_FAIL),
                autoforward_done_invoke_in_state(&sm, AUTOFORWARD_DONE_INVOKE_STATE_PHASE));
    }
    autoforward_done_invoke_destroy(&sm);
    return rc;
}
