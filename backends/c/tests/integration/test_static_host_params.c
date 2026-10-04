// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15 — a `<param expr>` of a `<send>` or an `<invoke>` the
// HOST serves, in a `datamodel="sce-static"` machine, carries the value of a typed
// expression read from the machine's own fields when the send or the invoke
// happens. C11 compile+run gate; the Rust, Kotlin, Go, C++ and Python twins drive
// the same document, `statechart_static_host_params`.
//
// This is the row that holds a REAL on the wire here: `ratio` is a `float64`, and a
// real crosses as ECMAScript spells it (ARCHITECTURE.md, "JSON Number Text") — the
// text `1.5` in the request's params and the JSON number `1.5` in its event data.
//
// The machine is built with NO script engine: its variables are fields, and a
// `<param>` that needed an engine to be read would not have one to ask. Linked
// WITHOUT `sce_c_scripting` and `lua54`, so the link is the proof.
//
// The run `bump`, `go` changes every variable before the send and the invoke read
// it, so a copy taken at start-up (3, false, "idle") is told from what the fields
// hold now (4, true, "busy"). `boom` is a multiplication a 32-bit field cannot
// hold: its pair is left out and reported (W3C SCXML 5.7.1), while the message
// still goes and the invocation still starts.

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "statechart_static_host_params_sm.h"

#define DECLARED_TYPE "x-sce-host"

typedef statechart_static_host_params_t sm_t;

enum { MAX_PARAMS = 8 };

// What the host saw of one request: a `<send>`'s or an `<invoke>`'s.
typedef struct {
    int calls;
    char event_data[256];
    int param_count;
    char names[MAX_PARAMS][32];
    char values[MAX_PARAMS][64];
} seen_t;

typedef struct {
    seen_t send;
    seen_t invoke;
} host_t;

static void note_param(seen_t *seen, const char *name, const char *value) {
    if (seen->param_count >= MAX_PARAMS) {
        return;
    }
    (void)snprintf(seen->names[seen->param_count], sizeof(seen->names[0]), "%s", name);
    (void)snprintf(seen->values[seen->param_count], sizeof(seen->values[0]), "%s", value);
    seen->param_count++;
}

static void on_send(void *user_data, const sce_host_send_request_t *request, sce_host_send_response_list_t *out) {
    seen_t *seen = &((host_t *)user_data)->send;
    (void)out;
    seen->calls++;
    (void)snprintf(seen->event_data, sizeof(seen->event_data), "%s", request->event_data);
    for (int i = 0; i < request->param_count; i++) {
        note_param(seen, request->params[i].name, request->params[i].value);
    }
}

static void on_invoke(void *user_data, const sce_host_invoke_event_t *event, sce_host_invoke_response_t *out) {
    seen_t *seen = &((host_t *)user_data)->invoke;
    (void)out;
    if (event->phase != SCE_HOST_INVOKE_START) {
        return;
    }
    seen->calls++;
    (void)snprintf(seen->event_data, sizeof(seen->event_data), "%s", event->event_data);
    for (int i = 0; i < event->param_count; i++) {
        note_param(seen, event->params[i].name, event->params[i].value);
    }
}

static void boot(sm_t *sm, host_t *host) {
    sce_host_processor_registry_t processors;
    sce_host_invoker_registry_t invokers;
    memset(&processors, 0, sizeof(processors));
    memset(&invokers, 0, sizeof(invokers));
    (void)sce_host_registry_register(&processors, DECLARED_TYPE, on_send, host);
    (void)sce_host_invoker_register(&invokers, DECLARED_TYPE, on_invoke, host);
    statechart_static_host_params_init_with_host_surfaces(sm, &processors, &invokers);
}

static bool drive(sm_t *sm, const char *name) {
    statechart_static_host_params_event_t event;
    if (!statechart_static_host_params_resolve_event_by_name(name, &event)) {
        (void)fprintf(stderr, "FAIL: the machine has no event `%s`\n", name);
        return false;
    }
    statechart_static_host_params_event_with_meta_t meta;
    memset(&meta, 0, sizeof(meta));
    meta.event = event;
    statechart_static_host_params_raise_external(sm, &meta);
    statechart_static_host_params_step(sm);
    return true;
}

static const char *param_of(const seen_t *seen, const char *name) {
    for (int i = 0; i < seen->param_count; i++) {
        if (strcmp(seen->names[i], name) == 0) {
            return seen->values[i];
        }
    }
    return NULL;
}

static int expect_text(const char *what, const char *got, const char *want) {
    if (got == NULL || strcmp(got, want) != 0) {
        (void)fprintf(stderr, "FAIL: %s is `%s`, want `%s`\n", what, got != NULL ? got : "(absent)", want);
        return 1;
    }
    return 0;
}

// The text each param crosses as, given what `count`, `ready`, `label` and `twice`
// hold: `delta` and `ratio` never change, and `boom` is left out.
static int expect_params(const char *kind, const seen_t *seen, const char *count, const char *ready, const char *label,
                         const char *twice) {
    char what[64];
    int bad = 0;

    const struct {
        const char *name;
        const char *want;
    } wanted[] = {{"count", count}, {"ready", ready}, {"label", label},
                  {"twice", twice}, {"delta", "-5"},  {"ratio", "1.5"}};

    for (size_t i = 0u; i < sizeof(wanted) / sizeof(wanted[0]); i++) {
        (void)snprintf(what, sizeof(what), "%s param `%s`", kind, wanted[i].name);
        bad |= expect_text(what, param_of(seen, wanted[i].name), wanted[i].want);
    }
    if (param_of(seen, "boom") != NULL) {
        (void)fprintf(stderr, "FAIL: the failed pair `boom` is in the %s's params\n", kind);
        bad = 1;
    }
    if (seen->param_count != 6) {
        (void)fprintf(stderr, "FAIL: the %s carried %d params, want 6\n", kind, seen->param_count);
        bad = 1;
    }
    return bad;
}

