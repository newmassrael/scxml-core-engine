// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15 — a `<param expr>` of an `<invoke>` the HOST serves,
// in a `datamodel="sce-static"` machine, carries the value of a typed expression
// read from the machine's own fields when the invocation starts. C11 compile+run
// gate; the Rust, Kotlin, Go, C++ and Python twins drive `statechart_static_host_params`
// (a `<send>` and an `<invoke>` both), where this one drives the invoke-only
// document the saved-state suites use: C11 serves the invoke and not yet a host
// `<send>`.
//
// The machine is built with NO script engine: its variables are fields, and a
// `<param>` that needed an engine to be read would not have one to ask. Linked
// WITHOUT `sce_c_scripting` and `lua54`, so the link is the proof.
//
// Fixtures, both generated WITH `--host-invoker x-sce-host` by
// `backends/c/tests/CMakeLists.txt`:
//
//   * sce-build/tests/fixtures/host_processor/statechart_static_host_invoke.scxml
//     — what is on the wire. `bump` raises `job` before `start`, so a copy taken
//     at start-up (7) is told from what the field holds now (8). The
//     `_sce_deadline_ms` param is the engine's and is not handed to the host, but
//     its pair is in the request's event data, as the saved state holds it.
//   * backends/c/tests/integration_resources/static_host_invoke_overflow/ — a
//     value that cannot be computed is left out and reported, and the invocation
//     still starts (W3C SCXML 5.7.1). A document of this channel alone: the
//     others hold the rule in `statechart_static_host_params.scxml`.

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "statechart_static_host_invoke_sm.h"
#include "static_host_invoke_overflow_sm.h"

#define DECLARED_TYPE "x-sce-host"

// What the invoker saw of the one start.
typedef struct {
    int starts;
    char invoke_id[32];
    char src[64];
    char content[32];
    char event_data[256];
    int param_count;
    char names[8][32];
    char values[8][64];
} seen_t;

static void invoker(void *user_data, const sce_host_invoke_event_t *event, sce_host_invoke_response_t *out) {
    seen_t *seen = (seen_t *)user_data;
    (void)out;
    if (event->phase != SCE_HOST_INVOKE_START) {
        return;
    }
    seen->starts++;
    (void)snprintf(seen->invoke_id, sizeof(seen->invoke_id), "%s", event->invoke_id);
    (void)snprintf(seen->src, sizeof(seen->src), "%s", event->src);
    (void)snprintf(seen->content, sizeof(seen->content), "%s", event->content);
    (void)snprintf(seen->event_data, sizeof(seen->event_data), "%s", event->event_data);
    seen->param_count = event->param_count < 8 ? event->param_count : 8;
    for (int i = 0; i < seen->param_count; i++) {
        (void)snprintf(seen->names[i], sizeof(seen->names[i]), "%s", event->params[i].name);
        (void)snprintf(seen->values[i], sizeof(seen->values[i]), "%s", event->params[i].value);
    }
}

static void boot(statechart_static_host_invoke_t *sm, seen_t *seen) {
    sce_host_invoker_registry_t wiring;
    memset(&wiring, 0, sizeof(wiring));
    (void)sce_host_invoker_register(&wiring, DECLARED_TYPE, invoker, seen);
    statechart_static_host_invoke_init_with_host_invokers(sm, &wiring);
}

