// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4 autoforward field preservation — C11 AOT local-invoke path.
//
// W3C §6.4 requires the parent to forward an exact copy of every external
// event to an `<invoke autoforward="true">` child. The public IRP suite
// never checks the copy's contents: test229 only asserts the event name
// crosses, and test230 is a manual test whose field comparison is done by a
// human reading two log dumps. A forward stripped down to the bare event
// name passes both.
//
// Fixture: integration_resources/autoforward_event_fields/autoforward_event_fields.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(autoforward_event_fields ...)`
// in `backends/c/tests/CMakeLists.txt`. The build itself is the §6.2.6
// freshness invariant — there is no committed tree for the c11 backend.

#include <stdint.h>
#include <stdio.h>

#include "autoforward_event_fields_sm.h"

int main(void) {
    autoforward_event_fields_t sm;
    autoforward_event_fields_init(&sm);

    // No `<send delay>` in this fixture: the child raises `childToParent`
    // during its own `_init`, the parent's macrostep loop autoforwards it
    // back, and the child's verdict rides home on `done.invoke.inv_echo`.
    autoforward_event_fields_run(&sm);

    int rc = autoforward_event_fields_in_state(&sm, AUTOFORWARD_EVENT_FIELDS_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr,
                "autoforward_event_fields: FAIL — the child reported "
                "`stripped`, so the autoforwarded copy of `childToParent` "
                "lost `_event.data.value`, `_event.origin` or "
                "`_event.invokeid`. W3C 6.4 requires an exact copy: "
                "`_forward_to_autoforward_children` must pass the source "
                "event's fields through `sce_forwarded_event_t`, not just "
                "the event name. Diagnostic: in_PASS=%d in_FAIL=%d "
                "in_phase=%d\n",
                autoforward_event_fields_in_state(&sm, AUTOFORWARD_EVENT_FIELDS_STATE_PASS),
                autoforward_event_fields_in_state(&sm, AUTOFORWARD_EVENT_FIELDS_STATE_FAIL),
                autoforward_event_fields_in_state(&sm, AUTOFORWARD_EVENT_FIELDS_STATE_PHASE));
    }
    autoforward_event_fields_destroy(&sm);
    return rc;
}
