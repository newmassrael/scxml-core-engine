// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A machine that answers an event by sending itself the next one, with no
// target, never lets the external queue empty - C11 AOT path.
//
// Every macrostep of such a machine ends, so SCE_MAX_MACROSTEP_MICROSTEPS never
// applies, and the main event loop takes the next external event whenever the
// queue is not empty: a host call that drains it did not return. The C11
// template's run_main_event_loop had that shape until it took the budget
// ARCHITECTURE.md "External-Event Budget" states as one contract for every
// engine. This driver holds this engine to it: the same outcomes, the same
// arithmetic, the same document as the Python, Rust, Go, Kotlin and C++ ones.
//
// The external queue here is a fixed ring (SCE_MAX_EVENTS) that drops on
// overflow, which is a different bound and not this one: a chain that sends the
// next event holds one at a time, so the ring never fills and never stopped it.
//
// The delayed outcomes run on a manual clock, so nothing here sleeps and the
// clock moves only where a case moves it. This engine hands a STATIC zero delay
// to its scheduler (Rust reads it as undelayed), so `zero` and `zero_expr` both
// reach the same-instant bound here.
//
// Fixture: tests/integration/external_chain_is_bounded.scxml. It is outside
// integration_resources/ for the reason scripts/regen_external_chain_is_bounded.sh
// states.
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(external_chain_is_bounded ...)` in
// `backends/c/tests/CMakeLists.txt`.

#include <stdint.h>
#include <stdio.h>

#include "external_chain_is_bounded_sm.h"

// The default the contract states, spelled here rather than read back from the
// engine: a test that asked the engine for its own limit would agree with any
// limit, including one an edit moved by three orders of magnitude.
#define DEFAULT_BUDGET 10000u

static int failures = 0;

#define EXPECT(cond, ...)                                                                                              \
    do {                                                                                                               \
        if (!(cond)) {                                                                                                 \
            fprintf(stderr, "external_chain_is_bounded: FAIL - ");                                                     \
            fprintf(stderr, __VA_ARGS__);                                                                              \
            fprintf(stderr, "\n");                                                                                     \
            failures = 1;                                                                                              \
        }                                                                                                              \
    } while (0)

// The machine on host-owned time. The clock is chosen at init: the engine
// refuses one afterwards, because deadlines armed against one clock do not
// compare with another.
static void started(external_chain_is_bounded_t *sm) {
    external_chain_is_bounded_init_with_clock(sm, sce_clock_manual(0u));
}

// One host call that delivers `event`: it joins the queue and the engine takes
// it off as the first event of the invocation that runs.
static void deliver(external_chain_is_bounded_t *sm, external_chain_is_bounded_event_t event) {
    external_chain_is_bounded_event_with_meta_t carrier = {0};
    carrier.event = event;
    external_chain_is_bounded_raise_external(sm, &carrier);
    external_chain_is_bounded_tick(sm);
}

// The fixture's <assign>s are the only witness of how far a chain got: every
// outcome leaves the machine in a state the configuration alone cannot tell
// apart from the others.
static int64_t counter_links(external_chain_is_bounded_t *sm) {
    int64_t v = -1;
    (void)external_chain_is_bounded_links(sm, &v);
    return v;
}

static int64_t counter_laps(external_chain_is_bounded_t *sm) {
    int64_t v = -1;
    (void)external_chain_is_bounded_laps(sm, &v);
    return v;
}

static int64_t counter_beats(external_chain_is_bounded_t *sm) {
    int64_t v = -1;
    (void)external_chain_is_bounded_beats(sm, &v);
    return v;
}

static int64_t counter_pokes(external_chain_is_bounded_t *sm) {
    int64_t v = -1;
    (void)external_chain_is_bounded_pokes(sm, &v);
    return v;
}

static int64_t counter_blinks(external_chain_is_bounded_t *sm) {
    int64_t v = -1;
    (void)external_chain_is_bounded_blinks(sm, &v);
    return v;
}

static int64_t counter_exprs(external_chain_is_bounded_t *sm) {
    int64_t v = -1;
    (void)external_chain_is_bounded_exprs(sm, &v);
    return v;
}

static int64_t counter_pulses(external_chain_is_bounded_t *sm) {
    int64_t v = -1;
    (void)external_chain_is_bounded_pulses(sm, &v);
    return v;
}

int main(void) {
    // The default budget is the documented one, and nothing has been cut before
    // the machine has done anything. The lower bound is the half a check like
    // this usually forgets: an accessor that always answers reads exactly like a
    // working one from the other side, so the head of the queue is asked too.
    {
        external_chain_is_bounded_t sm;
        started(&sm);
        EXPECT(external_chain_is_bounded_max_external_events_per_call(&sm) == DEFAULT_BUDGET,
               "the default budget is %u, got %u", DEFAULT_BUDGET,
               external_chain_is_bounded_max_external_events_per_call(&sm));
        EXPECT(external_chain_is_bounded_truncated_event_chains(&sm) == 0u,
               "something was cut before the machine had done anything");
        external_chain_is_bounded_event_t head = EXTERNAL_CHAIN_IS_BOUNDED_EVENT_POKE;
        EXPECT(!external_chain_is_bounded_last_truncated_event(&sm, &head),
               "the machine named a cut event before anything had been cut");
    }

    // This block returning at all is half the assertion: before the budget the
    // call did not.
    {
        external_chain_is_bounded_t sm;
        started(&sm);

        deliver(&sm, EXTERNAL_CHAIN_IS_BOUNDED_EVENT_SPIN);

        EXPECT(external_chain_is_bounded_truncated_event_chains(&sm) == 1u,
               "the call handed control back with an event still queued, and said so; without "
               "the count the host sees a machine that is running and has returned");
        // The host's own event is the first of the invocation, so the budget buys
        // the host's event and then budget - 1 links.
        EXPECT(counter_links(&sm) == (int64_t)DEFAULT_BUDGET - 1,
               "the chain must run exactly as far as the budget allows (want %lld, got %lld)",
               (long long)DEFAULT_BUDGET - 1, (long long)counter_links(&sm));
        external_chain_is_bounded_event_t head = EXTERNAL_CHAIN_IS_BOUNDED_EVENT_POKE;
        EXPECT(external_chain_is_bounded_last_truncated_event(&sm, &head) &&
                   head == EXTERNAL_CHAIN_IS_BOUNDED_EVENT_LINK,
               "the count says a call did not reach quiet; this says what it was still taking");
        EXPECT(sm.is_running, "the chain was cut, not the machine");
    }

    // The half that makes the count mean something: a chain that ends on its own
    // is not refused, however close to the budget it comes. `bounded` is the
    // host's event and five laps, six in all.
    {
        external_chain_is_bounded_t exactly;
        started(&exactly);
        EXPECT(external_chain_is_bounded_set_max_external_events_per_call(&exactly, 6u), "a budget of six was refused");
        deliver(&exactly, EXTERNAL_CHAIN_IS_BOUNDED_EVENT_BOUNDED);
        EXPECT(counter_laps(&exactly) == 5, "the finite chain is the host's event and five laps");
        EXPECT(external_chain_is_bounded_truncated_event_chains(&exactly) == 0u,
               "a call that takes exactly the budget and empties the queue refused nothing: a "
               "long chain is not a runaway");
        external_chain_is_bounded_event_t none = EXTERNAL_CHAIN_IS_BOUNDED_EVENT_POKE;
        EXPECT(!external_chain_is_bounded_last_truncated_event(&exactly, &none),
               "nothing was cut, so there is no event to name");

        external_chain_is_bounded_t one_short;
        started(&one_short);
        EXPECT(external_chain_is_bounded_set_max_external_events_per_call(&one_short, 5u),
               "a budget of five was refused");
        deliver(&one_short, EXTERNAL_CHAIN_IS_BOUNDED_EVENT_BOUNDED);
        EXPECT(counter_laps(&one_short) == 4, "one lap was left queued");
        EXPECT(external_chain_is_bounded_truncated_event_chains(&one_short) == 1u,
               "the call was one event short and said so");
        external_chain_is_bounded_event_t cut = EXTERNAL_CHAIN_IS_BOUNDED_EVENT_POKE;
        EXPECT(external_chain_is_bounded_last_truncated_event(&one_short, &cut) &&
                   cut == EXTERNAL_CHAIN_IS_BOUNDED_EVENT_LAP,
               "the cut left a lap queued");
    }

    // What the refusal did with the events it would not take: it left them
    // queued. An engine that dropped the queue stops short and never finishes;
    // one that ran the chain anyway finishes it in the first call.
    {
        external_chain_is_bounded_t sm;
        started(&sm);
        EXPECT(external_chain_is_bounded_set_max_external_events_per_call(&sm, 20u), "a budget of twenty was refused");

        deliver(&sm, EXTERNAL_CHAIN_IS_BOUNDED_EVENT_RESUME);
        EXPECT(external_chain_is_bounded_truncated_event_chains(&sm) == 1u, "the first call was cut");
        EXPECT(counter_beats(&sm) == 19, "the host's event and nineteen beats (got %lld)",
               (long long)counter_beats(&sm));

        deliver(&sm, EXTERNAL_CHAIN_IS_BOUNDED_EVENT_POKE);
        EXPECT(counter_beats(&sm) == 30,
               "the second call took the beats the first left on the queue, each in a budget of "
               "its own, and finished (got %lld)",
               (long long)counter_beats(&sm));
        EXPECT(counter_pokes(&sm) == 1, "and the host's second event was heard");
        EXPECT(external_chain_is_bounded_truncated_event_chains(&sm) == 1u,
               "the second call ended the way the clause says: nothing more is counted");
    }

    // delay="0ms" is due at the instant being processed. This engine hands it to
    // its scheduler, so a tick pops the entry, its handler arms another due at
    // the same reading, and the tick that is popping finds it. Each pass takes
    // one event, so a budget on the drain alone never trips - this is the case
    // ARCHITECTURE.md rule 5 exists for. This block returning at all is the
    // assertion.
    {
        external_chain_is_bounded_t sm;
        started(&sm);
        EXPECT(external_chain_is_bounded_set_max_external_events_per_call(&sm, 50u), "a budget of fifty was refused");

        deliver(&sm, EXTERNAL_CHAIN_IS_BOUNDED_EVENT_ZERO);
        EXPECT(external_chain_is_bounded_truncated_event_chains(&sm) == 0u,
               "entering the state arms one entry and pops none: nothing has been cut yet");

        external_chain_is_bounded_tick(&sm);

        EXPECT(external_chain_is_bounded_truncated_event_chains(&sm) == 1u,
               "the tick popped entries due at its own reading until the budget, left the due one "
               "waiting and said so");
        EXPECT(counter_blinks(&sm) == 50, "the budget of pops at one reading (got %lld)",
               (long long)counter_blinks(&sm));
        external_chain_is_bounded_event_t cut = EXTERNAL_CHAIN_IS_BOUNDED_EVENT_POKE;
        EXPECT(external_chain_is_bounded_last_truncated_event(&sm, &cut) &&
                   cut == EXTERNAL_CHAIN_IS_BOUNDED_EVENT_BLINK,
               "the cut left a blink due");
    }

    // The same chain through delayexpr="'0ms'", which has no static value an
    // engine could read as undelayed.
    {
        external_chain_is_bounded_t sm;
        started(&sm);
        EXPECT(external_chain_is_bounded_set_max_external_events_per_call(&sm, 50u), "a budget of fifty was refused");

        deliver(&sm, EXTERNAL_CHAIN_IS_BOUNDED_EVENT_ZERO_EXPR);
        external_chain_is_bounded_tick(&sm);

        EXPECT(external_chain_is_bounded_truncated_event_chains(&sm) == 1u,
               "the tick popped entries due at its own reading until the budget and said so");
        EXPECT(counter_exprs(&sm) == 50, "the budget of pops at one reading (got %lld)", (long long)counter_exprs(&sm));
    }

    // Eight pulses, each due one millisecond after the last. They are due at
    // later instants, so a legitimate time-driven workload is not a runaway
    // however small the budget: three here, against eight events.
    {
        external_chain_is_bounded_t walked;
        started(&walked);
        EXPECT(external_chain_is_bounded_set_max_external_events_per_call(&walked, 3u),
               "a budget of three was refused");
        deliver(&walked, EXTERNAL_CHAIN_IS_BOUNDED_EVENT_TIMED);
        for (int i = 0; i < 8; ++i) {
            external_chain_is_bounded_advance_time_ms(&walked, 1u);
        }
        EXPECT(counter_pulses(&walked) == 8, "every pulse was heard (got %lld)", (long long)counter_pulses(&walked));
        EXPECT(external_chain_is_bounded_truncated_event_chains(&walked) == 0u,
               "each pulse came in a tick of its own, at an instant of its own");

        external_chain_is_bounded_t jumped;
        started(&jumped);
        EXPECT(external_chain_is_bounded_set_max_external_events_per_call(&jumped, 3u),
               "a budget of three was refused");
        deliver(&jumped, EXTERNAL_CHAIN_IS_BOUNDED_EVENT_TIMED);
        external_chain_is_bounded_advance_time_ms(&jumped, 8u);
        EXPECT(counter_pulses(&jumped) == 8, "every pulse was heard in one jump (got %lld)",
               (long long)counter_pulses(&jumped));
        EXPECT(external_chain_is_bounded_truncated_event_chains(&jumped) == 0u,
               "eight entries came due on the way to one reading, seven of them at earlier "
               "instants: a clock that moved a long way is bounded by how far it moved, and is "
               "not a chain that does not end");
    }

    // A host chooses the budget, and a budget that takes no event is refused and
    // changes nothing.
    {
        external_chain_is_bounded_t sm;
        started(&sm);
        EXPECT(external_chain_is_bounded_set_max_external_events_per_call(&sm, 7u), "a budget of seven was refused");
        EXPECT(external_chain_is_bounded_max_external_events_per_call(&sm) == 7u, "the budget the host chose is seven");
        EXPECT(!external_chain_is_bounded_set_max_external_events_per_call(&sm, 0u),
               "a budget of zero takes no event and was accepted");
        EXPECT(external_chain_is_bounded_max_external_events_per_call(&sm) == 7u, "a refused budget changes nothing");
    }

    return failures;
}
