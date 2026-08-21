// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML C.2 + 6.2.3 `<send namelist>` over BasicHTTP — C11 AOT path.
//
// Two claims the IRP corpus states and cannot measure:
//
//   the namelist reaches the form   test518 is titled "namelist values get
//     encoded as POST parameters" and its whole verdict is `<transition
//     event="test" target="pass"/>` — it passes as soon as the event comes
//     back, whatever the message carried.
//
//   an unreadable item reports AND discards   W3C SCXML 5.9.2 puts
//     `error.execution` on the internal queue; 6.2.3 discards the message.
//     `<param>`'s per-item exception (5.7.1, "ignore the name and value")
//     has no counterpart for a namelist anywhere in the specification.
//
// This backend answered the second claim twice, differently. The
// SCXMLEventProcessor arm reads a namelist item with `lua_eval_namelist_var`'s
// declared-guard and raises `error.execution` at run time — correct, and W3C
// test553 witnesses it. The BasicHTTP arm lowered the same item through the
// ECMAScript frontend, which REFUSED the document instead, so a `<send
// namelist>` naming something the datamodel does not hold stopped being a
// runtime error and became a document this backend would not generate.
// test553 could not catch it: that document declares no `<data>` at all, so
// the frontend's unknown-identifier check had nothing to compare against.
//
// Fixture: integration_resources/send_namelist_over_http/send_namelist_over_http.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(send_namelist_over_http ...)`
// in `backends/c/tests/CMakeLists.txt`.
//
// Needs the W3C harness server on localhost:8080/test — the ctest entry
// declares `FIXTURES_REQUIRED w3c_c_http_server`, the same listener the C11
// W3C BasicHTTP fixtures use.

// `nanosleep` is POSIX, and the target is built with C_EXTENSIONS OFF — so
// the feature-test macro has to precede every include or `<time.h>` hides it.
#define _POSIX_C_SOURCE 199309L

#include <stdio.h>
#include <time.h>

#include "send_namelist_over_http_sm.h"

// §scxml-C-2-3: where the harness's inbound BasicHTTP listener answers, and
// therefore the address this machine publishes as its `_ioprocessors`
// location. The document addresses its own send through that entry rather
// than through a literal URL, so bind address and published address stay one
// fact.
static const char *const HTTP_ACCESS_URI = "http://localhost:8080/test";

// The fixture settles each phase with a delayed `<send>` (3 s then 2 s), so
// the host drives a real clock rather than a manual one: the HTTP round trip
// takes wall time that a stepped clock would not advance past.
static bool run_to_final(send_namelist_over_http_t *sm, unsigned budget_ms) {
    const struct timespec nap = {0, 10 * 1000 * 1000};  // 10 ms
    unsigned waited = 0u;
    while (waited < budget_ms) {
        send_namelist_over_http_tick(sm);
        send_namelist_over_http_step(sm);
        if (send_namelist_over_http_is_in_final_state(sm)) {
            return true;
        }
        nanosleep(&nap, NULL);
        waited += 10u;
    }
    return false;
}

int main(void) {
    int rc = 0;

    send_namelist_over_http_t sm;
    send_namelist_over_http_init_with_basic_http(&sm, HTTP_ACCESS_URI);

    if (!run_to_final(&sm, 15000u)) {
        printf("FAIL: send_namelist_over_http never reached a final state — the "
               "delayed timeoutMap / timeoutDiscard sends that give each phase "
               "its verdict never fired\n");
        send_namelist_over_http_destroy(&sm);
        return 1;
    }

    if (send_namelist_over_http_in_state(&sm, SEND_NAMELIST_OVER_HTTP_STATE_PASS)) {
        printf("PASS: namelist reached the form and a broken item discarded the "
               "message\n");
    } else if (send_namelist_over_http_in_state(&sm, SEND_NAMELIST_OVER_HTTP_STATE_FAILNAMELISTNEVERARRIVED)) {
        printf("FAIL: the BasicHTTP send never came back — the harness server did "
               "not answer, which is a different failure from posting the wrong "
               "form\n");
        rc = 1;
    } else if (send_namelist_over_http_in_state(&sm, SEND_NAMELIST_OVER_HTTP_STATE_FAILNAMELISTNOTPOSTED)) {
        printf("FAIL: `mapped` arrived without `Var1` in its data — W3C SCXML C.2 "
               "requires a namelist's variable names and values to be mapped to "
               "HTTP POST parameters\n");
        rc = 1;
    } else if (send_namelist_over_http_in_state(&sm, SEND_NAMELIST_OVER_HTTP_STATE_FAILMESSAGENOTDISCARDED)) {
        printf("FAIL: `shouldNotArrive` was delivered — W3C SCXML 6.2.3 discards "
               "the message when the evaluation of a <send>'s arguments produces "
               "an error\n");
        rc = 1;
    } else if (send_namelist_over_http_in_state(&sm, SEND_NAMELIST_OVER_HTTP_STATE_FAILNONAMELISTERROR)) {
        printf("FAIL: no `error.execution` preceded the timeout — W3C SCXML 5.9.2 "
               "requires it when a location expression yields no valid location\n");
        rc = 1;
    } else {
        printf("FAIL: settled in a state that is not a verdict state (active "
               "bitmap 0x%08x)\n",
               send_namelist_over_http_active_states(&sm));
        rc = 1;
    }

    send_namelist_over_http_destroy(&sm);
    return rc;
}
