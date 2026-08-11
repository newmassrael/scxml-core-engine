// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix C.1 `_event.origin` is an address — C11 AOT.
//
// The clause has two halves. The origin of a delivered event must match the
// `location` field the sending session published for the SCXML Event I/O
// Processor in its `_ioprocessors`, and that location is what a peer sends
// back to. A machine that puts a bare session id — or an invoke-instance
// path — there satisfies neither: the value matches nothing the sender
// published, and it names no target.
//
// The public IRP suite cannot separate the two spellings. Test 336 and test
// 350 both check `_event.origin` by sending to it with the sender and the
// receiver being the same session, so any value at all round-trips. Nothing
// in the corpus sends across sessions, which is the only arrangement where a
// bare id and a location differ.
//
// The fixture puts a second session on the other end, so the two halves
// separate and each has its own signal:
//
//   mismatch  the machine settles in `fail` — `_event.origin` did not equal
//             the location the child published for itself
//   routing   the machine settles in `await_reply` — a target that resolves
//             nowhere delivers no event to transition on, so a routing
//             violation produces no failure event, only a parked machine
//
// Fixture: integration_resources/event_origin_is_a_location/event_origin_is_a_location.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(event_origin_is_a_location ...)`
// in `backends/c/tests/CMakeLists.txt`. The build itself is the §6.2.6
// freshness invariant — there is no committed tree for the c11 backend.

#include <stdint.h>
#include <stdio.h>

#include "event_origin_is_a_location_sm.h"

int main(void) {
    event_origin_is_a_location_t sm;
    event_origin_is_a_location_init(&sm);

    // No `<send delay>` in this fixture: the child sends `fromChild` during
    // its own `_init`, and the parent's reply is a directed send consumed in
    // the same drain, so the macrostep loop in `_run` decides the verdict.
    event_origin_is_a_location_run(&sm);

    const int pass = event_origin_is_a_location_in_state(&sm, EVENT_ORIGIN_IS_A_LOCATION_STATE_PASS);
    const int fail = event_origin_is_a_location_in_state(&sm, EVENT_ORIGIN_IS_A_LOCATION_STATE_FAIL);
    const int parked = event_origin_is_a_location_in_state(&sm, EVENT_ORIGIN_IS_A_LOCATION_STATE_AWAIT_REPLY);

    if (!pass) {
        if (fail) {
            fprintf(stderr, "event_origin_is_a_location: FAIL — `_event.origin` did not carry the "
                            "sender's published `_ioprocessors` location. Appendix C.1 requires the "
                            "origin to match that location, which is what makes it an address a peer "
                            "can answer; a bare session id or an invoke-instance path matches nothing "
                            "the sender published.\n");
        } else if (parked) {
            fprintf(stderr, "event_origin_is_a_location: FAIL — the parent accepted `_event.origin` as "
                            "an address and sent `reply` to it, and nothing came back. Appendix C.1 "
                            "requires the published location to be a usable <send> target, so an origin "
                            "that routes nowhere fails the half a self-addressed test cannot "
                            "exercise.\n");
        } else {
            fprintf(stderr,
                    "event_origin_is_a_location: FAIL — settled in no verdict state, so the "
                    "origin was never judged. Diagnostic: in_PASS=%d in_FAIL=%d "
                    "in_AWAIT_REPLY=%d in_WAITING=%d\n",
                    pass, fail, parked,
                    event_origin_is_a_location_in_state(&sm, EVENT_ORIGIN_IS_A_LOCATION_STATE_WAITING));
        }
    }

    event_origin_is_a_location_destroy(&sm);
    return pass ? 0 : 1;
}
