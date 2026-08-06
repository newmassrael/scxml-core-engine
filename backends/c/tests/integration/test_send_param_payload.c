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
        } else {
            fprintf(stderr,
                    "send_param_payload: FAIL — settled in no verdict state, so neither "
                    "send was judged. Diagnostic: in_PASS=%d in_FAILCHILD=%d "
                    "in_FAILINTERNAL=%d\n",
                    pass, fail_child, fail_internal);
        }
    }

    send_param_payload_destroy(&sm);
    return pass ? 0 : 1;
}
