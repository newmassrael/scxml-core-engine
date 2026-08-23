// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2.4 + 6.3 — a `<send delay>` addressed to a HOST-served Event
// I/O Processor waits, and can be cancelled while it waits. C11 AOT path.
//
// W3C SCXML 6.2.4 puts the wait before the dispatch and says nothing about
// which processor the send named; 6.2.5 makes that set open. Put together, a
// host-served send carrying a delay is an ordinary delayed send whose delivery
// happens to be somebody else's. It was not: every backend chose the host
// branch ahead of the delay branch in one `elif` chain per language, so the act
// was performed at the instant the block ran and `delay` was discarded — while
// the manifest went on answering `needs_event_scheduler: true`, telling the
// host to drive with `_tick` for a wait the engine had already thrown away.
//
// Driven entirely on `sce_clock_manual`. Nothing here sleeps and nothing here
// can be decided by how loaded the build machine is: the host sets what time it
// is and the engine answers with the configuration that time implies. That
// matters more than usual on this axis, because a wall-clock version of the
// first scenario would pass on a slow machine for the wrong reason — the
// handler running "early" is only observable against a clock the test controls.
//
// This backend's entry is a fixed-size struct, so it also measures something
// the heap backends cannot get wrong: the request's strings are COPIED into the
// queue slot at send time. W3C SCXML 6.2 evaluates a send's fields when the
// block runs, and the buffers that held them are gone by the deadline.
//
// Fixture: sce-build/tests/fixtures/host_processor/statechart_delayed_host_send.scxml
// (the same document the other five channels drive), generated WITH
// `--host-processor x-sce-host` by `backends/c/tests/CMakeLists.txt`.

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "statechart_delayed_host_send_sm.h"

// The type the fixture was compiled for. `backends/c/tests/CMakeLists.txt`
// passes this same string to `--host-processor`.
#define DECLARED_TYPE "x-sce-host"

// What the handler saw: the engine's own reading of "now" at the moment it was
// asked to perform the act, in call order.
//
// The engine's clock rather than the test's bookkeeping, because that is the
// number the contract is about — a handler called at 0 ms for a
// `delay="200ms"` send is the defect, and any other witness only says it
// happened, not when the engine thought it was.
#define MAX_CALLS 8

typedef struct {
    int count;
    uint64_t at_ms[MAX_CALLS];
    char seen_type[64];
    char seen_event[64];
    // The machine, so the handler can ask it what time the engine thinks it is
    // rather than being told by the test.
    const statechart_delayed_host_send_t *sm;
} recorder_t;

static void answering_handler(void *user_data, const sce_host_send_request_t *request,
                              sce_host_send_response_list_t *out) {
    recorder_t *rec = (recorder_t *)user_data;
    if (rec->count < MAX_CALLS) {
        rec->at_ms[rec->count] = statechart_delayed_host_send_now_ms(rec->sm);
    }
    rec->count++;
    (void)snprintf(rec->seen_type, sizeof(rec->seen_type), "%s",
                   request->processor_type != NULL ? request->processor_type : "");
    (void)snprintf(rec->seen_event, sizeof(rec->seen_event), "%s",
                   request->event_name != NULL ? request->event_name : "");
    (void)sce_host_response_push(out, "turn.done", "");
}

// Start the machine on host-owned time with the wiring the caller asked for.
//
// Both seams at once, which is what `_init_with_clock_and_host_processors`
// exists for: `_init` is construction and initialisation in one call, and the
// fixture's first send is armed on the way into its initial state.
static void boot(statechart_delayed_host_send_t *sm, recorder_t *rec, bool with_handler) {
    sce_host_processor_registry_t wiring;
    memset(&wiring, 0, sizeof(wiring));
    if (with_handler) {
        rec->sm = sm;
        (void)sce_host_registry_register(&wiring, DECLARED_TYPE, answering_handler, rec);
    }
    statechart_delayed_host_send_init_with_clock_and_host_processors(sm, sce_clock_manual(0u), &wiring);
}

static int fail(const char *scenario, const char *what) {
    (void)fprintf(stderr, "delayed_host_send: FAIL [%s] - %s\n", scenario, what);
    return 1;
}

