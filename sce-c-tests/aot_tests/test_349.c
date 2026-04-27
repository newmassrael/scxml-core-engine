// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test349 — C11 AOT runner.
//
// W3C SCXML 6.2 + 5.10.1: an explicit `type="http://www.w3.org/TR/scxml/
// #SCXMLEventProcessor"` literal collapses to the default external
// dispatch; the receiving transition assigns `_event.origin` into Var1.
// With the SCXMLEventProcessor type carve-out (옵션 ρ) the literal
// reduces to bare-external send, and with no cond gating the assign
// the s0→s2→pass path runs regardless of whether `_event.origin` is
// bound to a session URI or stays nil — the round-trip exercises the
// type literal on two distinct sends without relying on origin
// resolution. Once the origin/origintype metadata fields land (옵션 τ)
// the assigned value will hold the deterministic `<name>_session`
// literal but the boolean outcome is unchanged.

#include <stdio.h>

#include "test349_sm.h"

int main(void) {
    test349_t sm;
    test349_init(&sm);
    test349_run(&sm);

    int rc = test349_in_state(&sm, TEST349_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test349: FAIL — active = 0x%08x\n", (unsigned)test349_active_states(&sm));
    }
    test349_destroy(&sm);
    return rc;
}
