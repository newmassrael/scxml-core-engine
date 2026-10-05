// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15 — a `<param>` name that repeats collects its values, in
// document order, into one array of the event's data (ARCHITECTURE.md, "JSON Object
// Key Order"; W3C SCXML test178), in a `<send>` the host serves, in an `<invoke>` the
// host serves, and in a final's `<donedata>`. C11 compile+run gate for the document
// `integration_resources/static_param_repeat/`, which is C11's alone: the other
// engines are held to the same case by `tests/json_text/object_key_order.json`, and
// the writer is held to it by `unit/forge_wire_repeat_test.c`.
//
// The machine is built with NO script engine: its variables are fields, and a
// `<param>` that needed an engine to be read would not have one to ask. Linked
// WITHOUT `sce_c_scripting` and `lua54`, so the link is the proof.
//
// What this holds, in the generated machine and not the writer alone:
//
//   * a send's `n` is `[3,4,5]` and its `m` is 6, the values of a name in the order
//     the document declares them although `m` is declared between them, and the
//     request's `params` carry the four pairs as text;
//   * an invoke's `n` is `[3,8]`: the value that cannot be computed is left out and
//     reported, the others are one array; `p` has one value left, which is that
//     value and not an array of one;
//   * a final's `<donedata>` is `{"n":[3,4]}`.

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "static_param_repeat_sm.h"

#define DECLARED_TYPE "x-sce-host"
#define MAX_PARAMS 8

typedef static_param_repeat_t sm_t;

// What the host saw of one request: the `<send>`'s or the `<invoke>`'s.
typedef struct {
    int calls;
    char event_data[128];
    int param_count;
    char names[MAX_PARAMS][16];
    char values[MAX_PARAMS][16];
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

static void drive(sm_t *sm, const char *name) {
    static_param_repeat_event_t event;
    if (!static_param_repeat_resolve_event_by_name(name, &event)) {
        (void)fprintf(stderr, "FAIL: the machine has no event `%s`\n", name);
        return;
    }
    static_param_repeat_event_with_meta_t meta;
    memset(&meta, 0, sizeof(meta));
    meta.event = event;
    static_param_repeat_raise_external(sm, &meta);
    static_param_repeat_step(sm);
}

static int expect_int(const char *what, long got, long want) {
    if (got != want) {
        (void)fprintf(stderr, "FAIL: %s is %ld, want %ld\n", what, got, want);
        return 1;
    }
    return 0;
}

static int expect_text(const char *what, const char *got, const char *want) {
    if (strcmp(got, want) != 0) {
        (void)fprintf(stderr, "FAIL: %s is `%s`, want `%s`\n", what, got, want);
        return 1;
    }
    return 0;
}

// The text of the request's `params`, one entry for each value, the pairs listed
// by name as the generator lists them: `m` first, then the three `n`s.
static int the_params_carry_each_value_as_text(const seen_t *seen) {
    static const char *const names[] = {"m", "n", "n", "n"};
    static const char *const values[] = {"6", "3", "4", "5"};
    int bad = expect_int("params carried by the send", seen->param_count, 4);
    for (int i = 0; i < seen->param_count && i < 4; ++i) {
        bad |= expect_text("param name", seen->names[i], names[i]);
        bad |= expect_text("param value", seen->values[i], values[i]);
    }
    return bad;
}

int main(void) {
    static sm_t sm;
    host_t host;
    memset(&host, 0, sizeof(host));
    sce_host_processor_registry_t processors;
    sce_host_invoker_registry_t invokers;
    memset(&processors, 0, sizeof(processors));
    memset(&invokers, 0, sizeof(invokers));
    (void)sce_host_registry_register(&processors, DECLARED_TYPE, on_send, &host);
    (void)sce_host_invoker_register(&invokers, DECLARED_TYPE, on_invoke, &host);
    static_param_repeat_init_with_host_surfaces(&sm, &processors, &invokers);

    drive(&sm, "go");
    int bad = expect_int("sends the host served", host.send.calls, 1);
    bad |= expect_text("the send's event data", host.send.event_data, "{\"m\":6,\"n\":[3,4,5]}");
    bad |= the_params_carry_each_value_as_text(&host.send);

    bad |= expect_int("invocations the host started", host.invoke.calls, 1);
    bad |= expect_text("the invoke's event data", host.invoke.event_data, "{\"n\":[3,8],\"p\":3}");
    // One error.execution for each value that cannot be computed: the invoke's `n`
    // and its `p`.
    bad |= expect_int("error.execution the document took", (long)static_param_repeat_get_errors(&sm), 2);

    drive(&sm, "finish");
    bad |= expect_int("the machine ended in its final", static_param_repeat_is_in_final_state(&sm) ? 1 : 0, 1);
    bad |= expect_text("the final's done data", static_param_repeat_done_data(&sm), "{\"n\":[3,4]}");

    static_param_repeat_destroy(&sm);
    if (bad != 0) {
        return 1;
    }
    (void)printf("static param repeat: ok\n");
    return 0;
}
