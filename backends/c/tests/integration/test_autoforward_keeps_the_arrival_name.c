// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4 + 5.10 + 3.12.1 — an autoforwarded copy keeps the name its
// event arrived under, on the C11 AOT local-invoke path.
//
// The host delivers two names the parent's document does not write: one a
// descriptor of it extends (`request.new`, delivered as the member `request`)
// and one only its `event="*"` takes (`other.thing`, delivered as the wildcard
// member). The copy the parent forwards to its child has to be told the whole
// name, which `_forward_to_autoforward_children` reads from the event's own
// `name` and not from the member's.
//
// Fixture: integration_resources/autoforward_keeps_the_arrival_name/
// autoforward_keeps_the_arrival_name.scxml (C-only).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(autoforward_keeps_the_arrival_name ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>
#include <string.h>

#include "autoforward_keeps_the_arrival_name_sm.h"

// Deliver `name` to the machine as a host does, by name, and run it to
// quiescence.
static void deliver(autoforward_keeps_the_arrival_name_t *sm, const char *name) {
    sce_forwarded_event_t arriving;
    memset(&arriving, 0, sizeof(arriving));
    sce_copy_bounded_id(arriving.name, name);
    autoforward_keeps_the_arrival_name_raise_external_forwarded(sm, &arriving);
    autoforward_keeps_the_arrival_name_run(sm);
}

int main(void) {
    autoforward_keeps_the_arrival_name_t sm;
    autoforward_keeps_the_arrival_name_init(&sm);
    autoforward_keeps_the_arrival_name_run(&sm);

    deliver(&sm, "request.new");
    const int heard_request =
        autoforward_keeps_the_arrival_name_in_state(&sm, AUTOFORWARD_KEEPS_THE_ARRIVAL_NAME_STATE_WAITOTHER);
    deliver(&sm, "other.thing");
    const int heard_other =
        autoforward_keeps_the_arrival_name_ended_in(&sm, AUTOFORWARD_KEEPS_THE_ARRIVAL_NAME_STATE_PASS);

    if (!heard_request || !heard_other) {
        (void)fprintf(stderr,
                      "autoforward_keeps_the_arrival_name: FAIL - the child was told %s: an autoforwarded "
                      "copy must carry the name the event arrived under (`request.new`, `other.thing`), "
                      "not the name of the member the parent delivered it as\n",
                      heard_request ? "the first name but not the second" : "neither name");
    }
    autoforward_keeps_the_arrival_name_destroy(&sm);
    return heard_request && heard_other ? 0 : 1;
}
