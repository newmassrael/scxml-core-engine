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
#include <stdlib.h>
#include <string.h>

#include "external_chain_is_bounded_sm.h"

#ifndef SCE_EXTERNAL_EVENT_BUDGET_TABLE
#error "SCE_EXTERNAL_EVENT_BUDGET_TABLE names tests/external_event_budget/budget_outcomes.json; the build defines it"
#endif

// The default and every budget and counter below come from
// tests/external_event_budget/budget_outcomes.json, read at run time: the table
// states them once for every engine, and a test that asked the engine for its
// own limit would agree with any limit, including one an edit moved by three
// orders of magnitude.
//
// The table is read by a scan for the key inside its section's object: the
// document is a fixed shape of nested objects with integer values and one array
// of integers. A key the scan does not find ends the run, because a number this
// test fell back to would be a literal again.
static char *table_text = NULL;

static _Noreturn void table_die(const char *section, const char *key, const char *why) {
    (void)fprintf(stderr, "external_chain_is_bounded: FAIL - %s: \"%s\" in \"%s\" of the table %s\n", why, key,
                  section != NULL ? section : "(top level)", SCE_EXTERNAL_EVENT_BUDGET_TABLE);
    exit(1);
}

static void table_load(void) {
    FILE *f = fopen(SCE_EXTERNAL_EVENT_BUDGET_TABLE, "rb");
    if (f == NULL) {
        table_die(NULL, "(file)", "cannot open");
    }
    if (fseek(f, 0, SEEK_END) != 0) {
        (void)fclose(f);
        table_die(NULL, "(file)", "cannot size");
    }
    const long size = ftell(f);
    rewind(f);
    table_text = size > 0 ? malloc((size_t)size + 1u) : NULL;
    if (table_text == NULL || fread(table_text, 1, (size_t)size, f) != (size_t)size) {
        (void)fclose(f);
        table_die(NULL, "(file)", "cannot read");
    }
    table_text[size] = '\0';
    (void)fclose(f);
}

// The text after the `"key":` that follows `from`, before `end` (NULL: the end of
// the table), or NULL when the key is not there. `"key"` is matched with its quotes
// and followed by a colon, so a word of the table's prose cannot stand for it.
static const char *table_member(const char *from, const char *end, const char *key) {
    char needle[64];
    (void)snprintf(needle, sizeof(needle), "\"%s\"", key);
    for (const char *p = strstr(from, needle); p != NULL && (end == NULL || p < end); p = strstr(p + 1, needle)) {
        const char *q = p + strlen(needle);
        while (*q == ' ' || *q == '\n' || *q == '\t') {
            q++;
        }
        if (*q == ':') {
            return q + 1;
        }
    }
    return NULL;
}

// The object of `section` (NULL: the table itself) as the text after its `{`, and the
// position of its `}` in `*end`. The objects of this table nest nothing, so the first
// `}` closes one.
static const char *table_section(const char *section, const char **end) {
    if (section == NULL) {
        *end = NULL;
        return table_text;
    }
    const char *open = table_member(table_text, NULL, section);
    open = open != NULL ? strchr(open, '{') : NULL;
    const char *close = open != NULL ? strchr(open, '}') : NULL;
    if (close == NULL) {
        table_die(NULL, section, "no such object");
    }
    *end = close;
    return open + 1;
}

// An integer of the table: `key` of `section`.
static long long table_value(const char *section, const char *key) {
    const char *end = NULL;
    const char *body = table_section(section, &end);
    const char *at = table_member(body, end, key);
    if (at == NULL) {
        table_die(section, key, "missing key");
    }
    char *stop = NULL;
    const long long v = strtoll(at, &stop, 10);
    if (stop == at) {
        table_die(section, key, "not an integer");
    }
    return v;
}

// A budget or a count of the table: never negative, and it fits the engine's
// unsigned type.
static uint32_t table_u32(const char *section, const char *key) {
    const long long v = table_value(section, key);
    if (v < 0 || v > (long long)UINT32_MAX) {
        table_die(section, key, "out of range");
    }
    return (uint32_t)v;
}

