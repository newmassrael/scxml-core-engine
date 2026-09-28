// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE_MESH.md §9.5, §mesh-19: an `<invoke type="sce:mesh-rpc">` reaches the
// host's Mesh router through the runtime's mesh-rpc door — C11 AOT path.
//
// The build lowers the invoke to a host-served one of type
// `SCE_MESH_RPC_INVOKE_TYPE`, so the machine carries an invoker registry
// although the build declared no invoker. The invoke belongs to the initial
// configuration and so starts inside `_init`: the router arrives through
// `_init_with_host_invokers`, in a registry the mesh-rpc door filled. The
// same three scenarios the Rust, Go, Kotlin and Python drivers run, with the
// same expected counts.
//
// Fixture: integration_resources/a_mesh_request_reaches_the_router/
// a_mesh_request_reaches_the_router.scxml (canonical, shared with the other
// channels).

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "a_mesh_request_reaches_the_router_sm.h"

typedef a_mesh_request_reaches_the_router_t sm_t;

// The request the router was started with, copied out of the call.
typedef struct {
    int starts;
    uint64_t token;
    char processor_type[64];
    char invoke_id[64];
    char src[64];
    char params[256];
    char event_data[256];
} request_t;

// Keeps what it is started with and answers nothing, so the test decides how
// the request ends.
static void router(void *user_data, const sce_host_invoke_event_t *event, sce_host_invoke_response_t *out) {
    request_t *request = (request_t *)user_data;
    (void)out;
    if (event->phase != SCE_HOST_INVOKE_START) {
        return;
    }
    request->starts++;
    request->token = event->token;
    (void)snprintf(request->processor_type, sizeof(request->processor_type), "%s", event->processor_type);
    (void)snprintf(request->invoke_id, sizeof(request->invoke_id), "%s", event->invoke_id);
    (void)snprintf(request->src, sizeof(request->src), "%s", event->src);
    (void)snprintf(request->event_data, sizeof(request->event_data), "%s", event->event_data);
    request->params[0] = '\0';
    for (int i = 0; i < event->param_count; i++) {
        size_t used = strlen(request->params);
        (void)snprintf(request->params + used, sizeof(request->params) - used, "%s=%s;", event->params[i].name,
                       event->params[i].value);
    }
}

// The machine run into `asking`, with the router registered through the
// mesh-rpc door.
static void start_with_router(sm_t *sm, request_t *request) {
    sce_host_invoker_registry_t invokers;
    memset(&invokers, 0, sizeof(invokers));
    memset(request, 0, sizeof(*request));
    if (!sce_host_invoker_register_mesh_rpc_invoker(&invokers, router, request)) {
        (void)fprintf(stderr, "FAIL: the mesh-rpc door refused the router\n");
    }
    a_mesh_request_reaches_the_router_init_with_host_invokers(sm, &invokers);
    a_mesh_request_reaches_the_router_run(sm);
}

// The one request the router was started with, checked against what the
// document wrote.
static int check_request(const request_t *request, const char *scenario) {
    const char *want_params = "_mesh_event=service.request.force;_mesh_deadline_ms=250;force=3;speed=3;";
    /* The author's pairs alone, typed: `force` was computed, `speed` was
       written as a string, and the envelope fields are not payload. */
    const char *want_event_data = "{\"force\":3,\"speed\":\"3\"}";
    if (request->starts != 1 || strcmp(request->processor_type, SCE_MESH_RPC_INVOKE_TYPE) != 0 ||
        strcmp(request->invoke_id, "ask") != 0 || strcmp(request->src, "#motor") != 0 ||
        strcmp(request->params, want_params) != 0 || strcmp(request->event_data, want_event_data) != 0) {
        (void)fprintf(stderr,
                      "FAIL [%s]: router saw %d start(s), last %s %s %s [%s] %s; want 1 %s ask #motor [%s] %s\n",
                      scenario, request->starts, request->processor_type, request->invoke_id, request->src,
                      request->params, request->event_data, SCE_MESH_RPC_INVOKE_TYPE, want_params, want_event_data);
        return 1;
    }
    return 0;
}

