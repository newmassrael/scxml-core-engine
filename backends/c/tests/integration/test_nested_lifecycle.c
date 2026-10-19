// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A statechart declared as a transition function answers as its machine does
// (docs/SCE_TRANSITION_FUNCTION.md) — C11 AOT, the nested lifecycle.
//
// The document declares `sce:transition-function="emit"` and reaches every rule
// the function is computed by: transitions held by an ancestor, a compound
// state entered by its `initial` attribute, by an `<initial>` element and by its
// first child, an internal and an external transition, a prefix and a wildcard
// descriptor, a targetless transition and a top-level final. This drives the
// machine to every state by the function's shortest path, sends every event
// there, and holds what the engine does to what the function says, pair by
// pair. The two headers in one translation unit are also what shows the
// function's names stay clear of the machine's.
//
// Fixture: integration_resources/nested_lifecycle/nested_lifecycle.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include "nested_lifecycle_sm.h"
#include "nested_lifecycle_transition_function.h"
#include "transition_function_equivalence.h"

static nested_lifecycle_t sm;

static void machine_start(void) {
    nested_lifecycle_init(&sm);
}

static void machine_stop(void) {
    nested_lifecycle_destroy(&sm);
}

static void machine_send(const char *name) {
    nested_lifecycle_event_t member = (nested_lifecycle_event_t)0;
    if (!nested_lifecycle_resolve_event_by_name(name, &member)) {
        return;
    }
    nested_lifecycle_event_with_meta_t carrier;
    memset(&carrier, 0, sizeof carrier);
    carrier.event = member;
    nested_lifecycle_raise_external(&sm, &carrier);
    nested_lifecycle_step(&sm);
}

static bool machine_holds(const char *const *ids, size_t count) {
    size_t active = 0;
    for (int s = 0; s < (int)NESTED_LIFECYCLE_STATE_COUNT; ++s) {
        if (nested_lifecycle_in_state(&sm, (nested_lifecycle_state_t)s)) {
            active++;
        }
    }
    if (active != count) {
        return false;
    }
    for (size_t k = 0; k < count; ++k) {
        nested_lifecycle_state_t state = (nested_lifecycle_state_t)0;
        if (!nested_lifecycle_state_from_name(ids[k], &state) || !nested_lifecycle_in_state(&sm, state)) {
            return false;
        }
    }
    return true;
}

static const char *machine_terminal(void) {
    nested_lifecycle_state_t state = (nested_lifecycle_state_t)0;
    return nested_lifecycle_terminal_state(&sm, &state) ? nested_lifecycle_state_name(state) : "";
}

static uint32_t machine_discarded(void) {
    return nested_lifecycle_discarded_external_events(&sm);
}

static const char *function_state_name(int state) {
    return nested_lifecycle_transition_function_state_name((nested_lifecycle_transition_function_state_t)state);
}

static const char *function_event_name(int event) {
    return nested_lifecycle_transition_function_event_name((nested_lifecycle_transition_function_event_t)event);
}

static bool function_transition(int state, int event, int *next) {
    nested_lifecycle_transition_function_state_t to = (nested_lifecycle_transition_function_state_t)state;
    if (!nested_lifecycle_transition_function_transition((nested_lifecycle_transition_function_state_t)state,
                                                         (nested_lifecycle_transition_function_event_t)event, &to)) {
        return false;
    }
    *next = (int)to;
    return true;
}

static bool function_is_final(int state) {
    return nested_lifecycle_transition_function_is_final((nested_lifecycle_transition_function_state_t)state);
}

static size_t function_configuration(int state, const char *const **ids) {
    const nested_lifecycle_transition_function_configuration_t configuration =
        nested_lifecycle_transition_function_configuration((nested_lifecycle_transition_function_state_t)state);
    *ids = configuration.ids;
    return configuration.count;
}

int main(void) {
    const tf_equivalence_function_t function = {
        .state_count = (int)NESTED_LIFECYCLE_TRANSITION_FUNCTION_NUM_STATES,
        .event_count = (int)NESTED_LIFECYCLE_TRANSITION_FUNCTION_NUM_EVENTS,
        .initial = (int)NESTED_LIFECYCLE_TRANSITION_FUNCTION_INITIAL,
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
