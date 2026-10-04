// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// datamodel="sce-static" (docs/SCE_ACCEPTED_SUBSET.md §2.15) under generated C:
// delayed <send>s that are still waiting, which the machine's own clock brings
// due and which a <cancel> removes.
//
// The document is `static_timers.scxml`, the one the Rust, Kotlin and Go suites
// save and restore. Four sends are armed on entering `waiting`:
//
//   inner    1 s  to the internal queue
//   beat     2 s  to the external queue
//   echo     2 s  to the external queue, due the same moment as `beat`
//   timeout  5 s  to the external queue, id `timer`, which `stop` cancels
//
// Each appends a digit to `trace` when it is delivered (`inner` 1, `beat` 2,
// `echo` 4, `timeout` 3), so the number says which were delivered and in what
// order. The host owns the clock, so no case sleeps and none depends on how
// loaded the machine is.
//
// Linked WITHOUT `sce_c_scripting` and `lua54`: the machine has no script
// engine, so the link is the proof.

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "static_timers_sm.h"

typedef static_timers_t sm_t;

// A machine started on a clock the test moves, with its entry already run.
static void start(sm_t *sm) {
    static_timers_init_with_clock(sm, sce_clock_manual(0u));
    static_timers_run(sm);
}

// The clock moves `ms` and the machine delivers what has come due.
static void advance(sm_t *sm, uint64_t ms) {
    static_timers_advance_time_ms(sm, ms);
    static_timers_run(sm);
}

static void stop(sm_t *sm) {
    static_timers_event_with_meta_t meta;
    memset(&meta, 0, sizeof(meta));
    meta.event = STATIC_TIMERS_EVENT_STOP;
    static_timers_raise_external(sm, &meta);
    static_timers_run(sm);
}

static int expect_trace(const sm_t *sm, const char *when, uint32_t want) {
    const uint32_t got = static_timers_get_trace(sm);
    if (got != want) {
        (void)fprintf(stderr, "FAIL: %s: trace is %u, want %u\n", when, (unsigned)got, (unsigned)want);
        return 1;
    }
    return 0;
}

static int expect_state(const sm_t *sm, const char *when, bool done) {
    const bool in_done = static_timers_ended_in(sm, STATIC_TIMERS_STATE_DONE);
    const bool waiting = static_timers_in_state(sm, STATIC_TIMERS_STATE_WAITING);
    if (done ? !in_done : (in_done || !waiting)) {
        (void)fprintf(stderr, "FAIL: %s: the machine %s in `done`, want it %s\n", when, in_done ? "is" : "is not",
                      done ? "in" : "still waiting, not");
        return 1;
    }
    return 0;
}

// Each send is delivered when its delay has passed and not before, and `beat`
// reaches the machine ahead of `echo`: they are due together and were sent in
// that order, so a queue that kept them by anything but arrival would give
// 1423.
static int the_sends_arrive_when_due_in_the_order_they_were_sent(void) {
    sm_t sm;
    start(&sm);
    int bad = expect_trace(&sm, "at the start", 0u);
    advance(&sm, 999u);
    bad |= expect_trace(&sm, "a millisecond before `inner`", 0u);
    advance(&sm, 1u);
    bad |= expect_trace(&sm, "at 1 s, `inner` due", 1u);
    advance(&sm, 999u);
    bad |= expect_trace(&sm, "a millisecond before `beat` and `echo`", 1u);
    advance(&sm, 1u);
    bad |= expect_trace(&sm, "at 2 s, `beat` then `echo` due", 124u);
    bad |= expect_state(&sm, "at 2 s", false);
    advance(&sm, 3000u);
    bad |= expect_trace(&sm, "at 5 s, `timeout` due", 1243u);
    bad |= expect_state(&sm, "at 5 s", true);
    static_timers_destroy(&sm);
    return bad;
}

// A clock that jumps past several sends delivers them all, in the order they
// fell due and not the order the host noticed them.
static int a_jump_past_every_send_delivers_them_in_due_order(void) {
    sm_t sm;
    start(&sm);
    advance(&sm, 60000u);
    int bad = expect_trace(&sm, "after one jump to 60 s", 1243u);
    bad |= expect_state(&sm, "after one jump to 60 s", true);
    static_timers_destroy(&sm);
    return bad;
}

// `stop` cancels the send whose id it names and no other: `beat` and `echo`
// still arrive, `timeout` never does, and the machine stays where it was.
static int a_cancel_removes_the_send_it_names(void) {
    sm_t sm;
    start(&sm);
    advance(&sm, 1000u);
    stop(&sm);
    advance(&sm, 100000u);
    int bad = expect_trace(&sm, "after `stop` and a long wait", 124u);
    bad |= expect_state(&sm, "after `stop` and a long wait", false);
    static_timers_destroy(&sm);
    return bad;
}

int main(void) {
    int bad = 0;
    bad |= the_sends_arrive_when_due_in_the_order_they_were_sent();
    bad |= a_jump_past_every_send_delivers_them_in_due_order();
    bad |= a_cancel_removes_the_send_it_names();
    if (bad != 0) {
        return 1;
    }
    (void)printf("static timers: ok\n");
    return 0;
}
