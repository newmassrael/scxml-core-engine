// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 `<send>` `<param>` payload delivery — C11 AOT.
//
// Two send paths that were fixed at the template layer with no runtime
// witness, because no committed fixture had a machine of the required
// shape. The suites could only show that nothing regressed; that same
// absence is why the defects survived as long as they did.
//
//   engine-less child -> parent   param emission was gated on the
//     *machine* needing a script engine rather than on the send needing
//     one, so a `datamodel="null"` child shipped its `<send>` with the
//     params dropped.
//
//   #_internal                    the internal raise took no event data,
//     so params were built and then discarded.
//
// The two reach distinct final states, so a failure names the path.
//
// Fixture: integration_resources/send_param_payload/send_param_payload.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(send_param_payload ...)`
// in `backends/c/tests/CMakeLists.txt`. The build itself is the §6.2.6
// freshness invariant — there is no committed tree for the c11 backend.

#include <stdint.h>
#include <stdio.h>

#include "send_param_payload_sm.h"

int main(void) {
    send_param_payload_t sm;
    send_param_payload_init(&sm);

    // No `<send delay>` in this fixture: the child sends during its own
    // `_init`, and the parent's `#_internal` loopback is raised inside the
    // macrostep that consumes it.
    send_param_payload_run(&sm);

    const int pass = send_param_payload_in_state(&sm, SEND_PARAM_PAYLOAD_STATE_PASS);
    const int fail_child = send_param_payload_in_state(&sm, SEND_PARAM_PAYLOAD_STATE_FAILCHILDPAYLOAD);
    const int fail_internal = send_param_payload_in_state(&sm, SEND_PARAM_PAYLOAD_STATE_FAILINTERNALPAYLOAD);
    const int fail_number = send_param_payload_in_state(&sm, SEND_PARAM_PAYLOAD_STATE_FAILNUMBERTYPE);
    const int fail_string = send_param_payload_in_state(&sm, SEND_PARAM_PAYLOAD_STATE_FAILSTRINGTYPE);
    const int fail_dup = send_param_payload_in_state(&sm, SEND_PARAM_PAYLOAD_STATE_FAILDUPLICATEPARAMS);
    const int fail_no_error = send_param_payload_in_state(&sm, SEND_PARAM_PAYLOAD_STATE_FAILNOPARAMERROR);
    const int fail_broken_sent = send_param_payload_in_state(&sm, SEND_PARAM_PAYLOAD_STATE_FAILBROKENPARAMDELIVERED);
    const int fail_sibling = send_param_payload_in_state(&sm, SEND_PARAM_PAYLOAD_STATE_FAILSIBLINGPARAMLOST);

    if (!pass) {
        if (fail_child) {
            fprintf(stderr, "send_param_payload: FAIL — `fromChild` arrived without "
                            "`_event.data.value`. A datamodel=\"null\" child needs no script "
                            "engine, but its `<send>` still has to carry the params it "
                            "declares: the gate is whether this send folds to literals, not "
                            "whether the machine needs an engine.\n");
        } else if (fail_internal) {
            fprintf(stderr, "send_param_payload: FAIL — `loopback` arrived without "
                            "`_event.data.carried`. A `<send target=\"#_internal\">` must raise "
                            "its params as event data, not build them and drop them at the "
                            "internal-raise boundary.\n");
        } else if (fail_number) {
            fprintf(stderr, "send_param_payload: FAIL — `typed` arrived with `_event.data.n` "
                            "unequal to 7. `expr=\"7\"` is the Number 7, and a backend that "
                            "stringifies on the way through delivers \"7\", which `===` finds "
                            "unequal.\n");
        } else if (fail_string) {
            fprintf(stderr, "send_param_payload: FAIL — `typed` arrived with `_event.data.s` "
                            "unequal to 'kept'. A param that has to be EVALUATED reaches the "
                            "runtime serialiser, whose string arm must emit the value rather "
                            "than an engine spelling of it.\n");
        } else if (fail_dup) {
            fprintf(stderr, "send_param_payload: FAIL — `typed` did not carry both values of "
                            "the repeated name `d` with their types. W3C SCXML 6.2 lets a name "
                            "repeat and every value must be delivered.\n");
        } else if (fail_no_error) {
            fprintf(stderr, "send_param_payload: FAIL — `withBadParam` arrived with no "
                            "`error.execution` before it. W3C SCXML 5.7.1 puts that error on "
                            "the internal queue while the `<send>` is being evaluated, so it "
                            "is dequeued first.\n");
        } else if (fail_broken_sent) {
            fprintf(stderr, "send_param_payload: FAIL — `_event.data.broken` arrived as the "
                            "empty string. W3C SCXML 5.7.1 says ignore the name AND the value, "
                            "so a receiver must find no field at all rather than a placeholder "
                            "under the name.\n");
        } else if (fail_sibling) {
            fprintf(stderr, "send_param_payload: FAIL — `_event.data.kept` did not survive "
                            "alongside the failed param. One `<param>` that will not evaluate "
                            "costs its own pair and nothing else.\n");
        } else {
            fprintf(stderr,
                    "send_param_payload: FAIL — settled in no verdict state, so no send was "
                    "judged: the machine stalled, which is what discarding a whole `<send>` "
                    "over one unevaluable `<param>` does to a document (W3C SCXML 5.7.1 drops "
                    "the pair, not the message). Diagnostic: in_PASS=%d in_FAILCHILD=%d "
                    "in_FAILINTERNAL=%d\n",
                    pass, fail_child, fail_internal);
        }
    }

    send_param_payload_destroy(&sm);
    return pass ? 0 : 1;
}
