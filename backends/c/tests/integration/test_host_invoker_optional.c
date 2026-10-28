// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.12 and docs/adr/0055, step 10 — a host-run `<invoke>`
// whose `sce:result` record holds values that may be absent. C11 compile+run
// gate; the Rust, Go, Kotlin, Python and C++ twins are
// `a_host_result_may_hold_a_value_that_is_absent.rs`, `host_invoker_optional_test.go`,
// `AHostResultMayHoldAValueThatIsAbsentTest`,
// `test_host_result_may_hold_a_value_that_is_absent.py` and
// `HostResultMayHoldAValueThatIsAbsentAotTest.cpp`.
//
// The machine is built with NO script engine: its variables are fields, and the
// link is the proof (this executable links without `sce_c_scripting` and `lua54`).
//
// The host answers through the generated typed interface, so the record it hands
// in holds each field that may be absent as the struct `sce/forge/optional.h`
// declares for its element; the machine's completion writes the record as JSON, an
// absent value as `null` with its key written all the same, and the machine reads
// each field where a test proves it present. What these hold apart is the three
// states a field has — absent, present and zero or empty, and present — which a
// sentinel could not: `""` is a label, `false` is a flag that was answered, and `0`
// is a day.
//
// Fixture: sce-build/tests/fixtures/host_processor/statechart_host_invoker_optional.scxml
// (the same document the other channels drive), generated WITH
// `--host-invoker x-sce-host` by `backends/c/tests/CMakeLists.txt`.

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "statechart_host_invoker_optional_sm.h"

// The type the fixture was compiled for. `backends/c/tests/CMakeLists.txt`
// passes this same string to `--host-invoker`.
#define DECLARED_TYPE "x-sce-host"

// A host implementing the generated invoker table; its work outlives the call, so
// each case completes the start it was handed.
typedef struct {
    int starts;
    uint64_t token;
} lookup_host_t;

static bool lookup_start(void *user_data, uint64_t token, statechart_host_invoker_optional_lookup_result_t *result) {
    (void)result;
    lookup_host_t *host = (lookup_host_t *)user_data;
    host->starts++;
    host->token = token;
    return false;
}

static void lookup_cancel(void *user_data, uint64_t token) {
    (void)user_data;
    (void)token;
}

static bool raise_event(statechart_host_invoker_optional_t *sm, const char *name) {
    statechart_host_invoker_optional_event_t event;
    if (!statechart_host_invoker_optional_resolve_event_by_name(name, &event)) {
        return false;
    }
    statechart_host_invoker_optional_event_with_meta_t meta;
    memset(&meta, 0, sizeof(meta));
    meta.event = event;
    statechart_host_invoker_optional_raise_external(sm, &meta);
    statechart_host_invoker_optional_step(sm);
    return true;
}

// A machine with the host's typed side registered, driven into `asking`: the
// lookup has started, and the host holds its token.
static bool boot_asking(statechart_host_invoker_optional_t *sm,
                        statechart_host_invoker_optional_x_sce_host_invoker_t *table, lookup_host_t *host) {
    sce_host_invoker_registry_t wiring;
    memset(host, 0, sizeof(*host));
    memset(table, 0, sizeof(*table));
    table->user_data = host;
    table->start_lookup = lookup_start;
    table->cancel_lookup = lookup_cancel;
    memset(&wiring, 0, sizeof(wiring));
    (void)statechart_host_invoker_optional_register_x_sce_host_invoker(&wiring, table);
    statechart_host_invoker_optional_init_with_host_invokers(sm, &wiring);
    if (!raise_event(sm, "ask")) {
        return false;
    }
    if (host->starts != 1) {
        (void)fprintf(stderr, "FAIL: the lookup was started %d times, want 1\n", host->starts);
        return false;
    }
    return true;
}

// What the machine is expected to have published after a completion.
typedef struct {
    unsigned answered;
    unsigned rows_seen;
    bool at_present;
    int64_t at_value;
    bool label_present;
    const char *label_value;
    bool hit_known;
    bool hit_true;
} answer_t;

static int expect_answer(const char *what, const statechart_host_invoker_optional_t *sm, const answer_t *want) {
    int bad = 0;
    const unsigned answered = (unsigned)statechart_host_invoker_optional_get_answered(sm);
    const unsigned rows_seen = (unsigned)statechart_host_invoker_optional_get_rows_seen(sm);
    const bool at_present = statechart_host_invoker_optional_get_at_present(sm);
    const int64_t at_value = statechart_host_invoker_optional_get_at_value(sm);
    const bool label_present = statechart_host_invoker_optional_get_label_present(sm);
    const char *label_value = statechart_host_invoker_optional_get_label_value(sm);
    const bool hit_known = statechart_host_invoker_optional_get_hit_known(sm);
    const bool hit_true = statechart_host_invoker_optional_get_hit_true(sm);
    if (answered != want->answered) {
        (void)fprintf(stderr, "FAIL [%s]: answered is %u, want %u\n", what, answered, want->answered);
        bad = 1;
    }
    if (rows_seen != want->rows_seen) {
        (void)fprintf(stderr, "FAIL [%s]: rows_seen is %u, want %u\n", what, rows_seen, want->rows_seen);
        bad = 1;
    }
    if (at_present != want->at_present) {
        (void)fprintf(stderr, "FAIL [%s]: at_present is %d, want %d\n", what, (int)at_present, (int)want->at_present);
        bad = 1;
    }
    if (at_value != want->at_value) {
        (void)fprintf(stderr, "FAIL [%s]: at_value is %lld, want %lld\n", what, (long long)at_value,
                      (long long)want->at_value);
        bad = 1;
    }
    if (label_present != want->label_present) {
        (void)fprintf(stderr, "FAIL [%s]: label_present is %d, want %d\n", what, (int)label_present,
                      (int)want->label_present);
        bad = 1;
    }
    if (label_value == NULL || strcmp(label_value, want->label_value) != 0) {
        (void)fprintf(stderr, "FAIL [%s]: label_value is `%s`, want `%s`\n", what,
                      label_value != NULL ? label_value : "(null)", want->label_value);
        bad = 1;
    }
    if (hit_known != want->hit_known) {
        (void)fprintf(stderr, "FAIL [%s]: hit_known is %d, want %d\n", what, (int)hit_known, (int)want->hit_known);
        bad = 1;
    }
    if (hit_true != want->hit_true) {
        (void)fprintf(stderr, "FAIL [%s]: hit_true is %d, want %d\n", what, (int)hit_true, (int)want->hit_true);
        bad = 1;
    }
    return bad;
}

