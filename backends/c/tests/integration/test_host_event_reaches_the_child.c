// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: autoforward is owed to the external event, not to the door it
// came through — C11 AOT path.
//
// The four sibling `autoforward_*` fixtures all let the machine forward events
// it queued for itself, so every one of them drives the engine through its
// external drain. This one hands the machine an event from outside and asks
// whether the `autoforward` child sees it. Appendix D's `mainEventLoop` binds
// the preliminary step (`applyFinalize` plus the autoforward `send`) to the
// external event it is about to select transitions for, so an engine with more
// than one way in has to run the step at each of them.
//
// Measured 2026-08-21: the C++ AOT engine had the step written inline in its
// queue drain, so its `processEvent()` skipped it. This backend has one door —
// `_raise_external` then `_step` — so the drain is the only place the step can
// live, and this fixture pins that. A later entry point that hands the event
// straight to the transition selector would go red here.
//
// Fixture: integration_resources/host_event_reaches_the_child/host_event_reaches_the_child.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(host_event_reaches_the_child ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "host_event_reaches_the_child_sm.h"

// Bounded rather than timed: every step here is the machine's own work, so a
// machine that has not arrived after this many is not slow, it is not going to.
#define MAX_STEPS 50

int main(void) {
    host_event_reaches_the_child_t sm;
    host_event_reaches_the_child_init(&sm);

    // The child opens the exchange (`ready` from its own onentry), so it is
    // provably alive for everything that follows and the verdict does not
    // depend on when this backend starts its pending invokes relative to the
    // external drain.
    for (int i = 0;
         i < MAX_STEPS && !host_event_reaches_the_child_in_state(&sm, HOST_EVENT_REACHES_THE_CHILD_STATE_ARMED); ++i) {
        host_event_reaches_the_child_step(&sm);
    }
    if (!host_event_reaches_the_child_in_state(&sm, HOST_EVENT_REACHES_THE_CHILD_STATE_ARMED)) {
        fprintf(stderr, "host_event_reaches_the_child: FAIL — the probe child never "
                        "sent `ready`, so the fixture never reached the state where a "
                        "host event can be handed over. That is a broken handshake, "
                        "not a forwarding verdict.\n");
        host_event_reaches_the_child_destroy(&sm);
        return 1;
    }

    // The axis: the host hands the machine an external event.
    host_event_reaches_the_child_event_with_meta_t evt;
    memset(&evt, 0, sizeof(evt));
    evt.event = HOST_EVENT_REACHES_THE_CHILD_EVENT_HOSTPING;
    host_event_reaches_the_child_raise_external(&sm, &evt);

    for (int i = 0; i < MAX_STEPS && !host_event_reaches_the_child_is_in_final_state(&sm); ++i) {
        host_event_reaches_the_child_step(&sm);
    }

    int rc = host_event_reaches_the_child_in_state(&sm, HOST_EVENT_REACHES_THE_CHILD_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr,
                "host_event_reaches_the_child: FAIL — the probe child answered "
                "`sawMarkerOnly`, so the event the host handed over was never "
                "forwarded to it and the child only ever saw the `marker` the "
                "parent's own transition body sent. W3C Appendix D "
                "`mainEventLoop` runs the autoforward `send` against the "
                "external event before it selects transitions for it, whichever "
                "door the event arrived through: an engine that runs that step "
                "only in its queue drain leaves an autoforward child blind to "
                "everything its host delivers. "
                "Diagnostic: in_PASS=%d in_FAIL=%d in_armed=%d\n",
                host_event_reaches_the_child_in_state(&sm, HOST_EVENT_REACHES_THE_CHILD_STATE_PASS),
                host_event_reaches_the_child_in_state(&sm, HOST_EVENT_REACHES_THE_CHILD_STATE_FAIL),
                host_event_reaches_the_child_in_state(&sm, HOST_EVENT_REACHES_THE_CHILD_STATE_ARMED));
    }
    host_event_reaches_the_child_destroy(&sm);
    return rc;
}
