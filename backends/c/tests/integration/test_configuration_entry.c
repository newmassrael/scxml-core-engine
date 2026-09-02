// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.11 — what `_init_at` accepts, and what it refuses, on the C11
// engine.
//
// The door exists so a host can bring a machine back where it was, in a new
// process, without replaying the entry actions the earlier run already ran.
// Refusals are the part that has to be enumerated rather than sampled: coming
// up "near" the requested configuration is the one outcome it must never
// produce, because nothing afterwards can detect it — `_in_state` answers,
// `_active_states` answers, and the set behind those answers is one the
// document never describes. A gate holding only the accepting case would pass
// on an engine that accepted everything.
//
// The C11 sibling of `backends/rust/runtime/tests/configuration_entry.rs`,
// `tests/integration/ConfigurationEntryAotTest.cpp`,
// `backends/go/tests/configuration_entry/`,
// `backends/python/tests/configuration_entry/` and
// `com.sce.integration.ConfigurationEntryTest`, asking the same questions of
// the same rules, so a set one engine accepts is one the others accept.
//
// Two machines, because the two halves of the door are different code paths:
//
//   - `parallel_regions_take_own_transitions` has `<parallel>` regions and a
//     datamodel the door has to declare;
//   - `statechart_native_action` has neither, and its every effect is a
//     `<sce:action>` — which is where this backend's own extra refusal lives:
//     a host that does not perform every act the document declares. The five
//     other backends get that from their type systems, so C is the one channel
//     where it is a value rather than a compile error.
//
// This file is deliberately not a fixture stem's driver: it drives documents
// that already exist in the tree rather than adding a document of its own,
// because the claim is about a runtime door and not about a topology.

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "parallel_regions_take_own_transitions_sm.h"
#include "statechart_native_action_sm.h"

#define P_(suffix) PARALLEL_REGIONS_TAKE_OWN_TRANSITIONS_##suffix
#define N_(suffix) STATECHART_NATIVE_ACTION_##suffix

typedef parallel_regions_take_own_transitions_state_t p_state_t;
typedef parallel_regions_take_own_transitions_configuration_rejection_t p_verdict_t;

static int failures = 0;

static void expect_verdict(const char *what, p_verdict_t got, p_verdict_t want) {
    if (got == want) {
        return;
    }
    fprintf(stderr, "FAIL: %s\n  expected: %s\n  answered: %s\n", what,
            parallel_regions_take_own_transitions_configuration_rejection_text(want),
            parallel_regions_take_own_transitions_configuration_rejection_text(got));
    ++failures;
}

static void expect_true(const char *what, bool condition) {
    if (condition) {
        return;
    }
    fprintf(stderr, "FAIL: %s\n", what);
    ++failures;
}

/* A mid-run configuration of the parallel document: both regions live, the
   deeper one in `working` and the shallower in `within`. Written out rather
   than taken from a live run because every refusal below is a MUTATION of it —
   one change each, so a refusal names one rule. */
static const p_state_t AT_WORK[] = {
    P_(STATE_RUN), P_(STATE_DRIVE), P_(STATE_RUNNING), P_(STATE_WORKING), P_(STATE_BUDGET), P_(STATE_WITHIN),
};
#define AT_WORK_COUNT (sizeof(AT_WORK) / sizeof(AT_WORK[0]))

static uint32_t bitmap_of(const p_state_t *states, size_t count) {
    uint32_t bits = 0u;
    for (size_t i = 0u; i < count; ++i) {
        bits |= (uint32_t)1u << (uint32_t)states[i];
    }
    return bits;
}

/* The set written above is a configuration of the document, so it is accepted
   and the machine comes up holding exactly it. This is the baseline every
   refusal below is one mutation away from — without it, a validator that
   refused everything would pass every other case in this file. */
