// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4 autoforward skips internal-queue events — C11 AOT path.
//
// Appendix D's `mainEventLoop` forwards only what it dequeues from the
// external queue; the internal drain above it has no forwarding step at
// all. 6.2 raises `error.execution` onto the internal queue when `<send>`
// names an unsupported type, so it must never reach an `autoforward`
// child — and it must be excluded by where it was raised, not by a filter
// that recognises its name.
//
// Sibling of `test_autoforward_done_invoke.c`, which pins the positive
// half. Together they leave no room for a name-based filter: one fails if
// `done.invoke` is withheld, the other if `error.execution` leaks.
//
// Fixture: integration_resources/autoforward_internal_queue/autoforward_internal_queue.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(autoforward_internal_queue ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdint.h>
#include <stdio.h>

#include "autoforward_internal_queue_sm.h"

int main(void) {
    autoforward_internal_queue_t sm;
    autoforward_internal_queue_init(&sm);

    // The child opens the exchange (`ready` from its own onentry), so it is
    // provably alive for everything that follows and the verdict does not
    // depend on when this backend starts its pending invokes relative to
    // the external drain.
    autoforward_internal_queue_run(&sm);

    int rc = autoforward_internal_queue_in_state(&sm, AUTOFORWARD_INTERNAL_QUEUE_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr,
                "autoforward_internal_queue: FAIL — the watcher saw "
                "`error.execution`, so an internal-queue event was "
                "autoforwarded. W3C Appendix D `mainEventLoop` forwards only "
                "what it dequeues from the external queue, and 6.2 raises "
                "`error.execution` onto the internal one: check that the "
                "event was not routed onto the external queue for some "
                "unrelated reason, which would leak it past any name-blind "
                "forward. Diagnostic: in_PASS=%d in_FAIL=%d in_phase=%d\n",
                autoforward_internal_queue_in_state(&sm, AUTOFORWARD_INTERNAL_QUEUE_STATE_PASS),
                autoforward_internal_queue_in_state(&sm, AUTOFORWARD_INTERNAL_QUEUE_STATE_FAIL),
                autoforward_internal_queue_in_state(&sm, AUTOFORWARD_INTERNAL_QUEUE_STATE_PHASE));
    }
    autoforward_internal_queue_destroy(&sm);
    return rc;
}
