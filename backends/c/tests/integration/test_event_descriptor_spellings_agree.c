// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.12.1: `wild`, `wild.` and `wild.*` are one descriptor — C11 AOT path.
//
// The clause calls the three spellings "functionally equivalent since they are
// token prefixes of exactly the same set of event names", and a descriptor
// ending in ".*" matches "zero or more tokens", so a bare ".*" is an empty
// token prefix and matches every event.
//
// No W3C fixture delivers a bare `foo` to a `foo.*` handler, and the four that
// write a bare ".*" write it as a catch-all to fail — which an engine that
// matches nothing on ".*" passes by never taking the transition it must not
// take. This fixture puts every case in positive polarity instead.
//
// Fixture: integration_resources/event_descriptor_spellings_agree/event_descriptor_spellings_agree.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(event_descriptor_spellings_agree ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdint.h>
#include <stdio.h>

#include "event_descriptor_spellings_agree_sm.h"

// Each failure final names the case it belongs to, so the diagnostic below
// says which spelling disagreed with the clause rather than only that one did.
static const char *failure_case(const event_descriptor_spellings_agree_t *sm) {
    if (event_descriptor_spellings_agree_in_state(sm, EVENT_DESCRIPTOR_SPELLINGS_AGREE_STATE_FAILSUFFIXED)) {
        return "`wild.*` did not catch bare `wild`, though the clause calls the "
               "spellings functionally equivalent";
    }
    if (event_descriptor_spellings_agree_in_state(sm, EVENT_DESCRIPTOR_SPELLINGS_AGREE_STATE_FAILDOTTED)) {
        return "`dot.` did not catch bare `dot`, though the clause calls the "
               "spellings functionally equivalent";
    }
    if (event_descriptor_spellings_agree_in_state(sm, EVENT_DESCRIPTOR_SPELLINGS_AGREE_STATE_FAILBOUNDED)) {
        return "`wild.*` caught `wilder`; the prefix is a whole token, so it must not";
    }
    if (event_descriptor_spellings_agree_in_state(sm, EVENT_DESCRIPTOR_SPELLINGS_AGREE_STATE_FAILUNIVERSAL)) {
        return "a bare `.*` did not catch `any.token.sequence`; an empty token "
               "prefix is a prefix of every event name";
    }
    return "the machine reached no final state at all; every case here has a "
           "literal fallback, so this means no transition matched an event that "
           "two of them describe";
}

int main(void) {
    event_descriptor_spellings_agree_t sm;
    event_descriptor_spellings_agree_init(&sm);
    event_descriptor_spellings_agree_run(&sm);

    int rc = 0;
    if (!event_descriptor_spellings_agree_in_state(&sm, EVENT_DESCRIPTOR_SPELLINGS_AGREE_STATE_PASS)) {
        fprintf(stderr, "event_descriptor_spellings_agree: FAIL - %s\n", failure_case(&sm));
        rc = 1;
    }

    event_descriptor_spellings_agree_destroy(&sm);
    return rc;
}
