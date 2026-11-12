// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test199 — C11 AOT runner.
//
// W3C SCXML 6.2: <send type="..."> with an unsupported literal type must
// raise error.execution on the internal queue. test199's s0 onentry has
// two sends — the first is `<send type="unsupported_type" event="event1"/>`
// (literal type matches neither SCXMLEventProcessor nor BasicHTTP, so it
// must false-fall into error.execution per W3C 6.2) followed by
// `<send event="timeout"/>`. W3C 5.10 says when an onentry block raises
// an error the processor must stop executing the remaining actions in
// that block, so the second `<send event="timeout"/>` never runs. The
// result is that error.execution reaches the internal queue alone and
// matches s0's `event="error.execution"` transition to `pass`.
//
// W3C SCXML 6.2: <send type="unsupported_type"> raises error.execution — the
// literal type matches neither SCXMLEventProcessor nor BasicHTTP, so the
// codegen-time false-fall (cpp `SendHelper::isSupportedSendType` mirror)
// emits the error raise and returns from the surrounding onentry; the
// receiving `event="error.execution"` transition matches before the second
// `<send event="timeout"/>` (skipped per W3C 5.10) can fire.

#include <stdio.h>

#include "../require_started.h"
#include "test199_sm.h"

int main(void) {
    test199_t sm;
    REQUIRE_STARTED(test199_init(&sm), TEST199_START_OK);
    test199_run(&sm);

    int rc = test199_ended_in(&sm, TEST199_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test199: FAIL — active = 0x%08x\n", (unsigned)test199_active_states(&sm));
    }
    test199_destroy(&sm);
    return rc;
}
