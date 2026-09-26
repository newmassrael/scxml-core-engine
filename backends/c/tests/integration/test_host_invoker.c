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
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "statechart_host_invoker_sm.h"

#ifndef SCE_HOST_INVOKE_DEADLINE_TABLE
#error "SCE_HOST_INVOKE_DEADLINE_TABLE must name the shared deadline table (backends/c/tests/CMakeLists.txt)"
#endif

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
    } else if (strcmp(name, "matched") == 0) {
        ok = statechart_host_invoker_matched(sm, &value);
    } else if (strcmp(name, "slotted") == 0) {
        ok = statechart_host_invoker_slotted(sm, &value);
    } else if (strcmp(name, "pinged") == 0) {
        ok = statechart_host_invoker_pinged(sm, &value);
    } else if (strcmp(name, "leaked") == 0) {
        ok = statechart_host_invoker_leaked(sm, &value);
    } else if (strcmp(name, "lost") == 0) {
        ok = statechart_host_invoker_lost(sm, &value);
    } else if (strcmp(name, "expired") == 0) {
        ok = statechart_host_invoker_expired(sm, &value);
    } else if (strcmp(name, "finished") == 0) {
        ok = statechart_host_invoker_finished(sm, &value);
    } else if (strcmp(name, "misdated") == 0) {
        ok = statechart_host_invoker_misdated(sm, &value);
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

// An invoker whose work outlives the call: it answers nothing on a start, so
// each invocation stays running until the test completes or cancels it. It
// records the lines `recording_invoker` does, and every start's token for a
// later `_complete_host_invoke`.
typedef struct {
    recorder_t rec;
    char ids[8][32];
    uint64_t tokens[8];
    int starts;
} running_t;

static void running_invoker(void *user_data, const sce_host_invoke_event_t *event, sce_host_invoke_response_t *out) {
    (void)out;
    running_t *run = (running_t *)user_data;
    char line[128];
    if (event->phase == SCE_HOST_INVOKE_START) {
        (void)snprintf(line, sizeof(line), "START id=%s", event->invoke_id);
        if (run->starts < (int)(sizeof(run->tokens) / sizeof(run->tokens[0]))) {
            (void)snprintf(run->ids[run->starts], sizeof(run->ids[0]), "%s", event->invoke_id);
            run->tokens[run->starts] = event->token;
        }
        run->starts++;
    } else {
        (void)snprintf(line, sizeof(line), "CANCEL id=%s", event->invoke_id);
    }
    record(&run->rec, line);
}

// The token of the latest start of `invoke_id`.
static uint64_t token_of(const running_t *run, const char *invoke_id) {
    for (int i = run->starts - 1; i >= 0; i--) {
        if (i < (int)(sizeof(run->tokens) / sizeof(run->tokens[0])) && strcmp(run->ids[i], invoke_id) == 0) {
            return run->tokens[i];
        }
    }
    (void)fprintf(stderr, "host_invoker: FAIL - `%s` never started\n", invoke_id);
    return UINT64_MAX;
}

static void boot_running(statechart_host_invoker_t *sm, running_t *run) {
    sce_host_invoker_registry_t wiring;
    memset(run, 0, sizeof(*run));
    memset(&wiring, 0, sizeof(wiring));
    (void)sce_host_invoker_register(&wiring, DECLARED_TYPE, running_invoker, run);
    statechart_host_invoker_init_with_host_invokers(sm, &wiring);
}

// This backend's delivery pair: enqueue, then run a macrostep.
static void deliver(statechart_host_invoker_t *sm, statechart_host_invoker_event_t event) {
    statechart_host_invoker_event_with_meta_t meta;
    memset(&meta, 0, sizeof(meta));
    meta.event = event;
    statechart_host_invoker_raise_external(sm, &meta);
    statechart_host_invoker_step(sm);
}

static int expect(const char *scenario, const char *what, bool ok) {
    if (ok) {
        return 0;
    }
    (void)fprintf(stderr, "host_invoker: FAIL [%s] - %s\n", scenario, what);
    return 1;
}

// W3C SCXML 6.4: `done.invoke` says the invoked process is over, so leaving the
// state afterwards has nothing to stop. Both invocations here complete
// synchronously; neither is cancelled.
static int a_completed_invocation_is_not_cancelled(void) {
    recorder_t rec;
    memset(&rec, 0, sizeof(rec));
    statechart_host_invoker_t sm;
    boot(&sm, &rec, DECLARED_TYPE);
    deliver(&sm, STATECHART_HOST_INVOKER_EVENT_LEAVE);

    int bad = 0;
    bad |= check("completed", "started", counter(&sm, "started"), 1);
    bad |=
        check("completed", "cancels", count_lines(&rec, "CANCEL id=probe") + count_lines(&rec, "CANCEL id=probe2"), 0);
    statechart_host_invoker_destroy(&sm);
    return bad;
}

// A host that finishes later reports it with the start's token, and the
// completion is taken once: a second report of the same run finds nothing, and
// the state's exit then cancels only the invocation still running.
static int a_late_completion_is_accepted_exactly_once(void) {
    running_t run;
    statechart_host_invoker_t sm;
    boot_running(&sm, &run);

    int bad = 0;
    bad |= check("late", "started before any completion", counter(&sm, "started"), 0);
    const uint64_t token = token_of(&run, "probe");
    bad |= expect("late", "a running invocation's completion was refused",
                  statechart_host_invoker_complete_host_invoke(&sm, DECLARED_TYPE, "probe", token, "ok"));
    statechart_host_invoker_step(&sm);
    // The fixture counts it only when `_event.invokeid` names the invocation
    // (W3C SCXML 5.10.1), so this is that assertion too.
    bad |= check("late", "started", counter(&sm, "started"), 1);
    bad |= expect("late", "the same run completed twice",
                  !statechart_host_invoker_complete_host_invoke(&sm, DECLARED_TYPE, "probe", token, "again"));
    statechart_host_invoker_step(&sm);
    bad |= check("late", "started after a second completion", counter(&sm, "started"), 1);

    deliver(&sm, STATECHART_HOST_INVOKER_EVENT_LEAVE);
    bad |= check("late", "cancels of probe", count_lines(&run.rec, "CANCEL id=probe"), 0);
    bad |= check("late", "cancels of probe2", count_lines(&run.rec, "CANCEL id=probe2"), 1);
    statechart_host_invoker_destroy(&sm);
    return bad;
}

// W3C SCXML 6.4: once the state has exited, what the cancelled process sends is
// ignored. The host's reply arrives after the cancel and is refused.
static int a_completion_after_the_cancel_is_refused(void) {
    running_t run;
    statechart_host_invoker_t sm;
    boot_running(&sm, &run);
    const uint64_t token = token_of(&run, "probe");
    deliver(&sm, STATECHART_HOST_INVOKER_EVENT_LEAVE);

    int bad = 0;
    bad |= expect("after-cancel", "a cancelled run's completion was accepted",
                  !statechart_host_invoker_complete_host_invoke(&sm, DECLARED_TYPE, "probe", token, "late"));
    statechart_host_invoker_step(&sm);
    bad |= check("after-cancel", "started", counter(&sm, "started"), 0);
    statechart_host_invoker_destroy(&sm);
    return bad;
}

// Re-entering the state starts the same `<invoke>` again under the same id.
// The first run's late reply carries the first start's token and is refused;
// the second run's is accepted.
static int a_restarted_invoke_refuses_the_first_runs_reply(void) {
    running_t run;
    statechart_host_invoker_t sm;
    boot_running(&sm, &run);
    const uint64_t first = token_of(&run, "probe");
    deliver(&sm, STATECHART_HOST_INVOKER_EVENT_LEAVE);
    deliver(&sm, STATECHART_HOST_INVOKER_EVENT_AGAIN);
    const uint64_t second = token_of(&run, "probe");

    int bad = 0;
    bad |= expect("restart", "a restart reused the first start's token", first != second);
    bad |= expect("restart", "the first run's reply was taken for the second run's",
                  !statechart_host_invoker_complete_host_invoke(&sm, DECLARED_TYPE, "probe", first, "stale"));
    statechart_host_invoker_step(&sm);
    bad |= check("restart", "started after a stale reply", counter(&sm, "started"), 0);
    bad |= expect("restart", "the second run's completion was refused",
                  statechart_host_invoker_complete_host_invoke(&sm, DECLARED_TYPE, "probe", second, "ok"));
    statechart_host_invoker_step(&sm);
    bad |= check("restart", "started", counter(&sm, "started"), 1);
    statechart_host_invoker_destroy(&sm);
    return bad;
}

// A host-run invocation's `done.invoke` raised through `_raise_external`
// skipped the running check, so the machine refuses it and counts the refusal.
// The metadata names the invocation, so without the refusal the fixture's
// guarded transition would take it.
static int a_done_invoke_raised_the_old_way_is_refused_and_counted(void) {
    running_t run;
    statechart_host_invoker_t sm;
    boot_running(&sm, &run);
    statechart_host_invoker_event_with_meta_t done;
    memset(&done, 0, sizeof(done));
    done.event = STATECHART_HOST_INVOKER_EVENT_DONE_INVOKE_PROBE;
    (void)snprintf(done.invoke_id, sizeof(done.invoke_id), "%s", "probe");
    statechart_host_invoker_raise_external(&sm, &done);
    statechart_host_invoker_step(&sm);

    int bad = 0;
    bad |= check("old-way", "started", counter(&sm, "started"), 0);
    bad |= check("old-way", "refused completions", statechart_host_invoker_refused_host_invoke_completions(&sm), 1);
    statechart_host_invoker_destroy(&sm);
    return bad;
}

// W3C SCXML 6.4.1: an invoke with no id and an `idlocation` gets a generated
// `stateid.platformid` id, written to the location, handed to the host, and
// carried as `_event.invokeid` on the completion. The document names no
// specific `done.invoke.<id>`, so the completion arrives as the generic
// `done.invoke`, and `matched` counts it only when its invokeid is what the
// document stored.
static int an_idlocation_holds_the_id_the_host_is_handed(void) {
    recorder_t rec;
    memset(&rec, 0, sizeof(rec));
    statechart_host_invoker_t sm;
    boot(&sm, &rec, DECLARED_TYPE);
    deliver(&sm, STATECHART_HOST_INVOKER_EVENT_LEAVE);

    bool handed = false;
    for (int i = 0; i < rec.calls && i < (int)(sizeof(rec.log) / sizeof(rec.log[0])); i++) {
        handed = handed || strncmp(rec.log[i], "START id=done._invoke_0 ", strlen("START id=done._invoke_0 ")) == 0;
    }
    int bad = 0;
    bad |= expect("idlocation", "the host was not handed the generated id", handed);
    bad |= check("idlocation", "matched", counter(&sm, "matched"), 1);
    statechart_host_invoker_destroy(&sm);
    return bad;
}

// W3C SCXML 6.2.4 / 6.4.1: an `idlocation` is a location expression, so the id
// is written the way `<assign>` writes (W3C SCXML 5.4): `slot.id` and
// `slot.sid` are member paths, and land. `n.nope.deeper` cannot take a value,
// so each element raises error.execution and is abandoned (W3C SCXML 5.9.2) —
// the host is never asked to start that invoke, and that message is never
// sent. `slot` is read back as the JSON text the session's own serializer
// produces, whose key order is the document's.
static int an_idlocation_is_assigned_like_a_location(void) {
    recorder_t rec;
    memset(&rec, 0, sizeof(rec));
    statechart_host_invoker_t sm;
    boot(&sm, &rec, DECLARED_TYPE);
    deliver(&sm, STATECHART_HOST_INVOKER_EVENT_LOCATE);

    bool handed = false;
    bool leaked_start = false;
    for (int i = 0; i < rec.calls && i < (int)(sizeof(rec.log) / sizeof(rec.log[0])); i++) {
        handed =
            handed || strncmp(rec.log[i], "START id=locating._invoke_1 ", strlen("START id=locating._invoke_1 ")) == 0;
        leaked_start = leaked_start || strstr(rec.log[i], "locating._invoke_2") != NULL;
    }
    int bad = 0;
    bad |= expect("locate", "the member-path invoke was not started", handed);
    bad |= expect("locate", "an invoke whose idlocation could not take the id was started", !leaked_start);

    char slot[256];
    const bool read = statechart_host_invoker_slot(&sm, slot, sizeof(slot), NULL);
    bad |= expect("locate", "the fixture declares `slot` as an object", read);
    bad |= expect("locate", "slot.id is not the generated invoke id",
                  read && strstr(slot, "\"id\":\"locating._invoke_1\"") != NULL);
    bad |= expect("locate", "slot.sid did not receive the send id",
                  read && strstr(slot, "\"sid\":\"") != NULL && strstr(slot, "\"sid\":\"\"") == NULL);

    bad |= check("locate", "slotted", counter(&sm, "slotted"), 1);
    bad |= check("locate", "pinged", counter(&sm, "pinged"), 1);
    bad |= check("locate", "leaked", counter(&sm, "leaked"), 0);
    bad |= check("locate", "lost", counter(&sm, "lost"), 2);
    statechart_host_invoker_destroy(&sm);
    return bad;
}

// The generic `done.invoke` is a host completion too when its invokeid names a
// host-run invoke, so raised around `_complete_host_invoke` it is refused like
// the specific name is.
static int a_generic_done_invoke_raised_the_old_way_is_refused(void) {
    running_t run;
    statechart_host_invoker_t sm;
    boot_running(&sm, &run);
    deliver(&sm, STATECHART_HOST_INVOKER_EVENT_LEAVE);
    statechart_host_invoker_event_with_meta_t done;
    memset(&done, 0, sizeof(done));
    done.event = STATECHART_HOST_INVOKER_EVENT_DONE_INVOKE;
    (void)snprintf(done.invoke_id, sizeof(done.invoke_id), "%s", "done._invoke_0");
    statechart_host_invoker_raise_external(&sm, &done);
    statechart_host_invoker_step(&sm);

    int bad = 0;
    bad |= check("generic-old-way", "matched", counter(&sm, "matched"), 0);
    bad |= check("generic-old-way", "refused completions", statechart_host_invoker_refused_host_invoke_completions(&sm),
                 1);
    statechart_host_invoker_destroy(&sm);
    return bad;
}

// The invocation ends with the state that started it. Still running when the
// state exits — a completed invocation has nothing left to cancel (the case
// above).
static int leaving_the_state_cancels_the_invocation(void) {
    running_t run;
    statechart_host_invoker_t sm;
    boot_running(&sm, &run);
    deliver(&sm, STATECHART_HOST_INVOKER_EVENT_LEAVE);
    recorder_t rec = run.rec;

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

    // Now one that is still running: cancelled once, and the second cancel has
    // nothing left to do.
    running_t run;
    boot_running(&sm, &run);
    if (!statechart_host_invoker_cancel_host_invoke(&sm, DECLARED_TYPE, "probe")) {
        (void)fprintf(stderr, "host_invoker: FAIL [never-started] - a started invocation reported nothing to cancel\n");
        bad = 1;
    }
    if (statechart_host_invoker_cancel_host_invoke(&sm, DECLARED_TYPE, "probe")) {
        (void)fprintf(stderr, "host_invoker: FAIL [never-started] - the same invocation was cancelled twice\n");
        bad = 1;
    }
    bad |= check("never-started", "cancels of probe", count_lines(&run.rec, "CANCEL id=probe"), 1);
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

// An invoker that answers nothing and records, per start, whether the request
// still carried the deadline param. `running_t`'s bookkeeping, with the param
// in the line.
static void timed_invoker(void *user_data, const sce_host_invoke_event_t *event, sce_host_invoke_response_t *out) {
    (void)out;
    running_t *run = (running_t *)user_data;
    char line[128];
    if (event->phase == SCE_HOST_INVOKE_START) {
        (void)snprintf(line, sizeof(line), "START id=%s deadline-param=%s", event->invoke_id,
                       param(event, SCE_HOST_INVOKE_DEADLINE_PARAM) != NULL ? "true" : "false");
        if (run->starts < (int)(sizeof(run->tokens) / sizeof(run->tokens[0]))) {
            (void)snprintf(run->ids[run->starts], sizeof(run->ids[0]), "%s", event->invoke_id);
            run->tokens[run->starts] = event->token;
        }
        run->starts++;
    } else {
        (void)snprintf(line, sizeof(line), "CANCEL id=%s", event->invoke_id);
    }
    record(&run->rec, line);
}

// A machine on a manual clock, driven into `timed`. The clock is handed to
// `_init`, which arms against it.
static void boot_timed(statechart_host_invoker_t *sm, running_t *run) {
    sce_host_invoker_registry_t wiring;
    memset(run, 0, sizeof(*run));
    memset(&wiring, 0, sizeof(wiring));
    (void)sce_host_invoker_register(&wiring, DECLARED_TYPE, timed_invoker, run);
    statechart_host_invoker_init_with_clock_and_host_invokers(sm, sce_clock_manual(0), &wiring);
    deliver(sm, STATECHART_HOST_INVOKER_EVENT_TIME);
}

// A deadline that passes while the invocation is still running ends it: the
// host is told to stop, the document receives `error.invoke.slow` with
// `_event.data` "deadline", and a reply afterwards is refused. The param is the
// machine's — the host never sees it — and a `<cancel>` of the empty send id
// does not reach it.
static int a_deadline_that_passes_ends_the_invocation(void) {
    running_t run;
    statechart_host_invoker_t sm;
    boot_timed(&sm, &run);

    int bad = 0;
    bad |= check("deadline", "starts of slow without the param",
                 count_lines(&run.rec, "START id=slow deadline-param=false"), 1);
    deliver(&sm, STATECHART_HOST_INVOKER_EVENT_FORGET);
    statechart_host_invoker_advance_time_ms(&sm, 49);
    bad |= check("deadline", "expired at 49 ms", counter(&sm, "expired"), 0);
    statechart_host_invoker_advance_time_ms(&sm, 1);
    bad |= check("deadline", "expired at 50 ms", counter(&sm, "expired"), 1);
    bad |= check("deadline", "cancels of slow", count_lines(&run.rec, "CANCEL id=slow"), 1);
    bad |= expect(
        "deadline", "a reply after the deadline was refused",
        !statechart_host_invoker_complete_host_invoke(&sm, DECLARED_TYPE, "slow", token_of(&run, "slow"), "late"));
    statechart_host_invoker_step(&sm);
    bad |= check("deadline", "finished", counter(&sm, "finished"), 0);
    statechart_host_invoker_destroy(&sm);
    return bad;
}

// The discriminator: a completion before the deadline is the outcome, and the
// deadline that comes due afterwards does nothing — no cancel, no
// `error.invoke`, and nothing left for the host to tick toward.
static int a_completion_before_the_deadline_disarms_it(void) {
    running_t run;
    statechart_host_invoker_t sm;
    boot_timed(&sm, &run);

    int bad = 0;
    bad |=
        expect("disarm", "a running invocation's completion was accepted",
               statechart_host_invoker_complete_host_invoke(&sm, DECLARED_TYPE, "slow", token_of(&run, "slow"), "ok"));
    statechart_host_invoker_step(&sm);
    bad |= check("disarm", "finished", counter(&sm, "finished"), 1);
    bad |= check("disarm", "time until next scheduled", statechart_host_invoker_time_until_next_scheduled_ms(&sm), -1);
    statechart_host_invoker_advance_time_ms(&sm, 100);
    bad |= check("disarm", "expired", counter(&sm, "expired"), 0);
    bad |= check("disarm", "cancels of slow", count_lines(&run.rec, "CANCEL id=slow"), 0);
    statechart_host_invoker_destroy(&sm);
    return bad;
}

// W3C SCXML 6.4.1: a deadline that is not a whole number of milliseconds is an
// argument that cannot be evaluated — error.execution, and the host is never
// asked to start the invocation.
static int a_deadline_that_is_not_milliseconds_starts_nothing(void) {
    running_t run;
    statechart_host_invoker_t sm;
    boot_timed(&sm, &run);

    int bad = 0;
    bad |= check("undated", "misdated", counter(&sm, "misdated"), 1);
    bad |= check("undated", "starts of undated", count_lines(&run.rec, "START id=undated deadline-param=false"), 0);
    bad |= check("undated", "starts of undated with the param",
                 count_lines(&run.rec, "START id=undated deadline-param=true"), 0);
    statechart_host_invoker_destroy(&sm);
    return bad;
}

// The table's JSON is read by walking it, not by a library this test does not
// link: whitespace and commas separate, `[` opens a list, `]` closes one, and a
// string is a quoted run with no escape — the table has no use for one, and a
// reader that ignored an escape would misread it silently, so meeting one is a
// failure.
static const char *skip_separators(const char *p) {
    while (*p == ' ' || *p == '\n' || *p == '\r' || *p == '\t' || *p == ',') {
        p++;
    }
    return p;
}

// The string literal at `*cursor` (after separators), copied into `out`, with
// `*cursor` moved past it.
static bool read_table_string(const char **cursor, char *out, size_t cap) {
    const char *p = skip_separators(*cursor);
    if (*p != '"') {
        return false;
    }
    p++;
    size_t n = 0;
    while (*p != '\0' && *p != '"') {
        if (*p == '\\' || n + 1 >= cap) {
            (void)fprintf(stderr, "host_invoker: FAIL [table] - a string this reader cannot take\n");
            return false;
        }
        out[n++] = *p++;
    }
    if (*p != '"') {
        return false;
    }
    out[n] = '\0';
    *cursor = p + 1;
    return true;
}

// The position just inside the list that follows the key `key`, or NULL.
static const char *open_table_list(const char *text, const char *key) {
    const char *at = strstr(text, key);
    if (at == NULL) {
        return NULL;
    }
    at = strchr(at, '[');
    return at != NULL ? at + 1 : NULL;
}

// Every runtime reads a deadline's text by one grammar, held to one table.
// strtoull would not do: it skips leading whitespace and reads a sign, and a
// deadline this backend honours while another refuses it makes a document
// depend on where it was compiled.
static int a_deadline_is_read_by_the_shared_table(void) {
    static char text[8192];
    FILE *file = fopen(SCE_HOST_INVOKE_DEADLINE_TABLE, "rb");
    if (file == NULL) {
        (void)fprintf(stderr, "host_invoker: FAIL [table] - cannot read %s\n", SCE_HOST_INVOKE_DEADLINE_TABLE);
        return 1;
    }
    size_t len = fread(text, 1, sizeof(text) - 1, file);
    (void)fclose(file);
    text[len] = '\0';

    int bad = 0;
    int accepted = 0;
    int refused = 0;
    char written[64];
    char ms[64];
    // `accepted` is a list of `[written, ms]` pairs.
    const char *cursor = open_table_list(text, "\"accepted\"");
    while (cursor != NULL) {
        cursor = skip_separators(cursor);
        if (*cursor != '[') {
            break;
        }
        cursor++;
        if (!read_table_string(&cursor, written, sizeof(written)) || !read_table_string(&cursor, ms, sizeof(ms))) {
            bad |= expect("table", "an accepted pair is two strings", false);
            break;
        }
        cursor = skip_separators(cursor);
        if (*cursor != ']') {
            bad |= expect("table", "an accepted pair is two strings", false);
            break;
        }
        cursor++;
        uint64_t got = 0;
        const bool ok = sce_parse_host_invoke_deadline_ms(written, &got);
        if (!ok || got != (uint64_t)strtoull(ms, NULL, 10)) {
            (void)fprintf(stderr, "host_invoker: FAIL [table] - \"%s\" did not read as %s\n", written, ms);
            bad = 1;
        }
        accepted++;
    }
    // `refused` is a list of strings.
    cursor = open_table_list(text, "\"refused\"");
    while (cursor != NULL && read_table_string(&cursor, written, sizeof(written))) {
        uint64_t got = 0;
        if (sce_parse_host_invoke_deadline_ms(written, &got)) {
            (void)fprintf(stderr, "host_invoker: FAIL [table] - \"%s\" was accepted\n", written);
            bad = 1;
        }
        refused++;
    }
    // A floor: an empty read would pass every check above.
    bad |= expect("table", "the table's accepted list was read", accepted > 0);
    bad |= expect("table", "the table's refused list was read", refused > 0);
    return bad;
}

int main(void) {
    int bad = 0;
    bad |= a_registered_invoker_is_started_with_what_the_document_wrote();
    bad |= what_the_request_says_is_evaluated_when_the_invocation_starts();
    bad |= leaving_the_state_cancels_the_invocation();
    bad |= cancel_is_not_delivered_for_an_invocation_that_never_started();
    bad |= a_completed_invocation_is_not_cancelled();
    bad |= a_late_completion_is_accepted_exactly_once();
    bad |= a_completion_after_the_cancel_is_refused();
    bad |= a_restarted_invoke_refuses_the_first_runs_reply();
    bad |= a_done_invoke_raised_the_old_way_is_refused_and_counted();
    bad |= an_idlocation_holds_the_id_the_host_is_handed();
    bad |= an_idlocation_is_assigned_like_a_location();
    bad |= a_generic_done_invoke_raised_the_old_way_is_refused();
    bad |= a_declared_type_with_no_invoker_still_raises_error_execution();
    bad |= an_invoker_registered_for_another_type_does_not_run_this_one();
    bad |= a_deadline_that_passes_ends_the_invocation();
    bad |= a_completion_before_the_deadline_disarms_it();
    bad |= a_deadline_that_is_not_milliseconds_starts_nothing();
    bad |= a_deadline_is_read_by_the_shared_table();

    if (bad != 0) {
        (void)fprintf(stderr, "host_invoker: FAIL - see the scenario(s) named above\n");
        return 1;
    }
    (void)printf("host_invoker: PASS - every scenario\n");
    return 0;
}
