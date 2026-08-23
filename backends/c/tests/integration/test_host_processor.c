// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2.5 — C11 compile+run gate for a `<send type>` the HOST
// serves.
//
// The clause makes the Event I/O Processor identifier extensible, so the
// set is open by design. SCE implemented two of them and refused
// everything else with `error.execution`; a platform could not widen the
// set. Rust and C++ grew a registry first, and this backend refused the
// declaration by name until it grew one of its own — the refusal being
// honest is exactly what made the gap a coverage debt rather than a
// silent drop.
//
// Fixture: sce-build/tests/fixtures/host_processor/statechart_host_processor.scxml
// (the same document the Rust and C++ channels drive), generated WITH
// `--host-processor x-sce-host` by `backends/c/tests/CMakeLists.txt`. The
// declaration is load-bearing: without it codegen emits the refusal and
// every scenario below would measure the refusal instead of the feature.
//
// Eight scenarios drive that one machine, and the pair at the top is the
// whole contract:
//
//   * a registered handler receives the send and its reply arrives as an
//     event — the feature working;
//   * the same machine with nothing registered raises `error.execution` —
//     a wiring mistake staying visible instead of reading as success.
//
// A gate holding only the first would pass on an engine that dispatched
// to nothing and called it delivered, which is the silence being repaid.
//
// The last scenario has no sibling on the other engines and could not
// have one: their reply lists grow, so a report cannot fail to fit. This
// one is bounded — the MCU profile has no allocator — and a bounded list
// that quietly shortened a report would leave the machine somewhere the
// document never described.

#include <stdbool.h>
#include <stdio.h>
#include <string.h>

#include "statechart_host_processor_sm.h"

// The type the fixture was compiled for. `backends/c/tests/CMakeLists.txt`
// passes this same string to `--host-processor`; a test registering a
// different one would measure nothing and pass, which is why the
// `refused` counter is asserted rather than the registration trusted.
#define DECLARED_TYPE "x-sce-host"

// What the handler saw, so the request can be asserted rather than just
// its arrival.
typedef struct {
    int calls;
    char seen_type[64];
    char seen_event[64];
    char seen_within[64];
    bool saw_within;
    // A name the send did not carry must read as absent. Asked because a
    // lookup that ignored the name — answering the first param whatever was
    // asked for — is right for every send carrying exactly one, which is
    // every send in this fixture.
    bool absent_name_read_as_absent;
    bool had_send_id;
    // How many replies to report, and under what name. A test sets these
    // to shape the answer rather than needing a handler per scenario.
    int replies;
    const char *reply_name;
} recorder_t;

static void copy_field(char *dst, size_t cap, const char *src) {
    if (src == NULL) {
        dst[0] = '\0';
        return;
    }
    (void)snprintf(dst, cap, "%s", src);
}

static void recording_handler(void *user_data, const sce_host_send_request_t *request,
                              sce_host_send_response_list_t *out) {
    recorder_t *rec = (recorder_t *)user_data;
    rec->calls++;
    copy_field(rec->seen_type, sizeof(rec->seen_type), request->processor_type);
    copy_field(rec->seen_event, sizeof(rec->seen_event), request->event_name);
    const char *within = sce_host_send_param(request, "within");
    rec->saw_within = (within != NULL);
    copy_field(rec->seen_within, sizeof(rec->seen_within), within);
    rec->absent_name_read_as_absent = (sce_host_send_param(request, "not.a.param.this.send.carries") == NULL);
    rec->had_send_id = (request->send_id != NULL && request->send_id[0] != '\0');

    for (int i = 0; i < rec->replies; i++) {
        (void)sce_host_response_push(out, rec->reply_name, "");
    }
}

// The fixture's `<assign>`s are the only witness: every outcome leaves the
// machine in the same single state, so the configuration cannot tell them
// apart.
static int64_t counter(const statechart_host_processor_t *sm, const char *name) {
    int64_t value = -1;
    bool ok = false;
    if (strcmp(name, "served") == 0) {
        ok = statechart_host_processor_served(sm, &value);
    } else if (strcmp(name, "refused") == 0) {
        ok = statechart_host_processor_refused(sm, &value);
    } else {
        ok = statechart_host_processor_plain(sm, &value);
    }
    if (!ok) {
        (void)fprintf(stderr, "host_processor: FAIL - the fixture declares `%s` and the machine could not read it\n",
                      name);
        return -1;
    }
    return value;
}

