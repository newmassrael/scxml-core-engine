// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.1 — C11 compile+run gate for an `<invoke type>` the HOST
// runs.
//
// The C11 machine carried the invoker registry and the template lowered the
// start and the cancel, but no C11 test ran either, so this backend held the
// lifecycle on trust. This is the channel the C++, Rust, Go, Python and Kotlin
// gates already are.
//
// Fixture: sce-build/tests/fixtures/host_processor/statechart_host_invoker.scxml
// (the same document those channels drive), generated WITH
// `--host-invoker x-sce-host` by `backends/c/tests/CMakeLists.txt`. The
// declaration is load-bearing: without it codegen emits the refusal and every
// scenario below would measure the refusal instead of the feature.
//
// An invoke is not a send: it has a LIFETIME. The scenarios hold the outcomes
// apart, because the configuration alone cannot:
//
//   * a registered invoker is STARTED with what the document wrote, each
//     invocation as itself, and its completion names the invocation;
//   * leaving the state CANCELS each one, once;
//   * a cancel is delivered only for an invocation that started;
//   * a declared type with nothing registered raises `error.execution`;
//   * an invoker registered for another type does not run this one.

#include <stdbool.h>
#include <stdio.h>
#include <string.h>

#include "statechart_host_invoker_sm.h"

// The type the fixture was compiled for. `backends/c/tests/CMakeLists.txt`
// passes this same string to `--host-invoker`.
#define DECLARED_TYPE "x-sce-host"

// What the invoker saw, in call order.
typedef struct {
    int calls;
    char log[8][128];
} recorder_t;

static void record(recorder_t *rec, const char *line) {
    if (rec->calls < (int)(sizeof(rec->log) / sizeof(rec->log[0]))) {
        (void)snprintf(rec->log[rec->calls], sizeof(rec->log[0]), "%s", line);
    }
    rec->calls++;
}

static const char *param(const sce_host_invoke_event_t *event, const char *name) {
    for (int i = 0; i < event->param_count; i++) {
        if (strcmp(event->params[i].name, name) == 0) {
            return event->params[i].value;
        }
    }
    return NULL;
}

// A recording invoker. Answers a completion on start so the `done.invoke` path
// is exercised too.
static void recording_invoker(void *user_data, const sce_host_invoke_event_t *event, sce_host_invoke_response_t *out) {
    recorder_t *rec = (recorder_t *)user_data;
    char line[128];
    if (event->phase == SCE_HOST_INVOKE_START) {
        const char *within = param(event, "within");
        (void)snprintf(line, sizeof(line), "START id=%s type=%s src=%s within=%s", event->invoke_id,
                       event->processor_type, event->src, within != NULL ? within : "absent");
        record(rec, line);
        out->has_done_data = true;
        (void)snprintf(out->done_data, sizeof(out->done_data), "%s", "ok");
    } else {
        (void)snprintf(line, sizeof(line), "CANCEL id=%s", event->invoke_id);
        record(rec, line);
    }
}

// The fixture's `<assign>`s are the only witness.
static int64_t counter(const statechart_host_invoker_t *sm, const char *name) {
    int64_t value = -1;
    bool ok = false;
    if (strcmp(name, "started") == 0) {
        ok = statechart_host_invoker_started(sm, &value);
    } else if (strcmp(name, "started2") == 0) {
        ok = statechart_host_invoker_started2(sm, &value);
    } else if (strcmp(name, "refused") == 0) {
        ok = statechart_host_invoker_refused(sm, &value);
    } else if (strcmp(name, "ended") == 0) {
        ok = statechart_host_invoker_ended(sm, &value);
    } else if (strcmp(name, "entered") == 0) {
        ok = statechart_host_invoker_entered(sm, &value);
    } else if (strcmp(name, "dropped") == 0) {
        ok = statechart_host_invoker_dropped(sm, &value);
    }
    if (!ok) {
        (void)fprintf(stderr, "host_invoker: FAIL - the fixture declares `%s` and the machine could not read it\n",
                      name);
        return -1;
    }
    return value;
}

static int check(const char *scenario, const char *what, int64_t got, int64_t want) {
    if (got == want) {
        return 0;
    }
    (void)fprintf(stderr, "host_invoker: FAIL [%s] - %s is %lld, expected %lld\n", scenario, what, (long long)got,
                  (long long)want);
    return 1;
}

static int check_line(const char *scenario, const recorder_t *rec, int index, const char *want) {
    if (index < rec->calls && strcmp(rec->log[index], want) == 0) {
        return 0;
    }
    (void)fprintf(stderr, "host_invoker: FAIL [%s] - call %d was `%s`, expected `%s`\n", scenario, index,
                  index < rec->calls ? rec->log[index] : "(none)", want);
    return 1;
}

static int count_lines(const recorder_t *rec, const char *want) {
    int n = 0;
    for (int i = 0; i < rec->calls && i < (int)(sizeof(rec->log) / sizeof(rec->log[0])); i++) {
        n += strcmp(rec->log[i], want) == 0 ? 1 : 0;
    }
    return n;
}