static void a_parallel_configuration_is_accepted(void) {
    parallel_regions_take_own_transitions_t sm;
    expect_verdict("a configuration of the document was refused",
                   parallel_regions_take_own_transitions_init_at(&sm, AT_WORK, AT_WORK_COUNT, NULL), P_(CONFIG_NONE));

    expect_true("the machine came up holding a different set from the one it was handed",
                parallel_regions_take_own_transitions_active_states(&sm) == bitmap_of(AT_WORK, AT_WORK_COUNT));
    expect_true("the resumed machine is not in the leaf it was handed",
                parallel_regions_take_own_transitions_in_state(&sm, P_(STATE_WORKING)));
    expect_true("the resumed machine holds `judging`, which it was not handed",
                !parallel_regions_take_own_transitions_in_state(&sm, P_(STATE_JUDGING)));
    expect_true("a resumed machine reports itself already finished",
                !parallel_regions_take_own_transitions_is_in_final_state(&sm));

    parallel_regions_take_own_transitions_destroy(&sm);
}

/* A resume is not a replay. `_init` walks the document from its initial
   configuration; `_init_at` is handed one, and the two must not agree — if
   they did, this door would be doing nothing and every claim above would hold
   on an engine that ignored its argument. */
static void a_resume_is_not_the_initial_configuration(void) {
    static const p_state_t elsewhere[] = {
        P_(STATE_RUN), P_(STATE_DRIVE), P_(STATE_RUNNING), P_(STATE_JUDGING), P_(STATE_BUDGET), P_(STATE_WITHIN),
    };

    parallel_regions_take_own_transitions_t fresh;
    parallel_regions_take_own_transitions_init(&fresh);
    const uint32_t initial = parallel_regions_take_own_transitions_active_states(&fresh);
    parallel_regions_take_own_transitions_destroy(&fresh);

    parallel_regions_take_own_transitions_t resumed;
    expect_verdict("a mid-run configuration was refused",
                   parallel_regions_take_own_transitions_init_at(&resumed, elsewhere,
                                                                 sizeof(elsewhere) / sizeof(elsewhere[0]), NULL),
                   P_(CONFIG_NONE));
    expect_true("the resumed machine came up in the document's INITIAL configuration, "
                "so the set it was handed changed nothing",
                parallel_regions_take_own_transitions_active_states(&resumed) != initial);
    expect_true("the resumed machine is not in `judging`, which the journal recorded",
                parallel_regions_take_own_transitions_in_state(&resumed, P_(STATE_JUDGING)));
    parallel_regions_take_own_transitions_destroy(&resumed);
}

static void an_empty_configuration_is_refused(void) {
    parallel_regions_take_own_transitions_t sm;
    expect_verdict("a machine is never in nothing",
                   parallel_regions_take_own_transitions_init_at(&sm, AT_WORK, 0u, NULL), P_(CONFIG_EMPTY));
    expect_verdict("a NULL set is the same nothing", parallel_regions_take_own_transitions_init_at(&sm, NULL, 3u, NULL),
                   P_(CONFIG_EMPTY));
}

/* W3C SCXML 3.11: a compound state holds exactly one active child. `working`
   and `judging` are both children of `running`, and a run stands in one. */
static void two_siblings_of_one_region_are_refused(void) {
    p_state_t configuration[AT_WORK_COUNT + 1u];
    memcpy(configuration, AT_WORK, sizeof(AT_WORK));
    configuration[AT_WORK_COUNT] = P_(STATE_JUDGING);

    parallel_regions_take_own_transitions_t sm;
    expect_verdict("`running` was given two active children, which is a configuration the "
                   "document has no reading for",
                   parallel_regions_take_own_transitions_init_at(&sm, configuration, AT_WORK_COUNT + 1u, NULL),
                   P_(CONFIG_COMPOUND_CHILD_COUNT));
}

/* W3C SCXML 3.11: a `<parallel>` holds EVERY region. Dropping one is the shape
   a host produces when it journals only the region it cares about. */