static int check(const char *scenario, const char *what, int64_t got, int64_t want) {
    if (got == want) {
        return 0;
    }
    (void)fprintf(stderr, "host_processor: FAIL [%s] - %s is %lld, expected %lld\n", scenario, what, (long long)got,
                  (long long)want);
    return 1;
}

// Run the machine once with the wiring `setup` describes.
static void drive(statechart_host_processor_t *sm, recorder_t *rec, const char *register_as) {
    sce_host_processor_registry_t wiring;
    memset(&wiring, 0, sizeof(wiring));
    if (register_as != NULL) {
        (void)sce_host_registry_register(&wiring, register_as, recording_handler, rec);
    }
    statechart_host_processor_init_with_host_processors(sm, &wiring);
    statechart_host_processor_run(sm);
}

// The feature working: the act reaches the host and its answer reaches the
// document.
static int a_registered_handler_receives_the_send_and_its_reply_arrives(void) {
    recorder_t rec;
    memset(&rec, 0, sizeof(rec));
    rec.replies = 1;
    rec.reply_name = "turn.done";

    statechart_host_processor_t sm;
    drive(&sm, &rec, DECLARED_TYPE);

    int bad = 0;
    bad |= check("served", "served", counter(&sm, "served"), 1);
    bad |= check("served", "refused", counter(&sm, "refused"), 0);
    // The false-positive guard: an ordinary `<send>` in the same block must
    // still deliver. Without it a change that broke every send while leaving
    // the host branch intact would read as a pass.
    bad |= check("served", "plain", counter(&sm, "plain"), 1);
    bad |= check("served", "handler calls", rec.calls, 1);

    if (strcmp(rec.seen_type, DECLARED_TYPE) != 0) {
        (void)fprintf(stderr, "host_processor: FAIL [served] - handler saw type `%s`, expected `%s`\n", rec.seen_type,
                      DECLARED_TYPE);
        bad = 1;
    }
    if (strcmp(rec.seen_event, "watch.turn") != 0) {
        (void)fprintf(stderr, "host_processor: FAIL [served] - handler saw event `%s`, expected `watch.turn`\n",
                      rec.seen_event);
        bad = 1;
    }
    // The payload the author wrote has to survive the crossing, or the
    // document can name an act but not parameterise it — which is most of
    // the reason to move an act into the document at all.
    if (!rec.saw_within || strcmp(rec.seen_within, "2500") != 0) {
        (void)fprintf(stderr, "host_processor: FAIL [served] - <param name='within'> reached the handler as `%s`\n",
                      rec.saw_within ? rec.seen_within : "(absent)");
        bad = 1;
    }
    if (!rec.absent_name_read_as_absent) {
        (void)fprintf(stderr,
                      "host_processor: FAIL [served] - a <param> name the send does not carry read as present\n");
        bad = 1;
    }
    // Correlating a reply, or honouring a `<cancel>`, needs the send id —
    // auto-generated here because the fixture declares none.
    if (!rec.had_send_id) {
        (void)fprintf(stderr, "host_processor: FAIL [served] - the request carried no send id\n");
        bad = 1;
    }
    statechart_host_processor_destroy(&sm);
    return bad;
}

// The other half, and the one that keeps the repair honest: the build
// declared the type so codegen emitted a dispatch, but nothing was
// registered, so nobody performed the act.
static int a_declared_type_with_no_handler_still_raises_error_execution(void) {
    recorder_t rec;
    memset(&rec, 0, sizeof(rec));

    statechart_host_processor_t sm;
    drive(&sm, &rec, NULL);

    int bad = 0;
    bad |= check("unregistered", "refused", counter(&sm, "refused"), 1);
    bad |= check("unregistered", "served", counter(&sm, "served"), 0);
    bad |= check("unregistered", "plain", counter(&sm, "plain"), 1);
    bad |= check("unregistered", "handler calls", rec.calls, 0);
    statechart_host_processor_destroy(&sm);
    return bad;
}

