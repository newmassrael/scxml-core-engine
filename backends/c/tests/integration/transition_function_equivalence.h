// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A generated machine held to the transition function generated beside it
// (docs/SCE_TRANSITION_FUNCTION.md).
//
// The function is computed at build time by the selection rule the engine
// follows at run time, so the two are one fact only if every pair agrees, and
// this is what asks: a fresh machine per pair, driven to the state by the
// shortest event path the function itself gives, then sent the event. What the
// engine reaches, and whether it took the event or discarded it, must be what
// the function says.
//
// C has no generics, so a driver hands the drive two tables of functions: the
// generated function's accessors with its enumerators read as ints, and the
// questions the drive asks of the machine. The drivers of the integration stems
// that declare a function share this, so the two stems ask the same question
// the same way.

#ifndef SCE_C_TESTS_TRANSITION_FUNCTION_EQUIVALENCE_H
#define SCE_C_TESTS_TRANSITION_FUNCTION_EQUIVALENCE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

/* The most states a function this drive holds may have: the drive keeps its
   search in arrays of this size rather than allocating. */
#define TF_EQUIVALENCE_MAX_STATES 64

/* A generated transition function, its enumerators read as ints. */
typedef struct {
    int state_count;
    int event_count;
    int initial;
    const char *(*state_name)(int state);
    const char *(*event_name)(int event);
    bool (*transition)(int state, int event, int *next);
    bool (*is_final)(int state);
    /* The configuration's ids, written through `ids`; returns how many. */
    size_t (*configuration)(int state, const char *const **ids);
} tf_equivalence_function_t;

/* What the drive asks of the engine it holds to a function. */
typedef struct {
    /* A fresh machine at its initial configuration, replacing the last one. */
    void (*start)(void);
    /* Release the machine `start` made. */
    void (*stop)(void);
    /* Deliver an event by name and run the machine to completion. */
    void (*send)(const char *event);
    /* Whether the active states are exactly the `count` ids given. */
    bool (*holds)(const char *const *ids, size_t count);
    /* The id of the final the run ended in, or "" while it is running. */
    const char *(*terminal)(void);
    /* How many events the machine has discarded. */
    uint32_t (*discarded)(void);
} tf_equivalence_engine_t;

static int tf_equivalence_failures;

static void tf_equivalence_expect(bool ok, const char *what, const char *state, const char *event) {
    if (!ok) {
        (void)fprintf(stderr, "MISMATCH (%s, %s): %s\n", state, event, what);
        tf_equivalence_failures++;
    }
}

static bool tf_equivalence_holds(const tf_equivalence_function_t *function, const tf_equivalence_engine_t *engine,
                                 int state) {
    const char *const *ids = NULL;
    const size_t count = function->configuration(state, &ids);
    return engine->holds(ids, count);
}

/* The shortest sequence of events from the initial state to `target`, read
   from the function alone, written to `path`; its length, or -1. */
static int tf_equivalence_shortest_path(const tf_equivalence_function_t *function, int target, int *path) {
    int frontier[TF_EQUIVALENCE_MAX_STATES];
    int parent[TF_EQUIVALENCE_MAX_STATES];
    int via[TF_EQUIVALENCE_MAX_STATES];
    bool seen[TF_EQUIVALENCE_MAX_STATES];
    int head = 0;
    int tail = 0;
    memset(seen, 0, sizeof seen);
    frontier[tail] = function->initial;
    parent[tail] = -1;
    via[tail] = -1;
    tail++;
    seen[function->initial] = true;
    while (head < tail) {
        const int at = head++;
        if (frontier[at] == target) {
            int length = 0;
            for (int k = at; parent[k] >= 0; k = parent[k]) {
                length++;
            }
            int position = length;
            for (int k = at; parent[k] >= 0; k = parent[k]) {
                path[--position] = via[k];
            }
            return length;
        }
        for (int event = 0; event < function->event_count; ++event) {
            int to = frontier[at];
            if (function->transition(frontier[at], event, &to) && !seen[to]) {
                seen[to] = true;
                frontier[tail] = to;
                parent[tail] = at;
                via[tail] = event;
                tail++;
            }
        }
    }
    return -1;
}

/* Drive the engine to every state of the function by the shortest path the
   function gives, send it every event there, and count every pair where the
   configuration it reaches, or whether it took the event or discarded it, is
   not what the function says. The configuration is compared as a set of ids:
   a state's ancestors are fixed by the document, so the set says what the
   ordered list says. Prints how many pairs it checked; returns the process
   status a driver exits with. */
static int tf_equivalence_run(const tf_equivalence_function_t *function, const tf_equivalence_engine_t *engine) {
    int pairs = 0;
    if (function->state_count > TF_EQUIVALENCE_MAX_STATES) {
        (void)fprintf(stderr, "FAIL: %d states, more than the drive holds\n", function->state_count);
        return 1;
    }
    engine->start();
    tf_equivalence_expect(tf_equivalence_holds(function, engine, function->initial),
                          "the machine does not start where the function does", "", "");
    engine->stop();
    for (int state = 0; state < function->state_count; ++state) {
        const char *state_name = function->state_name(state);
        int path[TF_EQUIVALENCE_MAX_STATES];
        const int length = tf_equivalence_shortest_path(function, state, path);
        if (length < 0) {
            tf_equivalence_expect(false, "the state is not reached from the initial state", state_name, "");
            continue;
        }
        for (int event = 0; event < function->event_count; ++event) {
            const char *event_name = function->event_name(event);
            engine->start();
            for (int k = 0; k < length; ++k) {
                engine->send(function->event_name(path[k]));
            }
            pairs++;
            if (function->is_final(state)) {
                tf_equivalence_expect(strcmp(engine->terminal(), state_name) == 0,
                                      "the path does not end the machine there", state_name, event_name);
                engine->stop();
                continue;
            }
            tf_equivalence_expect(tf_equivalence_holds(function, engine, state),
                                  "the path leaves the machine elsewhere", state_name, event_name);
            const uint32_t discarded = engine->discarded();
            engine->send(event_name);
            int to = state;
            if (!function->transition(state, event, &to)) {
                tf_equivalence_expect(engine->discarded() == discarded + 1u,
                                      "the function refuses it and the machine took it", state_name, event_name);
                tf_equivalence_expect(tf_equivalence_holds(function, engine, state),
                                      "the machine moved on an event it discarded", state_name, event_name);
            } else {
                tf_equivalence_expect(engine->discarded() == discarded,
                                      "the function takes it and the machine discarded it", state_name, event_name);
                if (function->is_final(to)) {
                    tf_equivalence_expect(strcmp(engine->terminal(), function->state_name(to)) == 0,
                                          "the machine did not end where the function says", state_name, event_name);
                } else {
                    tf_equivalence_expect(tf_equivalence_holds(function, engine, to),
                                          "the machine is not where the function says", state_name, event_name);
                }
            }
            engine->stop();
        }
    }
    const int expected = function->state_count * function->event_count;
    if (pairs != expected) {
        (void)fprintf(stderr, "FAIL: the drive checked %d pairs, want %d\n", pairs, expected);
        return 1;
    }
    if (tf_equivalence_failures > 0) {
        (void)fprintf(stderr, "FAIL: %d disagreement(s) between the machine and its function\n",
                      tf_equivalence_failures);
        return 1;
    }
    printf("PASS: the machine and its transition function agree on all %d pairs\n", pairs);
    return 0;
}

#endif /* SCE_C_TESTS_TRANSITION_FUNCTION_EQUIVALENCE_H */