// The axis. `waiting` arms a host-served send for 200 ms and an ordinary one
// for 100 ms; the ordinary one must arrive first, which is only true if the
// host-served one waited.
//
// The `tooEarly` final state is what the document reaches when it did not: the
// handler's reply is on the queue before the machine has been anywhere, so
// `turn.done` wins the race its own `delay` was supposed to lose.
static int a_host_served_send_waits_for_its_delay(void) {
    recorder_t rec;
    memset(&rec, 0, sizeof(rec));
    statechart_delayed_host_send_t sm;
    boot(&sm, &rec, true);

    int bad = 0;
    // Nothing is due at 0 ms. This is the whole defect in one assertion: with
    // the host branch chosen ahead of the delay branch, `_init` has already
    // performed the act by the time this line runs.
    if (rec.count != 0) {
        bad |= fail("waits", "the handler was asked to perform a delay=\"200ms\" send at 0 ms. W3C SCXML "
                             "6.2.4 makes the delay the wait the document asked for, and 6.2.5 does not "
                             "exempt a host-served processor from it");
    }
    if (!statechart_delayed_host_send_in_state(&sm, STATECHART_DELAYED_HOST_SEND_STATE_WAITING)) {
        bad |= fail("waits", "the machine should be waiting on its two delayed sends");
    }

    // 100 ms: the ordinary `probe` is due, the host-served send is not.
    statechart_delayed_host_send_advance_time_ms(&sm, 100u);
    if (!statechart_delayed_host_send_in_state(&sm, STATECHART_DELAYED_HOST_SEND_STATE_ARMED)) {
        bad |= fail("waits", "the 100 ms `probe` did not arrive first");
    }
    if (rec.count != 0) {
        bad |= fail("waits", "the host-served send was dispatched before its 200 ms deadline");
    }

    // 200 ms: now it is due, and the handler's reply moves the machine on.
    statechart_delayed_host_send_advance_time_ms(&sm, 100u);
    if (rec.count != 1 || rec.at_ms[0] != 200u) {
        (void)fprintf(stderr,
                      "delayed_host_send: FAIL [waits] - the host-served send did not fire at its 200 ms "
                      "deadline (calls=%d, first at %llu ms)\n",
                      rec.count, (unsigned long long)(rec.count > 0 ? rec.at_ms[0] : 0u));
        bad = 1;
    }
    // The strings crossed the wait: the buffers that held them belonged to the
    // `<send>` block and are long gone.
    if (strcmp(rec.seen_type, DECLARED_TYPE) != 0 || strcmp(rec.seen_event, "watch.turn") != 0) {
        (void)fprintf(stderr,
                      "delayed_host_send: FAIL [waits] - the deferred request arrived as type `%s` event "
                      "`%s`; the queue slot must OWN what the send evaluated\n",
                      rec.seen_type, rec.seen_event);
        bad = 1;
    }
    if (!statechart_delayed_host_send_in_state(&sm, STATECHART_DELAYED_HOST_SEND_STATE_CANCELLING)) {
        bad |= fail("waits", "the handler's `turn.done` did not reach the document");
    }

    statechart_delayed_host_send_destroy(&sm);
    return bad;
}

// W3C SCXML 6.3: a `<cancel>` drops a delayed send that has not been
// dispatched. A host-served one is not exempt, and the witness is host-side:
// the handler must never be asked to perform the cancelled act at all.
//
// This is the half that says which queue the deferred send is in. An engine
// that honoured the delay by any private means — a side list, a second queue —
// would pass the scenario above and fail here, because `<cancel sendid>`
// reaches the scheduler and nothing else.
static int a_cancel_drops_a_pending_host_served_send(void) {
    recorder_t rec;
    memset(&rec, 0, sizeof(rec));
    statechart_delayed_host_send_t sm;
    boot(&sm, &rec, true);

    int bad = 0;
    statechart_delayed_host_send_advance_time_ms(&sm, 100u);  // probe     -> armed
    statechart_delayed_host_send_advance_time_ms(&sm, 100u);  // turn.done -> cancelling
    statechart_delayed_host_send_advance_time_ms(&sm, 100u);  // settle    -> cancelPending
    if (!statechart_delayed_host_send_in_state(&sm, STATECHART_DELAYED_HOST_SEND_STATE_CANCELPENDING)) {
        bad |= fail("cancel", "the second round did not reach the state that runs <cancel sendid=\"h2\">");
    }

    // 400 ms: h2's deadline. It was cancelled at 300, so nothing may happen.
    statechart_delayed_host_send_advance_time_ms(&sm, 100u);
    if (rec.count != 1) {
        (void)fprintf(stderr,
                      "delayed_host_send: FAIL [cancel] - the handler was asked to perform `h2` at 400 ms "
                      "after <cancel sendid=\"h2\"> ran at 300 ms (calls=%d). A host-served act that a "
                      "document cancelled must not reach the host: the side effect is the point of the "
                      "act, and the document cannot take it back\n",
                      rec.count);
        bad = 1;
    }
    if (statechart_delayed_host_send_in_state(&sm, STATECHART_DELAYED_HOST_SEND_STATE_CANCELLOST)) {
        bad |= fail("cancel", "`turn.done` arrived for the cancelled send");
    }

    // 500 ms: `finish`. The verdict is itself scheduled, so a channel whose
    // tick loop stopped working fails here rather than passing by not moving.
    statechart_delayed_host_send_advance_time_ms(&sm, 100u);
    if (!statechart_delayed_host_send_in_state(&sm, STATECHART_DELAYED_HOST_SEND_STATE_PASS)) {
        bad |= fail("cancel", "the machine did not reach `pass`");
    }

    statechart_delayed_host_send_destroy(&sm);
    return bad;
}