static bool send_event(statechart_static_host_invoke_t *sm, const char *name) {
    statechart_static_host_invoke_event_t event;
    if (!statechart_static_host_invoke_resolve_event_by_name(name, &event)) {
        return false;
    }
    statechart_static_host_invoke_event_with_meta_t meta;
    memset(&meta, 0, sizeof(meta));
    meta.event = event;
    statechart_static_host_invoke_raise_external(sm, &meta);
    statechart_static_host_invoke_step(sm);
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

// The request carries what the fields hold when the invocation starts: `bump`
// made `job` 8 before `start`, and `label` is the string the document gave it.
static int a_param_carries_the_value_the_fields_hold_when_the_invocation_starts(void) {
    static statechart_static_host_invoke_t sm;
    seen_t seen;
    memset(&seen, 0, sizeof(seen));
    boot(&sm, &seen);
    int bad = send_event(&sm, "bump") ? 0 : 1;
    bad |= send_event(&sm, "start") ? 0 : 1;

    if (seen.starts != 1) {
        (void)fprintf(stderr, "FAIL: the host saw %d starts, want 1\n", seen.starts);
        statechart_static_host_invoke_destroy(&sm);
        return 1;
    }
    bad |= expect_text("invoke id", seen.invoke_id, "h");
    bad |= expect_text("src", seen.src, "job://report");
    bad |= expect_text("content", seen.content, "payload");
    bad |= expect_text("param `job`", param_of(&seen, "job"), "8");
    bad |= expect_text("param `label`", param_of(&seen, "label"), "report");
    if (param_of(&seen, "_sce_deadline_ms") != NULL) {
        (void)fprintf(stderr, "FAIL: the engine's deadline param was handed to the host\n");
        bad = 1;
    }
    // The pairs as the data model holds them — a number stays a number — in the
    // one order every engine writes them in: ascending by name, though the
    // document declares `label` before `job`, and the saved state holds this very
    // text (ARCHITECTURE.md, "JSON Object Key Order").
    bad |= expect_text("event data", seen.event_data, "{\"_sce_deadline_ms\":5000,\"job\":8,\"label\":\"report\"}");
    statechart_static_host_invoke_destroy(&sm);
    return bad;
}

// The control: nothing has written `job`, so the field still holds what its
// `<data expr>` gave it. Keeps the case above from passing on a value that is
// simply always the new one.
static int a_param_read_before_any_bump_carries_the_declared_value(void) {
    static statechart_static_host_invoke_t sm;
    seen_t seen;
    memset(&seen, 0, sizeof(seen));
    boot(&sm, &seen);
    int bad = send_event(&sm, "start") ? 0 : 1;
    if (seen.starts != 1) {
        (void)fprintf(stderr, "FAIL: the host saw %d starts, want 1\n", seen.starts);
        statechart_static_host_invoke_destroy(&sm);
        return 1;
    }
    bad |= expect_text("param `job`", param_of(&seen, "job"), "7");
    bad |= expect_text("event data", seen.event_data, "{\"_sce_deadline_ms\":5000,\"job\":7,\"label\":\"report\"}");
    statechart_static_host_invoke_destroy(&sm);
    return bad;
}

// W3C SCXML 5.7.1: `boom` is a multiplication a 32-bit field cannot hold, so its
// pair is the evaluation that failed: reported (`errors` counts the
// `error.execution` the document took) and left out of the request, while the
// invocation still starts with `ok`. A pair written with the zero of the failed
// value is a request that still looks complete; one dropped in silence is a
// request that does too — the count tells them apart.
static int a_param_whose_value_cannot_be_computed_is_reported_and_left_out(void) {
    static static_host_invoke_overflow_t sm;
    seen_t seen;
    memset(&seen, 0, sizeof(seen));
    sce_host_invoker_registry_t wiring;
    memset(&wiring, 0, sizeof(wiring));
    (void)sce_host_invoker_register(&wiring, DECLARED_TYPE, invoker, &seen);
    static_host_invoke_overflow_init_with_host_invokers(&sm, &wiring);

    static_host_invoke_overflow_event_t event;
    int bad = 0;
    if (!static_host_invoke_overflow_resolve_event_by_name("start", &event)) {
        (void)fprintf(stderr, "FAIL: the machine has no event `start`\n");
        static_host_invoke_overflow_destroy(&sm);
        return 1;
    }
    static_host_invoke_overflow_event_with_meta_t meta;
    memset(&meta, 0, sizeof(meta));
    meta.event = event;
    static_host_invoke_overflow_raise_external(&sm, &meta);
    static_host_invoke_overflow_step(&sm);

    if (seen.starts != 1) {
        (void)fprintf(stderr, "FAIL: the invocation still starts: the host saw %d starts\n", seen.starts);
        static_host_invoke_overflow_destroy(&sm);
        return 1;
    }
    bad |= expect_text("param `ok`", param_of(&seen, "ok"), "4");
    bad |= expect_text("param `flag`", param_of(&seen, "flag"), "true");
    if (param_of(&seen, "boom") != NULL) {
        (void)fprintf(stderr, "FAIL: the failed pair `boom` is in the request's params\n");
        bad = 1;
    }
    // The pairs that crossed, ascending by name, and no `boom` among them.
    bad |= expect_text("event data", seen.event_data, "{\"flag\":true,\"ok\":4}");
    if (static_host_invoke_overflow_get_errors(&sm) != 1u) {
        (void)fprintf(stderr, "FAIL: errors is %u, want 1: the failed pair was not reported once\n",
                      (unsigned)static_host_invoke_overflow_get_errors(&sm));
        bad = 1;
    }
    static_host_invoke_overflow_destroy(&sm);
    return bad;
}

int main(void) {
    int bad = 0;
    bad |= a_param_carries_the_value_the_fields_hold_when_the_invocation_starts();
    bad |= a_param_read_before_any_bump_carries_the_declared_value();
    bad |= a_param_whose_value_cannot_be_computed_is_reported_and_left_out();
    if (bad != 0) {
        return 1;
    }
    (void)printf("static host invoke: ok\n");
    return 0;
}