// The three counters as the run left them, and whether it ended in `done`.
static int check_counts(sm_t *sm, const char *scenario, int64_t answered, int64_t failed, int64_t refused) {
    int64_t got[3] = {-1, -1, -1};
    bool read = a_mesh_request_reaches_the_router_answered(sm, &got[0]) &&
                a_mesh_request_reaches_the_router_failed(sm, &got[1]) &&
                a_mesh_request_reaches_the_router_refused(sm, &got[2]);
    int bad = 0;
    if (!read || got[0] != answered || got[1] != failed || got[2] != refused) {
        (void)fprintf(stderr, "FAIL [%s]: answered/failed/refused = %lld/%lld/%lld, want %lld/%lld/%lld\n", scenario,
                      (long long)got[0], (long long)got[1], (long long)got[2], (long long)answered, (long long)failed,
                      (long long)refused);
        bad = 1;
    }
    if (!a_mesh_request_reaches_the_router_ended_in(sm, A_MESH_REQUEST_REACHES_THE_ROUTER_STATE_DONE)) {
        (void)fprintf(stderr, "FAIL [%s]: the run must end in `done`\n", scenario);
        bad = 1;
    }
    return bad;
}

// The router answers: done.invoke.ask ends the run.
static int a_mesh_request_is_answered_through_the_router(void) {
    sm_t sm;
    request_t request;
    start_with_router(&sm, &request);
    int bad = check_request(&request, "answered");
    if (!a_mesh_request_reaches_the_router_complete_host_invoke(&sm, SCE_MESH_RPC_INVOKE_TYPE, "ask", request.token,
                                                                "\"ok\"")) {
        (void)fprintf(stderr, "FAIL [answered]: the running request's completion was refused\n");
        bad = 1;
    }
    a_mesh_request_reaches_the_router_run(&sm);
    bad |= check_counts(&sm, "answered", 1, 0, 0);
    a_mesh_request_reaches_the_router_destroy(&sm);
    return bad;
}

// The router fails the request: error.invoke.ask, carrying the router's data.
static int a_mesh_request_the_router_fails_is_error_invoke(void) {
    sm_t sm;
    request_t request;
    start_with_router(&sm, &request);
    int bad = check_request(&request, "failed");
    if (!a_mesh_request_reaches_the_router_fail_host_invoke(&sm, SCE_MESH_RPC_INVOKE_TYPE, "ask", request.token,
                                                            "\"unreachable\"", "mesh://motor", NULL)) {
        (void)fprintf(stderr, "FAIL [failed]: the running request's failure was refused\n");
        bad = 1;
    }
    a_mesh_request_reaches_the_router_run(&sm);
    bad |= check_counts(&sm, "failed", 0, 1, 0);
    a_mesh_request_reaches_the_router_destroy(&sm);
    return bad;
}

// With no router registered the invoke names a type nobody runs:
// error.execution (W3C SCXML 6.4.1).
static int a_mesh_request_with_no_router_registered_is_error_execution(void) {
    sm_t sm;
    a_mesh_request_reaches_the_router_init(&sm);
    a_mesh_request_reaches_the_router_run(&sm);
    int bad = check_counts(&sm, "no router", 0, 0, 1);
    a_mesh_request_reaches_the_router_destroy(&sm);
    return bad;
}

int main(void) {
    int bad = 0;
    bad |= a_mesh_request_is_answered_through_the_router();
    bad |= a_mesh_request_the_router_fails_is_error_invoke();
    bad |= a_mesh_request_with_no_router_registered_is_error_execution();
    if (bad != 0) {
        return 1;
    }
    printf("PASS: a mesh request reaches the router\n");
    return 0;
}