// Initialise with the invokers `register_as` names (NULL registers nothing).
// Through `_init_with_host_invokers` rather than `_register_invoker` after
// `_init`, because the invocations of the initial configuration run inside
// `_init` (§scxml-6.4 runs them at the end of the macrostep that entered their
// state, and that macrostep is `_init`'s own).
static void boot(statechart_host_invoker_t *sm, recorder_t *rec, const char *register_as) {
    sce_host_invoker_registry_t wiring;
    memset(&wiring, 0, sizeof(wiring));
    if (register_as != NULL) {
        (void)sce_host_invoker_register(&wiring, register_as, recording_invoker, rec);
    }
    statechart_host_invoker_init_with_host_invokers(sm, &wiring);
}

static int a_registered_invoker_is_started_with_what_the_document_wrote(void) {
    recorder_t rec;
    memset(&rec, 0, sizeof(rec));
    statechart_host_invoker_t sm;
    boot(&sm, &rec, DECLARED_TYPE);

    int bad = 0;
    // The fixture counts a completion only when its `_event.invokeid` names the
    // invocation (W3C SCXML 5.10.1), so each counter is that assertion too.
    bad |= check("started", "started", counter(&sm, "started"), 1);
    bad |= check("started", "started2", counter(&sm, "started2"), 1);
    bad |= check("started", "refused", counter(&sm, "refused"), 0);
    bad |= check("started", "entered", counter(&sm, "entered"), 1);
    bad |= check("started", "invoker calls", rec.calls, 2);
    // Each invocation is started as itself: `probe2` begins with `probe`.
    bad |= check_line("started", &rec, 0, "START id=probe type=" DECLARED_TYPE " src=pane://turn within=2500");
    bad |= check_line("started", &rec, 1, "START id=probe2 type=" DECLARED_TYPE " src=pane://other within=absent");
    statechart_host_invoker_destroy(&sm);
    return bad;
}

// A request-recording invoker: one line per start, carrying every field the
// request holds and every param pair in the order the array gives them.
static void request_invoker(void *user_data, const sce_host_invoke_event_t *event, sce_host_invoke_response_t *out) {
    (void)out;
    recorder_t *rec = (recorder_t *)user_data;
    if (event->phase != SCE_HOST_INVOKE_START) {
        return;
    }
    char line[128];
    int used = snprintf(line, sizeof(line), "START id=%s src=%s content=%s params=", event->invoke_id, event->src,
                        event->content);
    for (int i = 0; i < event->param_count && used > 0 && (size_t)used < sizeof(line); i++) {
        used += snprintf(line + used, sizeof(line) - (size_t)used, "%s%s=%s", i > 0 ? "," : "", event->params[i].name,
                         event->params[i].value);
    }
    record(rec, line);
}

// W3C SCXML 6.4.1: `srcexpr`, `namelist`, `<param expr>` and `<content expr>`
// are read from the data model when the invocation starts. The request used
// to carry the literal params alone, so a document that computed what to
// invoke handed the host an empty description.
static int what_the_request_says_is_evaluated_when_the_invocation_starts(void) {
    recorder_t rec;
    memset(&rec, 0, sizeof(rec));
    statechart_host_invoker_t sm;
    sce_host_invoker_registry_t wiring;
    memset(&wiring, 0, sizeof(wiring));
    (void)sce_host_invoker_register(&wiring, DECLARED_TYPE, request_invoker, &rec);
    statechart_host_invoker_init_with_host_invokers(&sm, &wiring);
    memset(&rec, 0, sizeof(rec));  // `probe` / `probe2`, which the case above already reads
    statechart_host_invoker_event_with_meta_t evaluate;
    memset(&evaluate, 0, sizeof(evaluate));
    evaluate.event = STATECHART_HOST_INVOKER_EVENT_EVALUATE;
    statechart_host_invoker_raise_external(&sm, &evaluate);
    statechart_host_invoker_step(&sm);

    int bad = 0;
    // `req3`'s srcexpr cannot be evaluated, so it is never started.
    bad |= check("evaluate", "invoker calls", rec.calls, 2);
    // Namelist entries first, then each `<param>` in document order: a
    // repeated name is a second pair, and the one that failed is absent
    // (W3C SCXML 5.7.1) while the invocation still started.
    bad |= check_line("evaluate", &rec, 0, "START id=req src=pane://dyn content= params=n=7,twice=a,twice=8");
    bad |= check_line("evaluate", &rec, 1, "START id=req2 src= content=body:7 params=");
    // One error.execution for the dropped `<param>`, one for `req3`.
    bad |= check("evaluate", "dropped", counter(&sm, "dropped"), 2);
    statechart_host_invoker_destroy(&sm);
    return bad;
}

