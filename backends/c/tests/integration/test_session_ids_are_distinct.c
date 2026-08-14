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
#include <string.h>

#include "session_ids_are_distinct_sm.h"

// W3C SCXML 5.3: the machine answers what its own datamodel holds.
//
// Asserted here rather than in a fixture of its own because this document
// already declares the shape that separates a real reader from a frozen one:
// `firstSid` is authored `''` and then assigned the first child's session id
// while the run is going. A reader that answered the document's literal would
// say "" at every point, and pass any check that only asked whether it could
// be called.
//
// The other five backends have had this since the accessor emission landed;
// C11 emitted no datamodel reader at all, so a host driving a generated
// machine on an MCU could not ask it anything about its own datamodel. This
// is that gap's assertion, on the backend that had it.
static int check_datamodel_reader(const session_ids_are_distinct_t *sm) {
    char buf[SCE_MAX_ID_LEN];
    size_t len = (size_t)-1;

    if (!session_ids_are_distinct_first_sid(sm, buf, sizeof(buf), &len)) {
        fprintf(stderr, "session_ids_are_distinct: FAIL - `firstSid` could not be read. A `<data>` "
                        "the document declares with a string initializer must be readable off the "
                        "machine, in the host's own type.\n");
        return 1;
    }
    // The document authors `firstSid` as `''` and assigns a child's session id
    // while the run is going, so anything non-empty here is a value only the
    // datamodel knew. This is the whole discriminator: a reader backed by a
    // copy taken at generation time answers "" for the entire run.
    if (buf[0] == '\0') {
        fprintf(stderr, "session_ids_are_distinct: FAIL - `firstSid` read as empty. The reader must "
                        "report what the datamodel HOLDS, and by this point a child's session id "
                        "has been assigned over the authored empty string.\n");
        return 1;
    }
    if (len != strlen(buf)) {
        fprintf(stderr,
                "session_ids_are_distinct: FAIL - the reader reported length %zu for a %zu-byte "
                "answer. `len` carries the FULL length so a caller can size its buffer.\n",
                len, strlen(buf));
        return 1;
    }
    // A one-byte buffer can hold nothing but the terminator, so it forces the
    // truncating branch. `len` must still name what would have fit, which is
    // the only thing that lets a caller recover.
    char tiny[1];
    size_t needed = 0u;
    if (!session_ids_are_distinct_first_sid(sm, tiny, sizeof(tiny), &needed)) {
        fprintf(stderr, "session_ids_are_distinct: FAIL - a buffer too small to hold the answer "
                        "made the reader report that it could not answer. Truncation is a fact "
                        "about the caller's buffer, not about the machine.\n");
        return 1;
    }
    if (tiny[0] != '\0' || needed != len) {
        fprintf(stderr,
                "session_ids_are_distinct: FAIL - the truncating branch wrote %zu byte(s) and "
                "reported a needed length of %zu, expected an empty NUL-terminated prefix and %zu. "
                "A caller sizing its next buffer from this would get it wrong.\n",
                strlen(tiny), needed, len);
        return 1;
    }
    return 0;
}

int main(void) {
    // Zero-initialised rather than left indeterminate: the check below reads
    // the machine BEFORE `_init` has run, and an indeterminate `L` would make
    // that read undefined rather than answerable. `_init` memsets the struct
    // regardless, so this costs the run nothing.
    //
    // There is no second reading taken between `_init` and `_run`: measured,
    // `_init` stabilises the initial macrostep, so a child has already
    // reported and `firstSid` is assigned by the time it returns. A check
    // expecting the authored `''` there was asserting something false about
    // this document.
    session_ids_are_distinct_t sm = {0};

    // Nothing has read the document yet, so there is no session holding a
    // datamodel and the only honest answer is "cannot answer". A reader
    // backed by a struct member would report the authored literal here.
    {
        char buf[SCE_MAX_ID_LEN];
        if (session_ids_are_distinct_first_sid(&sm, buf, sizeof(buf), NULL)) {
            fprintf(stderr, "session_ids_are_distinct: FAIL - an uninitialised machine answered a "
                            "datamodel read. Before `_init` there is no session holding one.\n");
            return 1;
        }
    }

    session_ids_are_distinct_init(&sm);

    // No `<send delay>` here: both children report during their own `_init`,
    // so the macrostep loop in `_run` decides the verdict.
    session_ids_are_distinct_run(&sm);

    if (check_datamodel_reader(&sm)) {
        session_ids_are_distinct_destroy(&sm);
        return 1;
    }

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
