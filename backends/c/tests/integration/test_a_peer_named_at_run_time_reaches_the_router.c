// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE_MESH.md §mesh-19: a `targetexpr` that evaluates to a Mesh peer reaches
// the host's Mesh router — C11 AOT path.
//
// The build cannot lower this send, because the peer is only named once the
// expression is evaluated; the C11 send template asks the evaluated target
// (`sce_is_mesh_target`) and hands a peer to the router as a literal `#peer`
// is handed. The same two scenarios the Rust, Go, Kotlin and Python drivers
// run, with the same expected counts.
//
// Fixture: integration_resources/a_peer_named_at_run_time_reaches_the_router/
// a_peer_named_at_run_time_reaches_the_router.scxml (canonical, shared with the
// other channels).

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "a_peer_named_at_run_time_reaches_the_router_sm.h"

typedef a_peer_named_at_run_time_reaches_the_router_t sm_t;

// What the router saw, in call order.
typedef struct {
    int count;
    char processor_type[64];
    char target[64];
    char event[64];
} routed_t;

static void router(void *user_data, const sce_host_send_request_t *request, sce_host_send_response_list_t *out) {
    routed_t *routed = (routed_t *)user_data;
    routed->count++;
    (void)snprintf(routed->processor_type, sizeof(routed->processor_type), "%s",
                   request->processor_type != NULL ? request->processor_type : "");
    (void)snprintf(routed->target, sizeof(routed->target), "%s", request->target != NULL ? request->target : "");
    (void)snprintf(routed->event, sizeof(routed->event), "%s", request->event_name != NULL ? request->event_name : "");
    (void)out;
}

// `refused` and `looped` as the run left them, and whether it ended in `done`.
static int check_counts(sm_t *sm, const char *scenario, int64_t want_refused) {
    int bad = 0;
    int64_t refused = -1;
    int64_t looped = -1;
    if (!a_peer_named_at_run_time_reaches_the_router_refused(sm, &refused) || refused != want_refused) {
        (void)fprintf(stderr, "FAIL [%s]: refused = %lld, want %lld\n", scenario, (long long)refused,
                      (long long)want_refused);
        bad = 1;
    }
    if (!a_peer_named_at_run_time_reaches_the_router_looped(sm, &looped) || looped != 1) {
        (void)fprintf(stderr, "FAIL [%s]: looped = %lld, want 1\n", scenario, (long long)looped);
        bad = 1;
    }
    if (!a_peer_named_at_run_time_reaches_the_router_ended_in(sm,
                                                              A_PEER_NAMED_AT_RUN_TIME_REACHES_THE_ROUTER_STATE_DONE)) {
        (void)fprintf(stderr, "FAIL [%s]: the run must end in `done`\n", scenario);
        bad = 1;
    }
    return bad;
}

// With a router registered, the peer send is the router's — processor
// `sce:mesh`, the evaluated target, the event — and the non-peer target is
// still this session's.
static int a_peer_named_at_run_time_is_sent_to_the_router(void) {
    sm_t sm;
    routed_t routed;
    memset(&routed, 0, sizeof(routed));
    sce_host_processor_registry_t wiring;
    memset(&wiring, 0, sizeof(wiring));
    if (!sce_host_registry_register_mesh_router(&wiring, router, &routed)) {
        (void)fprintf(stderr, "FAIL [routed]: the router door refused the router\n");
        return 1;
    }
    a_peer_named_at_run_time_reaches_the_router_init_with_host_processors(&sm, &wiring);
    a_peer_named_at_run_time_reaches_the_router_run(&sm);

    int bad = check_counts(&sm, "routed", 0);
    if (routed.count != 1 || strcmp(routed.processor_type, SCE_MESH_PROCESSOR_TYPE) != 0 ||
        strcmp(routed.target, "#hmi") != 0 || strcmp(routed.event, "ping") != 0) {
        (void)fprintf(stderr, "FAIL [routed]: router saw %d send(s), last %s %s %s; want 1 %s #hmi ping\n",
                      routed.count, routed.processor_type, routed.target, routed.event, SCE_MESH_PROCESSOR_TYPE);
        bad = 1;
    }
    a_peer_named_at_run_time_reaches_the_router_destroy(&sm);
    return bad;
}

// With no router registered, the peer send is a send to a host processor
// nobody serves: one error.execution (W3C SCXML 6.2.5), and the non-peer
// target is unaffected.
static int a_peer_with_no_router_registered_is_error_execution(void) {
    sm_t sm;
    a_peer_named_at_run_time_reaches_the_router_init(&sm);
    a_peer_named_at_run_time_reaches_the_router_run(&sm);
    int bad = check_counts(&sm, "no router", 1);
    a_peer_named_at_run_time_reaches_the_router_destroy(&sm);
    return bad;
}

int main(void) {
    int bad = 0;
    bad |= a_peer_named_at_run_time_is_sent_to_the_router();
    bad |= a_peer_with_no_router_registered_is_error_execution();
    if (bad != 0) {
        return 1;
    }
    printf("PASS: a peer named at run time reaches the router\n");
    return 0;
}
