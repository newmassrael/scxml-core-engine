// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A statechart declared as a transition function answers as its machine does
// (docs/SCE_TRANSITION_FUNCTION.md) — C11 AOT, the flat lifecycle.
//
// The document declares `sce:transition-function="emit"`, so its machine is
// generated with a pure function of (state, event) beside it, as the header
// `flat_lifecycle_transition_function.h`, every symbol under its own prefix. The
// function was computed at build time by the selection rule the engine follows
// at run time; this drives the machine to every state by the function's
// shortest path, sends every event there, and holds what the engine does to
// what the function says, pair by pair. The two headers in one translation unit
// are also what shows the function's names stay clear of the machine's.
//
// Fixture: integration_resources/flat_lifecycle/flat_lifecycle.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include "flat_lifecycle_sm.h"
#include "flat_lifecycle_transition_function.h"
#include "transition_function_equivalence.h"

static flat_lifecycle_t sm;

static void machine_start(void) {
    flat_lifecycle_init(&sm);
}

static void machine_stop(void) {
    flat_lifecycle_destroy(&sm);
}

static void machine_send(const char *name) {
    flat_lifecycle_event_t member = (flat_lifecycle_event_t)0;
    if (!flat_lifecycle_resolve_event_by_name(name, &member)) {
        return;
    }
    flat_lifecycle_event_with_meta_t carrier;
    memset(&carrier, 0, sizeof carrier);
    carrier.event = member;
    flat_lifecycle_raise_external(&sm, &carrier);
    flat_lifecycle_step(&sm);
}

static bool machine_holds(const char *const *ids, size_t count) {
    size_t active = 0;
    for (int s = 0; s < (int)FLAT_LIFECYCLE_STATE_COUNT; ++s) {
        if (flat_lifecycle_in_state(&sm, (flat_lifecycle_state_t)s)) {
            active++;
        }
    }
    if (active != count) {
        return false;
    }
    for (size_t k = 0; k < count; ++k) {
        flat_lifecycle_state_t state = (flat_lifecycle_state_t)0;
        if (!flat_lifecycle_state_from_name(ids[k], &state) || !flat_lifecycle_in_state(&sm, state)) {
            return false;
        }
    }
    return true;
}

static const char *machine_terminal(void) {
    flat_lifecycle_state_t state = (flat_lifecycle_state_t)0;
    return flat_lifecycle_terminal_state(&sm, &state) ? flat_lifecycle_state_name(state) : "";
}

static uint32_t machine_discarded(void) {
    return flat_lifecycle_discarded_external_events(&sm);
}

static const char *function_state_name(int state) {
    return flat_lifecycle_transition_function_state_name((flat_lifecycle_transition_function_state_t)state);
}

static const char *function_event_name(int event) {
    return flat_lifecycle_transition_function_event_name((flat_lifecycle_transition_function_event_t)event);
}

static bool function_transition(int state, int event, int *next) {
    flat_lifecycle_transition_function_state_t to = (flat_lifecycle_transition_function_state_t)state;
    if (!flat_lifecycle_transition_function_transition((flat_lifecycle_transition_function_state_t)state,
                                                       (flat_lifecycle_transition_function_event_t)event, &to)) {
        return false;
    }
    *next = (int)to;
    return true;
}

static bool function_is_final(int state) {
    return flat_lifecycle_transition_function_is_final((flat_lifecycle_transition_function_state_t)state);
}

static size_t function_configuration(int state, const char *const **ids) {
    const flat_lifecycle_transition_function_configuration_t configuration =
        flat_lifecycle_transition_function_configuration((flat_lifecycle_transition_function_state_t)state);
    *ids = configuration.ids;
    return configuration.count;
}

int main(void) {
    const tf_equivalence_function_t function = {
        .state_count = (int)FLAT_LIFECYCLE_TRANSITION_FUNCTION_NUM_STATES,
        .event_count = (int)FLAT_LIFECYCLE_TRANSITION_FUNCTION_NUM_EVENTS,
        .initial = (int)FLAT_LIFECYCLE_TRANSITION_FUNCTION_INITIAL,
        .state_name = function_state_name,
        .event_name = function_event_name,
        .transition = function_transition,
        .is_final = function_is_final,
        .configuration = function_configuration,
    };
    const tf_equivalence_engine_t engine = {
        .start = machine_start,
        .stop = machine_stop,
        .send = machine_send,
        .holds = machine_holds,
        .terminal = machine_terminal,
        .discarded = machine_discarded,
    };
    return tf_equivalence_run(&function, &engine);
}
