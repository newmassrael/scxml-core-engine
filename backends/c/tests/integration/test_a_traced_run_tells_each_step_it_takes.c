// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The trace port (docs/adr/0115, D4 and D8) -- C11 AOT, on a manual clock.
//
// This target builds a_computed_event_name_is_matched_like_any_other with
// `SCE_TRACE_PORT=1` and `SCE_TRANSITION_OBSERVER=1`, so the machine calls both
// hooks below. The document takes six transitions, one out of each of s1 to s6,
// each on an event whose name a `<send>` computed: three off its external queue
// (s1's now, s2's after a delay, s6's addressed to this session by its id) and
// three off its internal queue (s3's now, s4's after a delay, s5's to a target
// it computed). So the run is a start, six event steps and a stop, and this
// holds what the trace port is told:
//
//   * the steps in that order, each event step of the kind its queue is and
//     naming the whole name its event was sent under, which is not the name of
//     the event of the document it matched (`request`, or the wildcard);
//   * each event step's transition, the state it exited, the state it entered
//     and the configuration it left;
//   * the transitions the steps took are the ones the transition observer is
//     told, in the order it is told them: the two hooks are told one run;
//   * the start enters s1, and the stop exits `pass` and names it the run's
//     outcome.
//
// The W3C documents docs/formal has models of take no external event, so this
// is the target that holds the external step. The same machine built with
// neither switch is test_a_computed_event_name_is_matched_like_any_other.c.
//
// Fixture:
// integration_resources/a_computed_event_name_is_matched_like_any_other/a_computed_event_name_is_matched_like_any_other.scxml

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include <sce/trace_json.h>
#include <sce/trace_port.h>

#include "../require_started.h"
#include "a_computed_event_name_is_matched_like_any_other_sm.h"

#if !A_COMPUTED_EVENT_NAME_IS_MATCHED_LIKE_ANY_OTHER_TRACE_PORT
#error "this target is built with SCE_TRACE_PORT=1 (backends/c/tests/CMakeLists.txt)"
#endif
#if !A_COMPUTED_EVENT_NAME_IS_MATCHED_LIKE_ANY_OTHER_TRANSITION_OBSERVER
#error "this target is built with SCE_TRANSITION_OBSERVER=1 (backends/c/tests/CMakeLists.txt)"
#endif

#define TOLD_MAX 16u
#define EVENT_NAME_CAP 64u

// One step as the hook was told it. The names of states are the machine's own,
// which live as long as the program; the event's name is the machine's only for
// the length of the call, so it is copied.
typedef struct {
    sce_trace_kind_t kind;
    char event[EVENT_NAME_CAP];
    bool has_event;
    size_t fired_count;
    sce_trace_fired_t fired;
    size_t exited_count;
    const char *exited;
    size_t entered_count;
    const char *entered;
    size_t configuration_count;
    const char *configuration;
    const char *outcome;
    bool written;
} told_step_t;

static told_step_t told[TOLD_MAX];
static size_t told_count;
static const char *observed_source[TOLD_MAX];
static uint32_t observed_index[TOLD_MAX];
static size_t observed_count;
static const a_computed_event_name_is_matched_like_any_other_t *traced_machine;
static int told_another_machine;

void a_computed_event_name_is_matched_like_any_other_trace_step(
    const a_computed_event_name_is_matched_like_any_other_t *sm, const sce_trace_step_t *step) {
    if (sm != traced_machine) {
        told_another_machine = 1;
    }
    if (told_count < TOLD_MAX) {
        told_step_t *kept = &told[told_count];
        kept->kind = step->kind;
        kept->has_event = step->event != NULL;
        (void)snprintf(kept->event, sizeof(kept->event), "%s", step->event != NULL ? step->event : "");
        kept->fired_count = step->fired_count;
        if (step->fired_count > 0u) {
            kept->fired = step->fired[0];
        }
        kept->exited_count = step->exited_count;
        kept->exited = step->exited_count > 0u ? step->exited[0] : NULL;
        kept->entered_count = step->entered_count;
        kept->entered = step->entered_count > 0u ? step->entered[0] : NULL;
        kept->configuration_count = step->configuration_count;
        kept->configuration = step->configuration_count > 0u ? step->configuration[0] : NULL;
        kept->outcome = step->outcome;
        // Every step the port tells is one the writer writes whole.
        char text[1024];
        sce_jw_t w;
        sce_jw_init(&w, text, sizeof(text));
        kept->written = sce_trace_json_step(&w, step) && sce_jw_finish(&w);
    }
    ++told_count;
}

void a_computed_event_name_is_matched_like_any_other_observe_transition(
    const a_computed_event_name_is_matched_like_any_other_t *sm,
    a_computed_event_name_is_matched_like_any_other_state_t source, uint32_t index) {
    if (sm != traced_machine) {
        told_another_machine = 1;
    }
    if (observed_count < TOLD_MAX) {
        observed_source[observed_count] = a_computed_event_name_is_matched_like_any_other_state_name(source);
        observed_index[observed_count] = index;
    }
    ++observed_count;
}

static int same(const char *a, const char *b) {
    return a != NULL && b != NULL && strcmp(a, b) == 0;
}