// The invocation ends with the state that started it.
static int leaving_the_state_cancels_the_invocation(void) {
    recorder_t rec;
    memset(&rec, 0, sizeof(rec));
    statechart_host_invoker_t sm;
    boot(&sm, &rec, DECLARED_TYPE);
    statechart_host_invoker_event_with_meta_t leave;
    memset(&leave, 0, sizeof(leave));
    leave.event = STATECHART_HOST_INVOKER_EVENT_LEAVE;
    statechart_host_invoker_raise_external(&sm, &leave);
    statechart_host_invoker_step(&sm);

    int bad = 0;
    bad |= check("leave", "ended", counter(&sm, "ended"), 1);
    bad |= check("leave", "cancels of probe", count_lines(&rec, "CANCEL id=probe"), 1);
    bad |= check("leave", "cancels of probe2", count_lines(&rec, "CANCEL id=probe2"), 1);
    statechart_host_invoker_destroy(&sm);
    return bad;
}

// A cancel is delivered once, and only for an invocation that started. Asked
// at the machine's surface, because driving it cannot produce "never started"
// for a registered invoker.
//
// The first half doubles as the contract `_init_with_host_invokers` exists
// for: an invoker registered after `_init` is too late for the initial
// configuration — those invocations were refused inside `_init` — so it is
// neither started nor cancelled.
static int cancel_is_not_delivered_for_an_invocation_that_never_started(void) {
    recorder_t late;
    memset(&late, 0, sizeof(late));
    statechart_host_invoker_t sm;
    statechart_host_invoker_init(&sm);
    (void)statechart_host_invoker_register_invoker(&sm, DECLARED_TYPE, recording_invoker, &late);

    int bad = 0;
    bad |= check("never-started", "refused before the registration", counter(&sm, "refused"), 2);
    if (statechart_host_invoker_cancel_host_invoke(&sm, DECLARED_TYPE, "probe")) {
        (void)fprintf(stderr, "host_invoker: FAIL [never-started] - a cancel was reported for a refused invocation\n");
        bad = 1;
    }
    bad |= check("never-started", "calls to the late invoker", late.calls, 0);
    statechart_host_invoker_destroy(&sm);

    // Now one that started: cancelled once, and the second cancel has nothing
    // left to do.
    recorder_t rec;
    memset(&rec, 0, sizeof(rec));
    boot(&sm, &rec, DECLARED_TYPE);
    if (!statechart_host_invoker_cancel_host_invoke(&sm, DECLARED_TYPE, "probe")) {
        (void)fprintf(stderr, "host_invoker: FAIL [never-started] - a started invocation reported nothing to cancel\n");
        bad = 1;
    }
    if (statechart_host_invoker_cancel_host_invoke(&sm, DECLARED_TYPE, "probe")) {
        (void)fprintf(stderr, "host_invoker: FAIL [never-started] - the same invocation was cancelled twice\n");
        bad = 1;
    }
    bad |= check("never-started", "cancels of probe", count_lines(&rec, "CANCEL id=probe"), 1);
    statechart_host_invoker_destroy(&sm);
    return bad;
}

// The build declared the type, so codegen emitted a start — but nothing was
// registered, so no process ran. One error.execution per invocation.
static int a_declared_type_with_no_invoker_still_raises_error_execution(void) {
    recorder_t rec;
    memset(&rec, 0, sizeof(rec));
    statechart_host_invoker_t sm;
    boot(&sm, &rec, NULL);

    int bad = 0;
    bad |= check("unregistered", "refused", counter(&sm, "refused"), 2);
    bad |= check("unregistered", "started", counter(&sm, "started"), 0);
    bad |= check("unregistered", "invoker calls", rec.calls, 0);
    statechart_host_invoker_destroy(&sm);
    return bad;
}

// The registry is keyed.
static int an_invoker_registered_for_another_type_does_not_run_this_one(void) {
    recorder_t rec;
    memset(&rec, 0, sizeof(rec));
    statechart_host_invoker_t sm;
    boot(&sm, &rec, "x-some-other-host");

    int bad = 0;
    bad |= check("other-type", "started", counter(&sm, "started"), 0);
    bad |= check("other-type", "refused", counter(&sm, "refused"), 2);
    bad |= check("other-type", "invoker calls", rec.calls, 0);
    statechart_host_invoker_destroy(&sm);
    return bad;
}

int main(void) {
    int bad = 0;
    bad |= a_registered_invoker_is_started_with_what_the_document_wrote();
    bad |= what_the_request_says_is_evaluated_when_the_invocation_starts();
    bad |= leaving_the_state_cancels_the_invocation();
    bad |= cancel_is_not_delivered_for_an_invocation_that_never_started();
    bad |= a_declared_type_with_no_invoker_still_raises_error_execution();
    bad |= an_invoker_registered_for_another_type_does_not_run_this_one();

    if (bad != 0) {
        (void)fprintf(stderr, "host_invoker: FAIL - see the scenario(s) named above\n");
        return 1;
    }
    (void)printf("host_invoker: PASS - 5 scenarios\n");
    return 0;
}
