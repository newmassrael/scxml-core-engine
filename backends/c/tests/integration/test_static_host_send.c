// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15 — a `<send>` the HOST serves, in a
// `datamodel="sce-static"` machine, carries typed `<param>`s read from the
// machine's own fields when the send is made, and waits for its `delay` with the
// request it was made with. C11 compile+run gate.
//
// The machine is built with NO script engine: its variables are fields, and a
// `<param>` that needed an engine to be read would not have one to ask. Linked
// WITHOUT `sce_c_scripting` and `lua54`, so the link is the proof.
//
// Fixture: sce-build/tests/fixtures/host_processor/statechart_static_delayed_host_send.scxml
// (the document the Rust, Kotlin and Go channels save and restore), generated WITH
// `--host-processor x-sce-host` by `backends/c/tests/CMakeLists.txt`. `start` arms
// a send 500 ms ahead with `job` 7 and `label` "report", carrying the id `a`; `bump`
// then makes `job` 8, and `cancel` drops the waiting send.
//
// What this holds, on a clock the test owns:
//
//   * the request is the one the send was made with — `job` 7, not the 8 the field
//     holds when the wait ends — handed to the host at 500 ms and not before;
//   * it crosses as the text of the request's `params` and as the JSON object of
//     its `event_data`, whose members are written in the one order every engine
//     writes them in (ascending by name, ARCHITECTURE.md, "JSON Object Key Order")
//     although the document declares `label` before `job`, so a host that compares
//     the request as bytes sees the same text from any backend;
//   * a `<cancel>` of the id removes the wait, and the host is never called.
//
// A second document, `integration_resources/static_host_send_content/` (C11 alone),
// holds a `<send>`'s literal `<content>`: the text it spells, finished at build time,
// as the request's `event_data` for a send the host serves and as the event a send to
// the machine itself delivers.

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "statechart_static_delayed_host_send_sm.h"
#include "static_host_send_content_sm.h"

#define DECLARED_TYPE "x-sce-host"

typedef statechart_static_delayed_host_send_t sm_t;

// What the host saw of the sends it was handed.
typedef struct {
    int calls;
    char event[32];
    char target[32];
    char send_id[16];
    char event_data[128];
    char content[64];
    char label[32];
    char job[16];
    int param_count;
} seen_t;

static void handler(void *user_data, const sce_host_send_request_t *request, sce_host_send_response_list_t *out) {
    seen_t *seen = (seen_t *)user_data;
    (void)out;
    seen->calls++;
    (void)snprintf(seen->event, sizeof(seen->event), "%s", request->event_name);
    (void)snprintf(seen->target, sizeof(seen->target), "%s", request->target);
    (void)snprintf(seen->send_id, sizeof(seen->send_id), "%s", request->send_id);
    (void)snprintf(seen->event_data, sizeof(seen->event_data), "%s", request->event_data);
    (void)snprintf(seen->content, sizeof(seen->content), "%s",
                   request->content != NULL ? request->content : "(absent)");
    const char *label = sce_host_send_param(request, "label");
    const char *job = sce_host_send_param(request, "job");
    (void)snprintf(seen->label, sizeof(seen->label), "%s", label != NULL ? label : "(absent)");
    (void)snprintf(seen->job, sizeof(seen->job), "%s", job != NULL ? job : "(absent)");
    seen->param_count = request->param_count;
}

static void boot(sm_t *sm, seen_t *seen) {
    sce_host_processor_registry_t wiring;
    memset(&wiring, 0, sizeof(wiring));
    (void)sce_host_registry_register(&wiring, DECLARED_TYPE, handler, seen);
    statechart_static_delayed_host_send_init_with_clock_and_host_processors(sm, sce_clock_manual(0u), &wiring);
}

static void send_event(sm_t *sm, const char *name) {
    statechart_static_delayed_host_send_event_t event;
    if (!statechart_static_delayed_host_send_resolve_event_by_name(name, &event)) {
        (void)fprintf(stderr, "FAIL: the machine has no event `%s`\n", name);
        return;
    }
    statechart_static_delayed_host_send_event_with_meta_t meta;
    memset(&meta, 0, sizeof(meta));
    meta.event = event;
    statechart_static_delayed_host_send_raise_external(sm, &meta);
    statechart_static_delayed_host_send_run(sm);
}