// A deferred act whose handler was never registered is still an act nobody
// performed, and W3C SCXML 6.2 reports that as `error.execution` — at the
// moment it was to be performed, not at the moment it was armed.
//
// The immediate path raises this at the send site. The deferred path cannot:
// that site has already returned by the time the deadline arrives, so whatever
// holds the act owes the report. Without this scenario a wiring mistake on a
// delayed send is perfect silence — the document waits for a reply that no
// longer has anyone to come from.
static int a_deferred_send_with_no_handler_reports_it_when_it_comes_due(void) {
    recorder_t rec;
    memset(&rec, 0, sizeof(rec));
    statechart_delayed_host_send_t sm;
    boot(&sm, &rec, false);

    int bad = 0;
    // At 100 ms the machine is in `armed`, whose `error.execution` transition
    // is the witness. Nothing has reported anything yet: the send was armed,
    // not performed, so there is nothing to report.
    statechart_delayed_host_send_advance_time_ms(&sm, 100u);
    if (!statechart_delayed_host_send_in_state(&sm, STATECHART_DELAYED_HOST_SEND_STATE_ARMED)) {
        bad |= fail("unserved", "the report arrived before the send was due; error.execution must be raised "
                                "when the act was to be performed, not when it was armed");
    }

    // 200 ms: the deadline. Nobody is registered, so nobody performs it.
    statechart_delayed_host_send_advance_time_ms(&sm, 100u);
    if (statechart_delayed_host_send_in_state(&sm, STATECHART_DELAYED_HOST_SEND_STATE_CANCELLING)) {
        bad |= fail("unserved", "nothing was registered to perform the act, yet `turn.done` arrived");
    }
    if (!statechart_delayed_host_send_in_state(&sm, STATECHART_DELAYED_HOST_SEND_STATE_UNSERVED)) {
        bad |= fail("unserved", "the deadline passed with no handler registered and nothing was reported. "
                                "The send site that raises this for an immediate send returned when the "
                                "send was armed, so whatever holds the deferred act owes the report");
    }

    statechart_delayed_host_send_destroy(&sm);
    return bad;
}

// The engine must be able to say when the deferred host send comes due, or a
// host driving on `_time_until_next_scheduled_ms` sleeps straight past it.
//
// A deferred act kept anywhere the deadline query cannot see would leave this
// answering -1 at 0 ms — "nothing is owed" — while an act was owed at 200.
static int the_engine_says_when_the_deferred_host_send_is_due(void) {
    recorder_t rec;
    memset(&rec, 0, sizeof(rec));
    statechart_delayed_host_send_t sm;
    boot(&sm, &rec, true);

    int bad = 0;
    int64_t due = statechart_delayed_host_send_time_until_next_scheduled_ms(&sm);
    if (due != 100) {
        (void)fprintf(stderr,
                      "delayed_host_send: FAIL [due] - the nearer of the two armed sends is the 100 ms "
                      "`probe`; the engine answered %lld\n",
                      (long long)due);
        bad = 1;
    }

    statechart_delayed_host_send_advance_time_ms(&sm, 100u);
    due = statechart_delayed_host_send_time_until_next_scheduled_ms(&sm);
    if (due != 100) {
        (void)fprintf(stderr,
                      "delayed_host_send: FAIL [due] - at 100 ms the host-served send is 100 ms out; the "
                      "engine answered %lld. A host sleeping on this answer must land on the deferred act, "
                      "not past it\n",
                      (long long)due);
        bad = 1;
    }

    statechart_delayed_host_send_destroy(&sm);
    return bad;
}

int main(void) {
    int bad = 0;
    bad |= a_host_served_send_waits_for_its_delay();
    bad |= a_cancel_drops_a_pending_host_served_send();
    bad |= a_deferred_send_with_no_handler_reports_it_when_it_comes_due();
    bad |= the_engine_says_when_the_deferred_host_send_is_due();

    if (bad != 0) {
        return 1;
    }
    (void)printf("delayed_host_send: PASS - a host-served <send delay> waits, and a <cancel> drops it\n");
    return 0;
}
