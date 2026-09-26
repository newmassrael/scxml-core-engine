// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A wildcard keeps its own guard and its own type - C11 AOT path.
//
// W3C SCXML 3.12.1 lets a `*` descriptor match every event, and that is all
// it changes: W3C SCXML 3.13 still asks the transition's `cond` whether it is
// enabled, and a `type="internal"` transition whose target is a proper
// descendant of its compound source still does not exit that source.
//
// This channel compiles a bare wildcard to `event != EVENT_NONE` rather than
// consulting a matcher, so the descriptor itself cannot fail here - what this
// driver measures is that the guard and the type survive that lowering.
//
// Fixture: integration_resources/wildcard_in_document_order/wildcard_in_document_order.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(wildcard_in_document_order ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdint.h>
#include <stdio.h>

#include "wildcard_in_document_order_sm.h"

// Each failure final names the case it belongs to, so the diagnostic below
// says which property of the wildcard was lost rather than only that one was.
static const char *failure_case(const wildcard_in_document_order_t *sm) {
    if (wildcard_in_document_order_ended_in(sm, WILDCARD_IN_DOCUMENT_ORDER_STATE_FAILGUARDIGNORED)) {
        return "a wildcard fired with its guard false, instead of leaving the event "
               "to its ancestor's transition";
    }
    if (wildcard_in_document_order_ended_in(sm, WILDCARD_IN_DOCUMENT_ORDER_STATE_FAILGUARDNEVERFIRED)) {
        return "a wildcard did not fire with its guard true, so the ancestor's "
               "transition was reached instead";
    }
    if (wildcard_in_document_order_ended_in(sm, WILDCARD_IN_DOCUMENT_ORDER_STATE_FAILGUARDEDINTERNALREENTERED)) {
        return "a guarded internal wildcard exited and re-entered its compound "
               "source, running its <onentry> twice";
    }
    if (wildcard_in_document_order_ended_in(sm, WILDCARD_IN_DOCUMENT_ORDER_STATE_FAILSEALEDINTERNALREENTERED)) {
        return "an unguarded internal wildcard exited and re-entered its compound "
               "source, running its <onentry> twice";
    }
    return "the machine reached no final state at all; resting in guardedFrom or "
           "sealedFrom means an internal wildcard was not taken";
}

int main(void) {
    wildcard_in_document_order_t sm;
    wildcard_in_document_order_init(&sm);
    wildcard_in_document_order_run(&sm);

    int rc = 0;
    if (!wildcard_in_document_order_ended_in(&sm, WILDCARD_IN_DOCUMENT_ORDER_STATE_PASS)) {
        fprintf(stderr, "wildcard_in_document_order: FAIL - %s\n", failure_case(&sm));
        rc = 1;
    }

    wildcard_in_document_order_destroy(&sm);
    return rc;
}
