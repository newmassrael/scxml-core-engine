// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE_MESH.md §9.5, §mesh-19 under `datamodel="sce-static"` (docs/adr/0005,
// decisions 5 and 7): an `<invoke type="sce:mesh-rpc">` whose peer and `<param>`s
// are typed expressions over the machine's own fields reaches the host's Mesh
// router through the runtime's mesh-rpc door — C11 AOT path. The script-model twin
// is `test_a_mesh_request_reaches_the_router.c`; the Rust, Go, Kotlin and Python
// ones drive the same document, `statechart_static_mesh_request`.
//
// The build lowers the invoke to a host-served one of type
// `SCE_MESH_RPC_INVOKE_TYPE`, so the machine carries an invoker registry although
// the build declared no invoker. The machine is built with NO script engine: its
// variables are fields, and a peer or a `<param>` that needed an engine to be read
// would not have one to ask. Linked WITHOUT `sce_c_scripting` and `lua54`, so the
// link is the proof.
//
// The run `bump`, `go` changes `load` and `label` before the request reads them,
// so a copy taken at start-up (3, "idle") is told from what the fields hold now
// (4, "busy"): the request carries 8 and "busy".

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "statechart_static_mesh_request_sm.h"

typedef statechart_static_mesh_request_t sm_t;

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

static bool drive(sm_t *sm, const char *name) {
    statechart_static_mesh_request_event_t event;
    if (!statechart_static_mesh_request_resolve_event_by_name(name, &event)) {
        (void)fprintf(stderr, "FAIL: the machine has no event `%s`\n", name);
        return false;
    }
    statechart_static_mesh_request_event_with_meta_t meta;
    memset(&meta, 0, sizeof(meta));
    meta.event = event;
    statechart_static_mesh_request_raise_external(sm, &meta);
    statechart_static_mesh_request_step(sm);
    return true;
}

// The machine standing at `idle`, with the router registered through the mesh-rpc
// door when `with_router`.
static void boot(sm_t *sm, request_t *request, bool with_router) {
    memset(request, 0, sizeof(*request));
    if (!with_router) {
        statechart_static_mesh_request_init(sm);
        statechart_static_mesh_request_run(sm);
        return;
    }
    sce_host_invoker_registry_t invokers;
    memset(&invokers, 0, sizeof(invokers));
    if (!sce_host_invoker_register_mesh_rpc_invoker(&invokers, router, request)) {
        (void)fprintf(stderr, "FAIL: the mesh-rpc door refused the router\n");
    }
    statechart_static_mesh_request_init_with_host_invokers(sm, &invokers);
    statechart_static_mesh_request_run(sm);
}

// The one request the router was started with, checked against what the
// document wrote and the fields held when it started.
static int check_request(const request_t *request, const char *scenario) {
    // The params as a set: a machine of this data model writes them in the order of
    // their names (the one order every engine writes a JSON object in), which is not
    // the order the document wrote them in, and the router is told by name.
    const char *want_params[] = {"_mesh_event=service.request.force;", "_mesh_deadline_ms=250;", "force=8;",
                                 "speed=busy;"};
    /* The author's pairs alone, typed: `force` was computed, `speed` is a
       string, and the envelope fields are not payload. */
    const char *want_event_data = "{\"force\":8,\"speed\":\"busy\"}";
    size_t want_count = sizeof(want_params) / sizeof(want_params[0]);
    size_t seen_count = 0u;
    for (const char *at = request->params; *at != '\0'; at++) {
        if (*at == ';') {
            seen_count++;
        }
    }
    bool params_match = seen_count == want_count;
    for (size_t i = 0u; i < want_count && params_match; i++) {
        params_match = strstr(request->params, want_params[i]) != NULL;
    }
    if (request->starts != 1 || strcmp(request->processor_type, SCE_MESH_RPC_INVOKE_TYPE) != 0 ||
        strcmp(request->invoke_id, "ask") != 0 || strcmp(request->src, "#motor") != 0 || !params_match ||
        strcmp(request->event_data, want_event_data) != 0) {
        (void)fprintf(stderr,
                      "FAIL [%s]: router saw %d start(s), last %s %s %s [%s] %s; want 1 %s ask #motor "
                      "[_mesh_event, _mesh_deadline_ms, force, speed] %s\n",
                      scenario, request->starts, request->processor_type, request->invoke_id, request->src,
                      request->params, request->event_data, SCE_MESH_RPC_INVOKE_TYPE, want_event_data);
        return 1;
    }
    return 0;
}