int main(void) {
    a_computed_event_name_is_matched_like_any_other_t sm;
    traced_machine = &sm;
    REQUIRE_STARTED(a_computed_event_name_is_matched_like_any_other_init_with_clock(&sm, sce_clock_manual(0u)),
                    A_COMPUTED_EVENT_NAME_IS_MATCHED_LIKE_ANY_OTHER_START_OK);
    for (int elapsed = 0; elapsed < 1000 && !a_computed_event_name_is_matched_like_any_other_is_in_final_state(&sm);
         elapsed += 10) {
        a_computed_event_name_is_matched_like_any_other_advance_time_ms(&sm, 10u);
    }

    int bad = 0;
    if (!a_computed_event_name_is_matched_like_any_other_ended_in(
            &sm, A_COMPUTED_EVENT_NAME_IS_MATCHED_LIKE_ANY_OTHER_STATE_PASS)) {
        (void)fprintf(stderr, "trace_port: FAIL - the traced run did not end in `pass`\n");
        bad = 1;
    }
    if (told_another_machine) {
        (void)fprintf(stderr, "trace_port: FAIL - a hook was handed another machine than the one run\n");
        bad = 1;
    }

    // The event steps, in the order the run takes them: the queue each event
    // comes off, the name it was sent under, and the transition it takes.
    static const struct {
        sce_trace_kind_t kind;
        const char *event;
        const char *source;
        const char *target;
    } expected[] = {
        {SCE_TRACE_EXTERNAL, "request.new", "s1", "s2"},   {SCE_TRACE_EXTERNAL, "other.thing", "s2", "s3"},
        {SCE_TRACE_INTERNAL, "request.again", "s3", "s4"}, {SCE_TRACE_INTERNAL, "last.one", "s4", "s5"},
        {SCE_TRACE_INTERNAL, "request.third", "s5", "s6"}, {SCE_TRACE_EXTERNAL, "last.two", "s6", "pass"},
    };

    const size_t expected_events = sizeof(expected) / sizeof(expected[0]);
    // A start, the event steps, and a stop.
    const size_t expected_count = expected_events + 2u;
    if (told_count != expected_count) {
        (void)fprintf(stderr, "trace_port: FAIL - the port was told %zu steps, not %zu\n", told_count, expected_count);
        bad = 1;
    } else {
        const told_step_t *start = &told[0];
        if (start->kind != SCE_TRACE_START || start->has_event || start->fired_count != 0u ||
            start->exited_count != 0u || start->entered_count != 1u || !same(start->entered, "s1") ||
            start->configuration_count != 1u || !same(start->configuration, "s1")) {
            (void)fprintf(stderr, "trace_port: FAIL - the first step is no start that enters s1 alone\n");
            bad = 1;
        }
        for (size_t i = 0u; i < expected_events; ++i) {
            const told_step_t *step = &told[i + 1u];
            if (step->kind != expected[i].kind || !step->has_event || strcmp(step->event, expected[i].event) != 0) {
                (void)fprintf(stderr, "trace_port: FAIL - step %zu is a %s step of `%s`, not a %s step of `%s`\n",
                              i + 1u, sce_trace_kind_name(step->kind), step->event,
                              sce_trace_kind_name(expected[i].kind), expected[i].event);
                bad = 1;
            }
            if (step->fired_count != 1u || !same(step->fired.source, expected[i].source) || step->fired.index != 0u) {
                (void)fprintf(stderr, "trace_port: FAIL - step %zu does not take the transition of %s\n", i + 1u,
                              expected[i].source);
                bad = 1;
            }
            if (step->exited_count != 1u || !same(step->exited, expected[i].source) || step->entered_count != 1u ||
                !same(step->entered, expected[i].target) || step->configuration_count != 1u ||
                !same(step->configuration, expected[i].target)) {
                (void)fprintf(stderr, "trace_port: FAIL - step %zu does not leave %s for %s alone\n", i + 1u,
                              expected[i].source, expected[i].target);
                bad = 1;
            }
        }
        const told_step_t *stop = &told[expected_count - 1u];
        if (stop->kind != SCE_TRACE_STOP || stop->has_event || stop->fired_count != 0u || stop->exited_count != 1u ||
            !same(stop->exited, "pass") || stop->entered_count != 0u || stop->configuration_count != 0u ||
            !same(stop->outcome, "pass")) {
            (void)fprintf(stderr, "trace_port: FAIL - the last step is no stop that exits `pass` and ends in it\n");
            bad = 1;
        }
        for (size_t i = 0u; i < expected_count; ++i) {
            if (!told[i].written) {
                (void)fprintf(stderr, "trace_port: FAIL - step %zu is not written whole\n", i);
                bad = 1;
            }
        }
    }

    // The transitions the steps took, in the order they took them, are the ones
    // the transition observer is told.
    if (observed_count != expected_events) {
        (void)fprintf(stderr, "trace_port: FAIL - the observer was told %zu transitions, not %zu\n", observed_count,
                      expected_events);
        bad = 1;
    } else if (told_count == expected_count) {
        for (size_t i = 0u; i < expected_events; ++i) {
            const told_step_t *step = &told[i + 1u];
            if (!same(step->fired.source, observed_source[i]) || step->fired.index != observed_index[i]) {
                (void)fprintf(stderr, "trace_port: FAIL - step %zu took (%s, %u), and the observer was told (%s, %u)\n",
                              i + 1u, step->fired.source != NULL ? step->fired.source : "", (unsigned)step->fired.index,
                              observed_source[i] != NULL ? observed_source[i] : "", (unsigned)observed_index[i]);
                bad = 1;
            }
        }
    }

    a_computed_event_name_is_matched_like_any_other_destroy(&sm);
    return bad;
}