// The pairs as the data model holds them — a number stays a number, a bool a bool,
// a string a string — in the one order every engine writes them in: ascending by
// name (ARCHITECTURE.md, "JSON Object Key Order"). `ratio` is the real: `1.5`, not
// `1.500000`.
static int expect_event_data(const char *kind, const seen_t *seen) {
    return expect_text(kind, seen->event_data,
                       "{\"count\":4,\"delta\":-5,\"label\":\"busy\",\"ratio\":1.5,\"ready\":true,\"twice\":8}");
}

static int a_send_param_carries_the_value_the_fields_hold_when_it_is_sent(void) {
    static sm_t sm;
    host_t host;
    memset(&host, 0, sizeof(host));
    boot(&sm, &host);
    int bad = drive(&sm, "bump") && drive(&sm, "go") ? 0 : 1;
    if (host.send.calls != 1) {
        (void)fprintf(stderr, "FAIL: one <send>, one request: the host saw %d\n", host.send.calls);
        statechart_static_host_params_destroy(&sm);
        return 1;
    }
    bad |= expect_params("send", &host.send, "4", "true", "busy", "8");
    bad |= expect_event_data("send event data", &host.send);
    statechart_static_host_params_destroy(&sm);
    return bad;
}

static int an_invoke_param_carries_the_value_the_fields_hold_when_it_starts(void) {
    static sm_t sm;
    host_t host;
    memset(&host, 0, sizeof(host));
    boot(&sm, &host);
    int bad = drive(&sm, "bump") && drive(&sm, "go") ? 0 : 1;
    if (host.invoke.calls != 1) {
        (void)fprintf(stderr, "FAIL: one <invoke>, one start: the host saw %d\n", host.invoke.calls);
        statechart_static_host_params_destroy(&sm);
        return 1;
    }
    // A copy taken at start-up would say count 3, ready false, label idle.
    bad |= expect_params("invoke", &host.invoke, "4", "true", "busy", "8");
    bad |= expect_event_data("invoke event data", &host.invoke);
    statechart_static_host_params_destroy(&sm);
    return bad;
}

// The same machine on the shorter run: nothing has written a variable, so the
// fields still hold what `<data expr>` gave them. The control that keeps the two
// cases above from passing on a value that is simply always the new one.
static int a_param_read_before_any_bump_carries_the_declared_values(void) {
    static sm_t sm;
    host_t host;
    memset(&host, 0, sizeof(host));
    boot(&sm, &host);
    int bad = drive(&sm, "go") ? 0 : 1;
    if (host.send.calls != 1 || host.invoke.calls != 1) {
        (void)fprintf(stderr, "FAIL: the host saw %d send(s) and %d start(s), want 1 and 1\n", host.send.calls,
                      host.invoke.calls);
        statechart_static_host_params_destroy(&sm);
        return 1;
    }
    bad |= expect_params("send", &host.send, "3", "false", "idle", "6");
    bad |= expect_params("invoke", &host.invoke, "3", "false", "idle", "6");
    statechart_static_host_params_destroy(&sm);
    return bad;
}

// W3C SCXML 5.7.1: a `<param>` whose value cannot be computed is reported with
// `error.execution` and its pair left out, while the message still goes and the
// invocation still starts. `errors` counts the reports the document took, one for
// the send and one for the invoke, so a pair dropped in silence is told from one
// reported.
static int a_param_whose_value_cannot_be_computed_is_reported_and_left_out(void) {
    static sm_t sm;
    host_t host;
    memset(&host, 0, sizeof(host));
    boot(&sm, &host);
    int bad = drive(&sm, "bump") && drive(&sm, "go") ? 0 : 1;
    if (host.send.calls != 1 || host.invoke.calls != 1) {
        (void)fprintf(stderr, "FAIL: the send still went and the invoke still started: saw %d and %d\n",
                      host.send.calls, host.invoke.calls);
        bad = 1;
    }
    if (statechart_static_host_params_get_errors(&sm) != 2u) {
        (void)fprintf(stderr,
                      "FAIL: errors is %u, want 2: one error.execution for the send's `boom`, one for the invoke's\n",
                      (unsigned)statechart_static_host_params_get_errors(&sm));
        bad = 1;
    }
    statechart_static_host_params_destroy(&sm);
    return bad;
}

int main(void) {
    int bad = 0;
    bad |= a_send_param_carries_the_value_the_fields_hold_when_it_is_sent();
    bad |= an_invoke_param_carries_the_value_the_fields_hold_when_it_starts();
    bad |= a_param_read_before_any_bump_carries_the_declared_values();
    bad |= a_param_whose_value_cannot_be_computed_is_reported_and_left_out();
    if (bad != 0) {
        return 1;
    }
    (void)printf("static host params: ok\n");
    return 0;
}