// A handler may perform work and have nothing to say. That is not an
// error, and reporting it as one would cost every fire-and-forget act a
// spurious `error.execution`.
static int a_handler_that_answers_nothing_is_not_an_error(void) {
    recorder_t rec;
    memset(&rec, 0, sizeof(rec));
    rec.replies = 0;
    rec.reply_name = "turn.done";

    statechart_host_processor_t sm;
    drive(&sm, &rec, DECLARED_TYPE);

    int bad = 0;
    bad |= check("silent", "refused", counter(&sm, "refused"), 0);
    bad |= check("silent", "served", counter(&sm, "served"), 0);
    bad |= check("silent", "handler calls", rec.calls, 1);
    statechart_host_processor_destroy(&sm);
    return bad;
}

// The registry is keyed. A lookup falling back to "any handler" would
// deliver a document's acts to a processor it never named.
static int a_handler_registered_for_another_type_does_not_serve_this_one(void) {
    recorder_t rec;
    memset(&rec, 0, sizeof(rec));
    rec.replies = 1;
    rec.reply_name = "turn.done";

    statechart_host_processor_t sm;
    drive(&sm, &rec, "x-some-other-host");

    int bad = 0;
    bad |= check("other-type", "served", counter(&sm, "served"), 0);
    bad |= check("other-type", "refused", counter(&sm, "refused"), 1);
    bad |= check("other-type", "handler calls", rec.calls, 0);
    statechart_host_processor_destroy(&sm);
    return bad;
}

// The query the generated dispatch uses to tell "ran and said nothing"
// from "was never wired up", and the rule that keeps one type from having
// two servers.
static int the_registry_reports_what_it_holds(void) {
    // Registering a type twice must REPLACE. Appending would leave dispatch
    // depending on registration order, and a host re-registering during a
    // run means to change what serves the act — not to add a second server
    // whose turn may never come.
    recorder_t superseded;
    recorder_t current;
    memset(&superseded, 0, sizeof(superseded));
    memset(&current, 0, sizeof(current));
    current.replies = 1;
    current.reply_name = "turn.done";

    sce_host_processor_registry_t wiring;
    memset(&wiring, 0, sizeof(wiring));
    (void)sce_host_registry_register(&wiring, DECLARED_TYPE, recording_handler, &superseded);
    (void)sce_host_registry_register(&wiring, DECLARED_TYPE, recording_handler, &current);

    statechart_host_processor_t sm;
    statechart_host_processor_init_with_host_processors(&sm, &wiring);
    statechart_host_processor_run(&sm);

    int bad = 0;
    bad |= check("registry", "superseded handler calls", superseded.calls, 0);
    bad |= check("registry", "current handler calls", current.calls, 1);
    bad |= check("registry", "served", counter(&sm, "served"), 1);

    if (!statechart_host_processor_has_event_processor(&sm, DECLARED_TYPE)) {
        (void)fprintf(stderr, "host_processor: FAIL [registry] - the registered type reads as absent\n");
        bad = 1;
    }
    if (statechart_host_processor_has_event_processor(&sm, "x-never-registered")) {
        (void)fprintf(stderr, "host_processor: FAIL [registry] - an unregistered type reads as present\n");
        bad = 1;
    }
    // Re-wiring a machine that is already initialised is the other moment
    // the API serves, and it must be accepted rather than refused as a
    // duplicate.
    if (!statechart_host_processor_register_event_processor(&sm, DECLARED_TYPE, recording_handler, &current)) {
        (void)fprintf(stderr, "host_processor: FAIL [registry] - re-registering a served type was refused\n");
        bad = 1;
    }
    statechart_host_processor_destroy(&sm);
    return bad;
}

// A reply may name an event this machine does not declare — a host serving
// several documents, or one that has moved on since. That is dropped,
// exactly as any undeclared event reaching the queue is, and it is NOT an
// error: the act was performed, and what the machine does with an event it
// has no transition for is already settled.
//
// The scenario exists because nothing else here forces the name lookup to
// fail. Without it, a dispatch that raised whatever the enum's zero value
// happens to be would pass every other scenario in this file.
static int a_reply_naming_an_undeclared_event_is_dropped(void) {
    recorder_t rec;
    memset(&rec, 0, sizeof(rec));
    rec.replies = 1;
    rec.reply_name = "turn.never.declared";

    statechart_host_processor_t sm;
    drive(&sm, &rec, DECLARED_TYPE);

    int bad = 0;
    bad |= check("undeclared-reply", "handler calls", rec.calls, 1);
    bad |= check("undeclared-reply", "served", counter(&sm, "served"), 0);
    bad |= check("undeclared-reply", "refused", counter(&sm, "refused"), 0);
    // The ordinary send in the same block still delivered, so the machine
    // kept running rather than being derailed by the unknown name.
    bad |= check("undeclared-reply", "plain", counter(&sm, "plain"), 1);
    statechart_host_processor_destroy(&sm);
    return bad;
}