// Ask, complete the lookup with `result` through the typed interface, settle the
// machine, and compare what it published with `want`.
static int complete_and_expect(const char *what, const statechart_host_invoker_optional_lookup_result_t *result,
                               const answer_t *want) {
    static statechart_host_invoker_optional_t sm;
    statechart_host_invoker_optional_x_sce_host_invoker_t table;
    lookup_host_t host;
    if (!boot_asking(&sm, &table, &host)) {
        statechart_host_invoker_optional_destroy(&sm);
        return 1;
    }
    int bad = 0;
    if (!statechart_host_invoker_optional_complete_lookup(&sm, host.token, result)) {
        (void)fprintf(stderr, "FAIL [%s]: a running invocation's typed completion was refused\n", what);
        bad = 1;
    }
    statechart_host_invoker_optional_step(&sm);
    bad |= expect_answer(what, &sm, want);
    statechart_host_invoker_optional_destroy(&sm);
    return bad;
}

// Every field that may be absent is: the machine reads three absent values and the
// count every answer carries.
static int an_absent_field_is_read_as_absent(void) {
    statechart_host_invoker_optional_lookup_result_t result;
    memset(&result, 0, sizeof(result));
    result.rows = 3;
    result.at = sce_optional_int64_none();
    result.label = sce_optional_string_16_none();
    result.hit = sce_optional_bool_none();
    const answer_t want = {1u, 3u, false, 0, false, "", false, false};
    return complete_and_expect("all absent", &result, &want);
}

// Every field is present: the machine reads each value.
static int a_present_field_is_read_as_its_value(void) {
    statechart_host_invoker_optional_lookup_result_t result;
    memset(&result, 0, sizeof(result));
    result.rows = 4;
    result.at = sce_optional_int64_some(41);
    result.label = sce_optional_string_16_some(sce_static_string_16_of("ab"));
    result.hit = sce_optional_bool_some(true);
    const answer_t want = {1u, 4u, true, 41, true, "ab", true, true};
    return complete_and_expect("all present", &result, &want);
}

// A zero, an empty text and a false flag are values, not absences: each is present,
// and is read as itself.
static int a_zero_an_empty_text_and_a_false_flag_are_values_and_not_absences(void) {
    statechart_host_invoker_optional_lookup_result_t result;
    memset(&result, 0, sizeof(result));
    result.rows = 1;
    result.at = sce_optional_int64_some(0);
    result.label = sce_optional_string_16_some(sce_static_string_16_of(""));
    result.hit = sce_optional_bool_some(false);
    const answer_t want = {1u, 1u, true, 0, true, "", true, false};
    return complete_and_expect("zero, empty, false", &result, &want);
}

// Each field is independent: a day with no label and a flag.
static int each_field_is_absent_or_present_on_its_own(void) {
    statechart_host_invoker_optional_lookup_result_t result;
    memset(&result, 0, sizeof(result));
    result.rows = 2;
    result.at = sce_optional_int64_some(7);
    result.label = sce_optional_string_16_none();
    result.hit = sce_optional_bool_some(true);
    const answer_t want = {1u, 2u, true, 7, false, "", true, true};
    return complete_and_expect("mixed", &result, &want);
}

// A key left out is not an absent value: every field of the first revision of a
// schema is carried by every delivery, and `null` is the one spelling of absence, so
// a completion that lacks `at` is refused as any typed payload the data does not fit
// is, and the machine reads no answer.
static int a_completion_that_leaves_a_key_out_is_refused(void) {
    static statechart_host_invoker_optional_t sm;
    statechart_host_invoker_optional_x_sce_host_invoker_t table;
    lookup_host_t host;
    if (!boot_asking(&sm, &table, &host)) {
        statechart_host_invoker_optional_destroy(&sm);
        return 1;
    }
    int bad = 0;
    if (!statechart_host_invoker_optional_complete_host_invoke(&sm, DECLARED_TYPE, "lookup", host.token,
                                                               "{\"rows\":1,\"label\":null,\"hit\":null}")) {
        (void)fprintf(stderr, "FAIL: a running invocation's completion was refused\n");
        bad = 1;
    }
    statechart_host_invoker_optional_step(&sm);
    if (statechart_host_invoker_optional_get_answered(&sm) != 0u) {
        (void)fprintf(stderr, "FAIL: a record without `at` was read as an answer\n");
        bad = 1;
    }
    statechart_host_invoker_optional_destroy(&sm);
    return bad;
}

int main(void) {
    int bad = 0;
    bad |= an_absent_field_is_read_as_absent();
    bad |= a_present_field_is_read_as_its_value();
    bad |= a_zero_an_empty_text_and_a_false_flag_are_values_and_not_absences();
    bad |= each_field_is_absent_or_present_on_its_own();
    bad |= a_completion_that_leaves_a_key_out_is_refused();
    if (bad != 0) {
        return 1;
    }
    (void)printf("host invoker optional: ok\n");
    return 0;
}