static void a_parallel_with_a_region_missing_is_refused(void) {
    static const p_state_t configuration[] = {
        P_(STATE_RUN),
        P_(STATE_DRIVE),
        P_(STATE_RUNNING),
        P_(STATE_WORKING),
    };

    parallel_regions_take_own_transitions_t sm;
    expect_verdict("`budget` is a region of `run` and a run is always in both at once",
                   parallel_regions_take_own_transitions_init_at(
                       &sm, configuration, sizeof(configuration) / sizeof(configuration[0]), NULL),
                   P_(CONFIG_PARALLEL_REGION_MISSING));
}

/* The set has to be ancestor-closed: a state is active only if its parent is. */
static void a_configuration_that_skips_an_ancestor_is_refused(void) {
    static const p_state_t configuration[] = {
        P_(STATE_RUN), P_(STATE_DRIVE), P_(STATE_WORKING), P_(STATE_BUDGET), P_(STATE_WITHIN),
    };

    parallel_regions_take_own_transitions_t sm;
    expect_verdict("`working` is a child of `running`, which the set does not hold",
                   parallel_regions_take_own_transitions_init_at(
                       &sm, configuration, sizeof(configuration) / sizeof(configuration[0]), NULL),
                   P_(CONFIG_ANCESTOR_MISSING));
}

/* Checked before the arity counts, because a duplicate would otherwise read as
   a second child and the refusal would name the wrong rule. */
static void a_repeated_state_is_refused(void) {
    p_state_t configuration[AT_WORK_COUNT + 1u];
    memcpy(configuration, AT_WORK, sizeof(AT_WORK));
    configuration[AT_WORK_COUNT] = P_(STATE_WORKING);

    parallel_regions_take_own_transitions_t sm;
    expect_verdict("a state named twice",
                   parallel_regions_take_own_transitions_init_at(&sm, configuration, AT_WORK_COUNT + 1u, NULL),
                   P_(CONFIG_DUPLICATE));
}

/* W3C SCXML 3.11: a configuration closes on exactly one root. `settled` is a
   top-level `<final>`, so a set holding both it and `run` describes two
   machines. */
static void two_roots_are_refused(void) {
    p_state_t configuration[AT_WORK_COUNT + 1u];
    memcpy(configuration, AT_WORK, sizeof(AT_WORK));
    configuration[AT_WORK_COUNT] = P_(STATE_SETTLED);

    parallel_regions_take_own_transitions_t sm;
    expect_verdict("two disjoint trees",
                   parallel_regions_take_own_transitions_init_at(&sm, configuration, AT_WORK_COUNT + 1u, NULL),
                   P_(CONFIG_ROOT_COUNT));
}

/* The refusal only this backend needs a name for. Its siblings' configurations
   hold a generated enum or a sealed object, so a state the document does not
   have cannot be constructed; C can be handed any integer, and walking a
   bitmap it does not fit is undefined behaviour rather than a refusal. */
static void a_state_this_document_does_not_have_is_refused(void) {
    const p_state_t configuration[] = {
        P_(STATE_RUN),
        (p_state_t)P_(STATE_COUNT),
    };

    parallel_regions_take_own_transitions_t sm;
    expect_verdict("a value outside the document's state enumeration",
                   parallel_regions_take_own_transitions_init_at(&sm, configuration, 2u, NULL),
                   P_(CONFIG_UNKNOWN_STATE));
}

/* The claim that makes every refusal above safe to act on: validation runs
   BEFORE any mutation. On this backend "any mutation" starts with the memset,
   so a refused call must leave the caller's storage untouched — and a host that
   reads a rejection still holds whatever it had there. */
static void a_refused_entry_leaves_the_callers_storage_untouched(void) {
    parallel_regions_take_own_transitions_t sm;
    memset(&sm, 0xA5, sizeof(sm));

    unsigned char before[sizeof(sm)];
    memcpy(before, &sm, sizeof(sm));

    expect_verdict("the empty set", parallel_regions_take_own_transitions_init_at(&sm, AT_WORK, 0u, NULL),
                   P_(CONFIG_EMPTY));

    expect_true("a refused entry wrote into the caller's storage", memcmp(before, &sm, sizeof(sm)) == 0);
}

