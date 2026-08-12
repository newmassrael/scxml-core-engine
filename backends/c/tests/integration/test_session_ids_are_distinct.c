// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10: `_sessionid` is the id of a session — C11 AOT.
//
// The clause binds `_sessionid` to "the system-generated id for the current
// SCXML session", and Appendix C.1.1 derives the address a session publishes
// from that id. Two live sessions holding one id therefore publish one
// address, and a `<send>` addressed to either reaches both.
//
// No test in the public IRP corpus can ask: every one of them that reaches
// `_sessionid` runs a single session, so a processor that hands the same
// value to every session it starts passes them all. The C11 backend did
// exactly that until this fixture was added.
//
// The fixture runs two children at once, each reporting the id it was
// issued, and the parent compares them. Reused id lands in `fail`; a second
// report that never arrives leaves the parent parked and this driver times
// out, which is the honest signal for a child that was never started.
//
// Fixture: integration_resources/session_ids_are_distinct/session_ids_are_distinct.scxml
// (canonical, shared with every other channel).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(session_ids_are_distinct ...)` in
// `backends/c/tests/CMakeLists.txt`. The build is the freshness invariant —
// there is no committed tree for the c11 backend.

#include <stdio.h>

#include "session_ids_are_distinct_sm.h"

int main(void) {
    session_ids_are_distinct_t sm;
    session_ids_are_distinct_init(&sm);

    // No `<send delay>` here: both children report during their own `_init`,
    // so the macrostep loop in `_run` decides the verdict.
    session_ids_are_distinct_run(&sm);

    const int pass = session_ids_are_distinct_in_state(&sm, SESSION_IDS_ARE_DISTINCT_STATE_PASS);
    const int fail = session_ids_are_distinct_in_state(&sm, SESSION_IDS_ARE_DISTINCT_STATE_FAIL);
    const int parked = session_ids_are_distinct_in_state(&sm, SESSION_IDS_ARE_DISTINCT_STATE_ONE_SEEN);

    if (!pass) {
        if (fail) {
            fprintf(stderr, "session_ids_are_distinct: FAIL - two live sessions reported the same `_sessionid`. W3C "
                            "SCXML 5.10 binds it to the id of the current session, and C.1.1 publishes an address "
                            "derived from it, so one id for two sessions is one address for two sessions.\n");
        } else if (parked) {
            fprintf(stderr, "session_ids_are_distinct: FAIL - only one child reported its `_sessionid`, so the ids "
                            "were never compared.\n");
        } else {
            fprintf(stderr,
                    "session_ids_are_distinct: FAIL - settled in no verdict state, so the ids "
                    "were never judged. Diagnostic: in_PASS=%d in_FAIL=%d in_ONE_SEEN=%d\n",
                    pass, fail, parked);
        }
    }

    session_ids_are_distinct_destroy(&sm);
    return pass ? 0 : 1;
}