// The three counters as the run left them, and whether it ended in `done`.
static int check_counts(sm_t *sm, const char *scenario, uint32_t answered, uint32_t failed, uint32_t refused) {
    int bad = 0;
    if (statechart_static_mesh_request_get_answered(sm) != answered ||
        statechart_static_mesh_request_get_failed(sm) != failed ||
        statechart_static_mesh_request_get_refused(sm) != refused) {
        (void)fprintf(stderr, "FAIL [%s]: answered/failed/refused = %u/%u/%u, want %u/%u/%u\n", scenario,
                      (unsigned)statechart_static_mesh_request_get_answered(sm),
                      (unsigned)statechart_static_mesh_request_get_failed(sm),
                      (unsigned)statechart_static_mesh_request_get_refused(sm), (unsigned)answered, (unsigned)failed,
                      (unsigned)refused);
        bad = 1;
    }
    if (!statechart_static_mesh_request_ended_in(sm, STATECHART_STATIC_MESH_REQUEST_STATE_DONE)) {
        (void)fprintf(stderr, "FAIL [%s]: the run must end in `done`\n", scenario);
        bad = 1;
    }
    return bad;
}

// The router answers: done.invoke.ask ends the run.
static int a_static_mesh_request_is_answered_through_the_router(void) {
    static sm_t sm;
    request_t request;
    boot(&sm, &request, true);
    int bad = drive(&sm, "bump") && drive(&sm, "go") ? 0 : 1;
    bad |= check_request(&request, "answered");
    if (!statechart_static_mesh_request_complete_host_invoke(&sm, SCE_MESH_RPC_INVOKE_TYPE, "ask", request.token,
                                                             "\"ok\"")) {
        (void)fprintf(stderr, "FAIL [answered]: the running request's completion was refused\n");
        bad = 1;
    }
    statechart_static_mesh_request_run(&sm);
    bad |= check_counts(&sm, "answered", 1, 0, 0);
    statechart_static_mesh_request_destroy(&sm);
    return bad;
}

// The router fails the request: error.invoke.ask, carrying the router's data.
static int a_static_mesh_request_the_router_fails_is_error_invoke(void) {
    static sm_t sm;
    request_t request;
    boot(&sm, &request, true);
    int bad = drive(&sm, "bump") && drive(&sm, "go") ? 0 : 1;
    bad |= check_request(&request, "failed");
    if (!statechart_static_mesh_request_fail_host_invoke(&sm, SCE_MESH_RPC_INVOKE_TYPE, "ask", request.token,
                                                         "\"unreachable\"", "mesh://motor", NULL)) {
        (void)fprintf(stderr, "FAIL [failed]: the running request's failure was refused\n");
        bad = 1;
    }
    statechart_static_mesh_request_run(&sm);
    bad |= check_counts(&sm, "failed", 0, 1, 0);
    statechart_static_mesh_request_destroy(&sm);
    return bad;
}

// The control: nothing has written a field, so the request carries what `<data
// expr>` gave them and not the values `bump` would have set.
static int the_request_carries_the_fields_as_they_stand_and_not_a_copy_from_start_up(void) {
    static sm_t sm;
    request_t request;
    boot(&sm, &request, true);
    int bad = drive(&sm, "go") ? 0 : 1;
    if (request.starts != 1 || strcmp(request.event_data, "{\"force\":6,\"speed\":\"idle\"}") != 0) {
        (void)fprintf(stderr, "FAIL [control]: router saw %d start(s) with `%s`, want 1 with the declared values\n",
                      request.starts, request.event_data);
        bad = 1;
    }
    statechart_static_mesh_request_destroy(&sm);
    return bad;
}

// The peer is the string the machine holds when the invocation starts: `ghost`
// changes it, so the router is handed that name as the request's `src`, and the
// host's router is the one that looks it up among its bindings.
static int the_peer_is_the_string_the_machine_holds_when_the_request_starts(void) {
    static sm_t sm;
    request_t request;
    boot(&sm, &request, true);
    int bad = drive(&sm, "ghost") && drive(&sm, "go") ? 0 : 1;
    if (request.starts != 1 || strcmp(request.src, "#ghost") != 0) {
        (void)fprintf(stderr, "FAIL [peer]: router saw %d start(s) naming `%s`, want 1 naming `#ghost`\n",
                      request.starts, request.src);
        bad = 1;
    }
    statechart_static_mesh_request_destroy(&sm);
    return bad;
}

// With no router registered the invoke names a type nobody runs:
// error.execution (W3C SCXML 6.4.1).
static int a_static_mesh_request_with_no_router_registered_is_error_execution(void) {
    static sm_t sm;
    request_t request;
    boot(&sm, &request, false);
    int bad = drive(&sm, "bump") && drive(&sm, "go") ? 0 : 1;
    bad |= check_counts(&sm, "no router", 0, 0, 1);
    statechart_static_mesh_request_destroy(&sm);
    return bad;
}

int main(void) {
    int bad = 0;
    bad |= a_static_mesh_request_is_answered_through_the_router();
    bad |= a_static_mesh_request_the_router_fails_is_error_invoke();
    bad |= the_request_carries_the_fields_as_they_stand_and_not_a_copy_from_start_up();
    bad |= the_peer_is_the_string_the_machine_holds_when_the_request_starts();
    bad |= a_static_mesh_request_with_no_router_registered_is_error_execution();
    if (bad != 0) {
        return 1;
    }
    printf("PASS: a static mesh request reaches the router\n");
    return 0;
}