/* W3C SCXML 3.3: every state this document declares reads back from its own
   name.

   A host can only record a configuration as TEXT — the enum is a build artefact
   of one binary, and the process that resumes is a different one. This walks
   `_STATE_COUNT` rather than a list spelled here, so a document that grows a
   state grows this check with it. */
static void every_state_reads_back_from_its_own_name(void) {
    expect_true("the document declares fewer states than the walk claims to cover", (unsigned)P_(STATE_COUNT) >= 8u);

    for (uint32_t i = 0u; i < (uint32_t)P_(STATE_COUNT); ++i) {
        const p_state_t state = (p_state_t)i;
        const char *name = parallel_regions_take_own_transitions_state_name(state);
        expect_true("a state of this document publishes no name", name[0] != '\0');

        p_state_t back;
        expect_true("a name this machine publishes could not be read back",
                    parallel_regions_take_own_transitions_state_from_name(name, &back));
        expect_true("a name read back as a different state", back == state);
    }

    p_state_t sink = P_(STATE_RUN);
    expect_true(
        "a name the document does not carry was answered with a state rather than "
        "refused; a name guessed at is how a restore reaches a configuration nobody "
        "recorded",
        !parallel_regions_take_own_transitions_state_from_name("a-state-this-document-does-not-declare", &sink));
    expect_true("a refused lookup wrote through its out-parameter", sink == P_(STATE_RUN));
    expect_true("a NULL name was not refused", !parallel_regions_take_own_transitions_state_from_name(NULL, &sink));
    expect_true("a value outside the enumeration was given a name",
                parallel_regions_take_own_transitions_state_name((p_state_t)P_(STATE_COUNT))[0] == '\0');
}

/* A configuration that crossed a process: journalled as names, read back
   through the generated reverse table, and handed to the door. This is the
   whole point of the pair — the two halves in one call chain rather than each
   proved alone. */
static void a_configuration_journalled_as_names_is_accepted_back(void) {
    parallel_regions_take_own_transitions_t writer;
    parallel_regions_take_own_transitions_init(&writer);
    parallel_regions_take_own_transitions_run(&writer);
    parallel_regions_take_own_transitions_event_with_meta_t e = {0};
    e.event = P_(EVENT_E);
    parallel_regions_take_own_transitions_raise_external(&writer, &e);
    parallel_regions_take_own_transitions_run(&writer);

    char journal[P_(STATE_COUNT)][64];
    size_t journalled = 0u;
    for (uint32_t i = 0u; i < (uint32_t)P_(STATE_COUNT); ++i) {
        if (!parallel_regions_take_own_transitions_in_state(&writer, (p_state_t)i)) {
            continue;
        }
        const char *name = parallel_regions_take_own_transitions_state_name((p_state_t)i);
        snprintf(journal[journalled], sizeof(journal[journalled]), "%s", name);
        ++journalled;
    }
    parallel_regions_take_own_transitions_destroy(&writer);

    expect_true("a run of this document journalled nothing", journalled > 0u);

    p_state_t configuration[P_(STATE_COUNT)];
    size_t restored = 0u;
    for (size_t i = 0u; i < journalled; ++i) {
        p_state_t state;
        expect_true("a name a run journalled could not be read back",
                    parallel_regions_take_own_transitions_state_from_name(journal[i], &state));
        configuration[restored++] = state;
    }

    parallel_regions_take_own_transitions_t reader;
    expect_verdict("a configuration a run actually reached was refused on the way back",
                   parallel_regions_take_own_transitions_init_at(&reader, configuration, restored, NULL),
                   P_(CONFIG_NONE));
    expect_true("the resumed configuration is not the journalled one",
                parallel_regions_take_own_transitions_active_states(&reader) == bitmap_of(configuration, restored));
    parallel_regions_take_own_transitions_destroy(&reader);
}

