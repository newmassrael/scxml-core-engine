// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test412 — C11 AOT runner.
//
// W3C SCXML 3.6/3.3.2: executable content inside `<initial><transition>`
// runs after the parent state's onentry and before the child's onentry,
// so the raise sequence across the s0/s01/s011 entry chain must produce
// event1 (s01.onentry) then event2 (s01.<initial>.transition body) then
// event3 (s011.onentry) in that order. The fixture cross-checks the
// ordering through a chain of single-event transitions: s011's
// eventless transition routes to s02; event1 advances to s03; event2
// advances to s04; event3 advances to pass. Any other interleaving
// trips one of the wildcard `<transition event="*" target="fail"/>`
// arms. The C11 chain loop emits the per-state initial_transition_actions
// switch arm immediately after the chain element's onentry runs (and
// before the next chain element's onentry), exactly matching cpp
// enterStates' default-initial branch ordering.
//
// Spec-mirror parity (cpp tests/CMakeLists.txt:817 registers the same
// fixture as `sce_generate_static_w3c_test(412 ... TYPE SCHEDULED)`;
// the C11 _run loop reaches pass via macrostep alone because the
// safety-net `<send event="timeout" delay="1s"/>` is only scheduled,
// not fired, by the in-process scheduler when the macrostep drains
// the queue first).

#include <stdio.h>

#include "test412_sm.h"

int main(void) {
    test412_t sm;
    test412_init(&sm);
    test412_run(&sm);

    int rc = test412_in_state(&sm, TEST412_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test412: FAIL — active = 0x%08x\n", (unsigned)test412_active_states(&sm));
    }
    test412_destroy(&sm);
    return rc;
}
