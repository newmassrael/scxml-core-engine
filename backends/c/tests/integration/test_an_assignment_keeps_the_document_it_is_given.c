// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.4 and B.2: `<assign location="v" expr="_event.data"/>` leaves
// `v` holding the DOM a `<send>`'s XML `<content>` arrived as, and so does a
// member-path location — C11 AOT.
//
// The template runs `<location> = (<expr>)` in the Lua state for every
// location, so the value never leaves the engine; the C++ engines once read a
// bare-name location's value out and wrote it back, which lost the document.
//
// Fixture:
// integration_resources/an_assignment_keeps_the_document_it_is_given/an_assignment_keeps_the_document_it_is_given.scxml
// (canonical, shared with every channel).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(an_assignment_keeps_the_document_it_is_given ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdint.h>
#include <stdio.h>

#include "../require_started.h"
#include "an_assignment_keeps_the_document_it_is_given_sm.h"

int main(void) {
    an_assignment_keeps_the_document_it_is_given_t sm;
    REQUIRE_STARTED(an_assignment_keeps_the_document_it_is_given_init(&sm),
                    AN_ASSIGNMENT_KEEPS_THE_DOCUMENT_IT_IS_GIVEN_START_OK);

    // No `<send delay>` here: the event the session sends itself is on the
    // external queue once `_init` returns, and `_run` consumes it.
    an_assignment_keeps_the_document_it_is_given_run(&sm);

    const int kept = an_assignment_keeps_the_document_it_is_given_ended_in(
        &sm, AN_ASSIGNMENT_KEEPS_THE_DOCUMENT_IT_IS_GIVEN_STATE_KEPT);
    const int name_lost = an_assignment_keeps_the_document_it_is_given_ended_in(
        &sm, AN_ASSIGNMENT_KEEPS_THE_DOCUMENT_IT_IS_GIVEN_STATE_NAMELOST);
    const int member_lost = an_assignment_keeps_the_document_it_is_given_ended_in(
        &sm, AN_ASSIGNMENT_KEEPS_THE_DOCUMENT_IT_IS_GIVEN_STATE_MEMBERLOST);

    if (!kept) {
        if (name_lost) {
            fprintf(stderr, "an_assignment_keeps_the_document_it_is_given: FAIL: "
                            "`<assign location=\"v\" expr=\"_event.data\"/>` did not leave the "
                            "document in `v`: the value was read out of the data model and "
                            "written back.\n");
        } else if (member_lost) {
            fprintf(stderr, "an_assignment_keeps_the_document_it_is_given: FAIL: "
                            "`<assign location=\"holder.doc\" expr=\"_event.data\"/>` did not "
                            "leave the document in `holder.doc`.\n");
        } else {
            fprintf(stderr, "an_assignment_keeps_the_document_it_is_given: FAIL: settled in no "
                            "verdict state: its own `doc` event never arrived.\n");
        }
    }

    an_assignment_keeps_the_document_it_is_given_destroy(&sm);
    return kept ? 0 : 1;
}