// C11-only, and the reason is the profile rather than the clause: this
// backend's reply list is a fixed-capacity array because an MCU has no
// allocator, so a handler CAN report more than the build reserved room
// for. Dropping the surplus quietly would leave the document mid-act with
// no trace; the list says it overflowed and the dispatch raises
// `error.execution`.
//
// The assertion has two halves on purpose. The events that DID fit must
// still arrive — an overflow is not a reason to discard the whole report
// — and the refusal must be raised exactly once.
static int a_report_that_does_not_fit_is_reported_rather_than_shortened(void) {
    recorder_t rec;
    memset(&rec, 0, sizeof(rec));
    rec.replies = (int)SCE_MAX_HOST_RESPONSES + 1;
    rec.reply_name = "turn.done";

    statechart_host_processor_t sm;
    drive(&sm, &rec, DECLARED_TYPE);

    int bad = 0;
    bad |= check("overflow", "served", counter(&sm, "served"), (int64_t)SCE_MAX_HOST_RESPONSES);
    bad |= check("overflow", "refused", counter(&sm, "refused"), 1);
    statechart_host_processor_destroy(&sm);
    return bad;
}

// The other way a bounded report fails to fit: one entry too large for its
// slot. The whole entry is refused rather than shortened, and for the same
// reason the count arm refuses — a truncated payload is one that parses to
// something the host never said, and a truncated event NAME can match a
// DIFFERENT transition, making the machine take a step nobody asked for.
//
// Its own scenario because the count arm above cannot reach this branch:
// one reply that does not fit overflows a list that had room for four.
typedef struct {
    const char *data;
    int calls;
    bool push_accepted;
} oversize_ctx_t;

static void oversize_handler(void *user_data, const sce_host_send_request_t *request,
                             sce_host_send_response_list_t *out) {
    oversize_ctx_t *ctx = (oversize_ctx_t *)user_data;
    (void)request;
    ctx->calls++;
    ctx->push_accepted = sce_host_response_push(out, "turn.done", ctx->data);
}

static int an_entry_too_large_for_its_slot_is_refused(void) {
    static char oversized[SCE_MAX_DATA_LEN + 64];
    memset(oversized, 'x', sizeof(oversized) - 1u);
    oversized[sizeof(oversized) - 1u] = '\0';

    oversize_ctx_t ctx;
    memset(&ctx, 0, sizeof(ctx));
    ctx.data = oversized;

    sce_host_processor_registry_t wiring;
    memset(&wiring, 0, sizeof(wiring));
    (void)sce_host_registry_register(&wiring, DECLARED_TYPE, oversize_handler, &ctx);

    statechart_host_processor_t sm;
    statechart_host_processor_init_with_host_processors(&sm, &wiring);
    statechart_host_processor_run(&sm);

    int bad = 0;
    bad |= check("oversize", "handler calls", ctx.calls, 1);
    if (ctx.push_accepted) {
        (void)fprintf(stderr, "host_processor: FAIL [oversize] - a payload larger than the slot was accepted\n");
        bad = 1;
    }
    bad |= check("oversize", "served", counter(&sm, "served"), 0);
    bad |= check("oversize", "refused", counter(&sm, "refused"), 1);
    statechart_host_processor_destroy(&sm);
    return bad;
}

int main(void) {
    int bad = 0;
    bad |= a_registered_handler_receives_the_send_and_its_reply_arrives();
    bad |= a_declared_type_with_no_handler_still_raises_error_execution();
    bad |= a_handler_that_answers_nothing_is_not_an_error();
    bad |= a_handler_registered_for_another_type_does_not_serve_this_one();
    bad |= the_registry_reports_what_it_holds();
    bad |= a_reply_naming_an_undeclared_event_is_dropped();
    bad |= a_report_that_does_not_fit_is_reported_rather_than_shortened();
    bad |= an_entry_too_large_for_its_slot_is_refused();

    if (bad != 0) {
        (void)fprintf(stderr, "host_processor: FAIL - see the scenario(s) named above\n");
        return 1;
    }
    (void)printf("host_processor: PASS - 8 scenarios\n");
    return 0;
}