// The integers of the array `key` of `section`, written into `out` (room for `cap`);
// how many there are.
static size_t table_array(const char *section, const char *key, long long *out, size_t cap) {
    const char *end = NULL;
    const char *body = table_section(section, &end);
    const char *at = table_member(body, end, key);
    at = at != NULL ? strchr(at, '[') : NULL;
    const char *close = at != NULL ? strchr(at, ']') : NULL;
    if (close == NULL) {
        table_die(section, key, "no such array");
    }
    size_t n = 0;
    const char *p = at + 1;
    while (p < close) {
        char *stop = NULL;
        const long long v = strtoll(p, &stop, 10);
        if (stop == p) {
            break;
        }
        if (n == cap) {
            table_die(section, key, "array longer than the test reads");
        }
        out[n++] = v;
        p = stop;
        while (p < close && (*p == ',' || *p == ' ' || *p == '\n' || *p == '\t')) {
            p++;
        }
    }
    if (n == 0) {
        table_die(section, key, "empty array");
    }
    return n;
}

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
    table_load();
    const uint32_t default_budget = table_u32(NULL, "default_budget");

    // The default budget is the documented one, and nothing has been cut before
    // the machine has done anything. The lower bound is the half a check like
    // this usually forgets: an accessor that always answers reads exactly like a
    // working one from the other side, so the head of the queue is asked too.
    {
        external_chain_is_bounded_t sm;
        started(&sm);
        EXPECT(external_chain_is_bounded_max_external_events_per_call(&sm) == default_budget,
               "the default budget is %u, got %u", default_budget,
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

        EXPECT(external_chain_is_bounded_truncated_event_chains(&sm) == table_u32("spin", "truncated"),
               "the call handed control back with an event still queued, and said so; without "
               "the count the host sees a machine that is running and has returned");
        // The host's own event is the first of the invocation, so the budget buys
        // the host's event and then budget - 1 links.
        EXPECT(counter_links(&sm) == (int64_t)default_budget - 1,
               "the chain must run exactly as far as the budget allows (want %lld, got %lld)",
               (long long)default_budget - 1, (long long)counter_links(&sm));
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
        EXPECT(external_chain_is_bounded_set_max_external_events_per_call(&exactly, table_u32("exact", "budget")),
               "the budget that takes the whole finite chain was refused");
        deliver(&exactly, EXTERNAL_CHAIN_IS_BOUNDED_EVENT_BOUNDED);
        EXPECT(counter_laps(&exactly) == table_value("exact", "laps"),
               "the finite chain is the host's event and the laps the table counts (got %lld)",
               (long long)counter_laps(&exactly));
        EXPECT(external_chain_is_bounded_truncated_event_chains(&exactly) == table_u32("exact", "truncated"),
               "a call that takes exactly the budget and empties the queue refused nothing: a "
               "long chain is not a runaway");
        external_chain_is_bounded_event_t none = EXTERNAL_CHAIN_IS_BOUNDED_EVENT_POKE;
        EXPECT(!external_chain_is_bounded_last_truncated_event(&exactly, &none),
               "nothing was cut, so there is no event to name");

        external_chain_is_bounded_t one_short;
        started(&one_short);
        EXPECT(external_chain_is_bounded_set_max_external_events_per_call(&one_short, table_u32("one_short", "budget")),
               "the budget one event short of the chain was refused");
        deliver(&one_short, EXTERNAL_CHAIN_IS_BOUNDED_EVENT_BOUNDED);
        EXPECT(counter_laps(&one_short) == table_value("one_short", "laps"), "one lap was left queued (got %lld)",
               (long long)counter_laps(&one_short));
        EXPECT(external_chain_is_bounded_truncated_event_chains(&one_short) == table_u32("one_short", "truncated"),
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
        EXPECT(external_chain_is_bounded_set_max_external_events_per_call(&sm, table_u32("resume", "budget")),
               "the budget of the resumed chain was refused");

        deliver(&sm, EXTERNAL_CHAIN_IS_BOUNDED_EVENT_RESUME);
        EXPECT(external_chain_is_bounded_truncated_event_chains(&sm) == table_u32("resume", "truncated"),
               "the first call was cut");
        EXPECT(counter_beats(&sm) == table_value("resume", "beats_after_first_call"),
               "the host's event and the beats the budget leaves room for (got %lld)", (long long)counter_beats(&sm));

        deliver(&sm, EXTERNAL_CHAIN_IS_BOUNDED_EVENT_POKE);
        EXPECT(counter_beats(&sm) == table_value("resume", "beats_after_second_call"),
               "the second call took the beats the first left on the queue, each in a budget of "
               "its own, and finished (got %lld)",
               (long long)counter_beats(&sm));
        EXPECT(counter_pokes(&sm) == table_value("resume", "pokes"), "and the host's second event was heard");
        EXPECT(external_chain_is_bounded_truncated_event_chains(&sm) == table_u32("resume", "truncated"),
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
        const uint32_t budget = table_u32("static_zero", "budget");
        EXPECT(external_chain_is_bounded_set_max_external_events_per_call(&sm, budget),
               "the budget of the static-zero chain was refused");

        deliver(&sm, EXTERNAL_CHAIN_IS_BOUNDED_EVENT_ZERO);
        EXPECT(external_chain_is_bounded_truncated_event_chains(&sm) == 0u,
               "entering the state arms one entry and pops none: nothing has been cut yet");

        external_chain_is_bounded_tick(&sm);

        EXPECT(external_chain_is_bounded_truncated_event_chains(&sm) == 1u,
               "the tick popped entries due at its own reading until the budget, left the due one "
               "waiting and said so");
        EXPECT(counter_blinks(&sm) == (int64_t)budget, "the budget of pops at one reading (got %lld)",
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
        EXPECT(external_chain_is_bounded_set_max_external_events_per_call(&sm, table_u32("zero_expr", "budget")),
               "the budget of the delay-expression chain was refused");

        deliver(&sm, EXTERNAL_CHAIN_IS_BOUNDED_EVENT_ZERO_EXPR);
        EXPECT(external_chain_is_bounded_truncated_event_chains(&sm) == table_u32("zero_expr", "truncated_after_entry"),
               "entering the state pops nothing: nothing has been cut yet");
        external_chain_is_bounded_tick(&sm);

        EXPECT(external_chain_is_bounded_truncated_event_chains(&sm) == table_u32("zero_expr", "truncated_after_tick"),
               "the tick popped entries due at its own reading until the budget and said so");
        EXPECT(counter_exprs(&sm) == table_value("zero_expr", "exprs"), "the budget of pops at one reading (got %lld)",
               (long long)counter_exprs(&sm));
    }

    // Eight pulses, each due one millisecond after the last. They are due at
    // later instants, so a legitimate time-driven workload is not a runaway
    // however small the budget: three here, against eight events.
    {
        const uint32_t timed_budget = table_u32("timed", "budget");
        const uint32_t pulses = table_u32("timed", "pulses");

        external_chain_is_bounded_t walked;
        started(&walked);
        EXPECT(external_chain_is_bounded_set_max_external_events_per_call(&walked, timed_budget),
               "the budget of the timed chain was refused");
        deliver(&walked, EXTERNAL_CHAIN_IS_BOUNDED_EVENT_TIMED);
        for (uint32_t i = 0; i < pulses; ++i) {
            external_chain_is_bounded_advance_time_ms(&walked, 1u);
        }
        EXPECT(counter_pulses(&walked) == (int64_t)pulses, "every pulse was heard (got %lld)",
               (long long)counter_pulses(&walked));
        EXPECT(external_chain_is_bounded_truncated_event_chains(&walked) == table_u32("timed", "truncated"),
               "each pulse came in a tick of its own, at an instant of its own");

        external_chain_is_bounded_t jumped;
        started(&jumped);
        EXPECT(external_chain_is_bounded_set_max_external_events_per_call(&jumped, timed_budget),
               "the budget of the timed chain was refused");
        deliver(&jumped, EXTERNAL_CHAIN_IS_BOUNDED_EVENT_TIMED);
        external_chain_is_bounded_advance_time_ms(&jumped, pulses);
        EXPECT(counter_pulses(&jumped) == (int64_t)pulses, "every pulse was heard in one jump (got %lld)",
               (long long)counter_pulses(&jumped));
        EXPECT(external_chain_is_bounded_truncated_event_chains(&jumped) == table_u32("timed", "truncated"),
               "eight entries came due on the way to one reading, seven of them at earlier "
               "instants: a clock that moved a long way is bounded by how far it moved, and is "
               "not a chain that does not end");
    }

    // A host chooses the budget, and a budget that takes no event is refused and
    // changes nothing.
    {
        external_chain_is_bounded_t sm;
        started(&sm);
        const uint32_t chosen = table_u32("host", "budget");
        EXPECT(external_chain_is_bounded_set_max_external_events_per_call(&sm, chosen),
               "the budget the host chose was refused");
        EXPECT(external_chain_is_bounded_max_external_events_per_call(&sm) == chosen, "the budget the host chose is %u",
               chosen);

        // The table's refused budgets include negative ones, which this engine's
        // unsigned parameter cannot spell: the ones it can are tried, and at least
        // one must be, so a table that lost them does not read as a pass.
        long long refused[8];
        const size_t refused_count = table_array("host", "refused", refused, sizeof(refused) / sizeof(refused[0]));
        size_t tried = 0;
        for (size_t i = 0; i < refused_count; ++i) {
            if (refused[i] < 0 || refused[i] > (long long)UINT32_MAX) {
                continue;
            }
            EXPECT(!external_chain_is_bounded_set_max_external_events_per_call(&sm, (uint32_t)refused[i]),
                   "a budget of %lld takes no event and was accepted", refused[i]);
            tried++;
        }
        EXPECT(tried > 0u, "no refused budget of the table fits the engine's unsigned type");
        EXPECT(external_chain_is_bounded_max_external_events_per_call(&sm) == chosen,
               "a refused budget changes nothing");
    }

    free(table_text);
    return failures;
}
