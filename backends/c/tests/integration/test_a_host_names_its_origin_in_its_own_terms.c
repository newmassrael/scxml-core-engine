// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1 (docs/adr/0090, test 3): a host names the origin of an event
// it delivers in its own terms — an ending of a host-run invocation whose origin
// is {x-sce-host, motor} reads back `motor` and `x-sce-host` — C11 AOT. The
// address has no scheme, so it is no SCXML location, and the type is the
// host's: the engine neither wraps the one nor defaults the other.
//
// Fixture:
// integration_resources/a_host_names_its_origin_in_its_own_terms/a_host_names_its_origin_in_its_own_terms.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python / Swift channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(a_host_names_its_origin_in_its_own_terms ... HOST_INVOKER x-sce-host)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "../require_started.h"
#include "a_host_names_its_origin_in_its_own_terms_sm.h"

typedef a_host_names_its_origin_in_its_own_terms_t sm_t;

// The type the build was told the host runs; the CMake registration passes this
// same string to `--host-invoker`.
#define HOST_TYPE "x-sce-host"

typedef struct {
    int starts;
    uint64_t token;
} run_t;

// An invoker whose work outlives the start: the test ends it, naming the party
// it stands for.
static void running_invoker(void *user_data, const sce_host_invoke_event_t *event, sce_host_invoke_response_t *out) {
    run_t *run = (run_t *)user_data;
    (void)out;
    if (event->phase == SCE_HOST_INVOKE_START) {
        run->starts++;
        run->token = event->token;
    }
}

int main(void) {
    sm_t sm;
    run_t run;
    sce_host_invoker_registry_t wiring;
    memset(&run, 0, sizeof(run));
    memset(&wiring, 0, sizeof(wiring));
    if (!sce_host_invoker_register(&wiring, HOST_TYPE, running_invoker, &run)) {
        fprintf(stderr, "FAIL: the host could not register its invoker\n");
        return 1;
    }
    REQUIRE_STARTED(a_host_names_its_origin_in_its_own_terms_init_with_host_invokers(&sm, &wiring),
                    A_HOST_NAMES_ITS_ORIGIN_IN_ITS_OWN_TERMS_START_OK);
    a_host_names_its_origin_in_its_own_terms_run(&sm);

    int ok = 1;
    if (run.starts != 1) {
        fprintf(stderr, "FAIL: the host was asked to run `job` %d times, want 1\n", run.starts);
        a_host_names_its_origin_in_its_own_terms_destroy(&sm);
        return 1;
    }

    sce_event_origin_t named;
    if (!sce_event_origin_set(&named, HOST_TYPE, "motor")) {
        fprintf(stderr, "FAIL: the host's pair did not fit\n");
        ok = 0;
    }
    if (!a_host_names_its_origin_in_its_own_terms_complete_host_invoke_from(&sm, HOST_TYPE, "job", run.token, "{}",
                                                                            &named)) {
        fprintf(stderr, "FAIL: the running invocation's completion was refused\n");
        ok = 0;
    }
    a_host_names_its_origin_in_its_own_terms_run(&sm);

    if (!a_host_names_its_origin_in_its_own_terms_ended_in(&sm, A_HOST_NAMES_ITS_ORIGIN_IN_ITS_OWN_TERMS_STATE_DONE)) {
        fprintf(stderr, "FAIL: the run must end in `done`\n");
        ok = 0;
    }
    int64_t host_named = -1;
    if (!a_host_names_its_origin_in_its_own_terms_host_named(&sm, &host_named) || host_named != 1) {
        fprintf(stderr,
                "FAIL: hostNamed = %lld, want 1: the completion did not read back the address and the type the "
                "host named\n",
                (long long)host_named);
        ok = 0;
    }
    a_host_names_its_origin_in_its_own_terms_destroy(&sm);
    if (!ok) {
        return 1;
    }
    printf("PASS: the host named its origin in its own terms\n");
    return 0;
}