static void advance(sm_t *sm, uint64_t ms) {
    statechart_static_delayed_host_send_advance_time_ms(sm, ms);
    statechart_static_delayed_host_send_run(sm);
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

// The wait ends at 500 ms, and the request is the one made at 0: `bump` raised
// `job` to 8 in between, and a request evaluated again when the wait ends would
// say 8.
static int the_send_waits_with_the_request_it_was_made_with(void) {
    static sm_t sm;
    seen_t seen;
    memset(&seen, 0, sizeof(seen));
    boot(&sm, &seen);
    send_event(&sm, "start");
    send_event(&sm, "bump");
    advance(&sm, 499u);
    int bad = expect_int("host calls a millisecond before the wait ends", seen.calls, 0);
    advance(&sm, 1u);
    bad |= expect_int("host calls when the wait ends", seen.calls, 1);
    if (seen.calls == 1) {
        bad |= expect_text("event", seen.event, "audit");
        bad |= expect_text("target", seen.target, "job://report");
        bad |= expect_text("send id", seen.send_id, "a");
        bad |= expect_text("param `job` (made at 7, not the 8 the field holds now)", seen.job, "7");
        bad |= expect_text("param `label`", seen.label, "report");
        bad |= expect_int("params carried", seen.param_count, 2);
        // The one text every engine writes: members ascending by name, though the
        // document declares `label` first.
        bad |= expect_text("event data", seen.event_data, "{\"job\":7,\"label\":\"report\"}");
    }
    statechart_static_delayed_host_send_destroy(&sm);
    return bad;
}

// The wait is dropped by a `<cancel>` of its id, and the host is never asked.
static int a_cancelled_send_never_reaches_the_host(void) {
    static sm_t sm;
    seen_t seen;
    memset(&seen, 0, sizeof(seen));
    boot(&sm, &seen);
    send_event(&sm, "start");
    advance(&sm, 200u);
    send_event(&sm, "cancel");
    advance(&sm, 10000u);
    int bad = expect_int("host calls after the cancel", seen.calls, 0);
    statechart_static_delayed_host_send_destroy(&sm);
    return bad;
}

// A literal `<content>` is the text it spells (SCE Accepted Subset §2.15), with no
// script engine to read it: the host's request carries the JSON string the
// whitespace-normalised text makes, and the text as written as its `content`; a send
// to the machine itself delivers the event, which the machine counts.
static int a_literal_content_is_the_text_it_spells(void) {
    static static_host_send_content_t sm;
    seen_t seen;
    memset(&seen, 0, sizeof(seen));
    sce_host_processor_registry_t wiring;
    memset(&wiring, 0, sizeof(wiring));
    (void)sce_host_registry_register(&wiring, DECLARED_TYPE, handler, &seen);
    static_host_send_content_init_with_host_processors(&sm, &wiring);
    static_host_send_content_event_t go;
    if (!static_host_send_content_resolve_event_by_name("go", &go)) {
        (void)fprintf(stderr, "FAIL: the content machine has no event `go`\n");
        static_host_send_content_destroy(&sm);
        return 1;
    }
    static_host_send_content_event_with_meta_t meta;
    memset(&meta, 0, sizeof(meta));
    meta.event = go;
    static_host_send_content_raise_external(&sm, &meta);
    static_host_send_content_run(&sm);
    int bad = expect_int("host calls", seen.calls, 1);
    if (seen.calls == 1) {
        bad |= expect_text("event", seen.event, "note");
        bad |= expect_text("target", seen.target, "job://report");
        bad |= expect_text("event data (the text, normalised, as a JSON string)", seen.event_data, "\"two words\"");
        bad |= expect_text("content (the text as written)", seen.content, "two   words");
    }
    bad |= expect_int("events the machine sent itself and counted", (long)static_host_send_content_get_heard(&sm), 1);
    static_host_send_content_destroy(&sm);
    return bad;
}

int main(void) {
    int bad = 0;
    bad |= the_send_waits_with_the_request_it_was_made_with();
    bad |= a_cancelled_send_never_reaches_the_host();
    bad |= a_literal_content_is_the_text_it_spells();
    if (bad != 0) {
        return 1;
    }
    (void)printf("static host send: ok\n");
    return 0;
}