/* The host that performs every act the linear document declares. Silent, except
   for the two counters — which are what says no entry or exit content ran
   during a resume. */
static int idle_entries = 0;
static int assembling_exits = 0;

static void host_append_fragment_payload(void *user_data, const uint8_t *payload, size_t payload_len, uint32_t offset) {
    (void)user_data;
    (void)payload;
    (void)payload_len;
    (void)offset;
}

static void host_reset_slot(void *user_data) {
    (void)user_data;
}

static void host_on_idle_entry(void *user_data) {
    (void)user_data;
    ++idle_entries;
}

static void host_on_assembling_exit(void *user_data) {
    (void)user_data;
    ++assembling_exits;
}

/* A machine with no `<parallel>` and no script engine: one leaf, no ancestors,
   and a datamodel declaration with nothing to declare. The same door has to
   close there too — and this is where the act table's own refusal lives. */
static void a_linear_configuration_round_trips(void) {
    static const statechart_native_action_state_t assembling[] = {
        N_(STATE_ASSEMBLING),
    };

    statechart_native_action_actions_t host;
    memset(&host, 0, sizeof(host));

    /* W3C SCXML G.7: a host that does not perform every act the document
       declares is refused — and refused AFTER the set has been read, because
       an empty set is not a configuration whoever is asking. */
    statechart_native_action_t sm;
    if (statechart_native_action_init_at(&sm, assembling, 1u, &host) != N_(CONFIG_HOST_ACTIONS_INCOMPLETE)) {
        fprintf(stderr, "FAIL: a host with an unfilled act table was accepted (W3C SCXML G.7)\n");
        ++failures;
    }

    host.append_fragment_payload = host_append_fragment_payload;
    host.reset_slot = host_reset_slot;
    host.on_idle_entry = host_on_idle_entry;
    host.on_assembling_exit = host_on_assembling_exit;

    idle_entries = 0;
    assembling_exits = 0;

    if (statechart_native_action_init_at(&sm, assembling, 1u, &host) != N_(CONFIG_NONE)) {
        fprintf(stderr, "FAIL: a single-state configuration was refused\n");
        ++failures;
    }
    if (!statechart_native_action_in_state(&sm, N_(STATE_ASSEMBLING))) {
        fprintf(stderr, "FAIL: the resumed machine is not in the state it was handed\n");
        ++failures;
    }
    if (statechart_native_action_in_state(&sm, N_(STATE_IDLE))) {
        fprintf(stderr, "FAIL: the resumed machine also holds `idle`, which it was not handed — "
                        "and `idle` is where `_init` would have put it\n");
        ++failures;
    }
    if (idle_entries != 0 || assembling_exits != 0) {
        fprintf(stderr, "FAIL: entry/exit content ran during a resume: %d entries, %d exits\n", idle_entries,
                assembling_exits);
        ++failures;
    }
    statechart_native_action_destroy(&sm);
}

int main(void) {
    a_parallel_configuration_is_accepted();
    a_resume_is_not_the_initial_configuration();
    an_empty_configuration_is_refused();
    two_siblings_of_one_region_are_refused();
    a_parallel_with_a_region_missing_is_refused();
    a_configuration_that_skips_an_ancestor_is_refused();
    a_repeated_state_is_refused();
    two_roots_are_refused();
    a_state_this_document_does_not_have_is_refused();
    a_refused_entry_leaves_the_callers_storage_untouched();
    every_state_reads_back_from_its_own_name();
    a_configuration_journalled_as_names_is_accepted_back();
    a_linear_configuration_round_trips();

    if (failures != 0) {
        fprintf(stderr, "FAIL: %d configuration-entry claim(s) did not hold\n", failures);
        return 1;
    }
    printf("PASS: configuration entry (W3C SCXML 3.11)\n");
    return 0;
}
