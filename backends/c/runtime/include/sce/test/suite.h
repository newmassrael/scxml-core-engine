/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/**
 * @file suite.h
 * @brief A test suite of the test profile as its runner reads it
 *        (docs/SCE_TEST_PROFILE.md §4.11, §11.7, §11.8, §12.2), and the runner
 *
 * The test cases in run order, each with its guard and the selection that runs
 * it in a world of the suite's features, and the suite's module parameters; and
 * the runner, which runs the cases a world selects one at a time, in that
 * order. The C11 port of `sce/include/test/Suite.h` and
 * `sce_rust_runtime::test::run_suite`.
 *
 * The tables are generated from the `test-case` and `test-suite` documents (a
 * case's configuration from its `test-configuration`), as constant storage.
 * They are data: what a case does is its components' machines, which the run
 * drives (sce/test/test_case_run.h). The runner is a library function, with no
 * `main`: a generated or hand-written test calls it with the system adapters
 * that play the SUT (`sce_test_run_suite`), or with the recorded input trace of
 * each test case (`sce_test_run_suite_with_traces`), which the program of a
 * generated suite does (sce/test/runner.h).
 *
 * C's own, against its siblings: the runner returns no list, since a list of
 * results would need storage sized by the suite. It runs each case in one run
 * the caller places, reused case after case, and hands each result to the
 * caller's function as it ends, valid for that call. Header-only and
 * allocation-free.
 */

#ifndef SCE_TEST_SUITE_H
#define SCE_TEST_SUITE_H

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>

#include "sce/saved_json.h"
#include "sce/test/input_trace.h"
#include "sce/test/module_parameters.h"
#include "sce/test/test_case_run.h"

#ifdef __cplusplus
extern "C" {
#endif

/**
 * The features a world of a suite's feature model enables (ADR 0057 §2): a
 * configuration of the model, closed under its `requires` edges. The names are
 * lent.
 */
typedef struct sce_test_features_s {
    const char *const *names;
    size_t count;
} sce_test_features_t;

/** Whether `world` enables `feature`: the one test a selection is made of. */
static inline bool sce_test_enabled(const sce_test_features_t *world, const char *feature) {
    for (size_t i = 0u; i < world->count; ++i) {
        if (strcmp(world->names[i], feature) == 0) {
            return true;
        }
    }
    return false;
}

/**
 * A `test-case` (§11.7): its name and what it runs in.
 *
 * The configuration and the machines of its components are its
 * configuration's; the module parameters are the ids every component of the
 * configuration reads, which the suite must give.
 */
typedef struct sce_test_case_s {
    /** The test case's name in the suite and in the test log. */
    const char *name;
    const sce_test_configuration_t *configuration;
    sce_test_machine_factory_t machines;
    const char *const *module_parameters;
    size_t module_parameter_count;
    /** The reader of its configuration's inputs, which a trace replayed for it
        is read with (`sce_test_run_suite_with_traces`): which records each
        system port takes, and each record's value from its canonical form. NULL
        for a configuration with no system port, whose every input is refused. */
    sce_test_input_reader_fn inputs;
} sce_test_case_t;

/** A selection (§11.8): whether a world of the suite's features runs a test
 *  case. A condition of ADR 0057's grammar, lowered to the features it names. */
typedef bool (*sce_test_selection_fn)(const sce_test_features_t *world);

/** One `<sce:execute>` (§11.8, P11): a test case, its guard, and its selection. */
typedef struct sce_test_execute_s {
    sce_test_case_t test_case;
    /** Whether the test case has a guard, its time limit, and the guard. */
    bool guarded;
    sce_test_duration_t guard;
    /** The selection, or NULL for a test case every world runs. */
    sce_test_selection_fn selection;
} sce_test_execute_t;

/** A `test-suite` (§11.8): its test cases in run order and its module
 *  parameters at their defaults. */
typedef struct sce_test_suite_s {
    const char *name;
    sce_test_module_parameters_t module_parameters;
    const sce_test_execute_t *executes;
    size_t execute_count;
} sce_test_suite_t;

/**
 * Builds the system adapter a test case runs against, given the test case's
 * name: a fresh adapter for each case, so that no case sees another's trace, or
 * NULL for none. The runner gives the adapter back through its `release` once
 * the case has run. `user_data` is handed back unchanged.
 */
typedef struct sce_test_adapter_factory_s {
    sce_test_system_adapter_t *(*make)(void *user_data, const char *test_case);
    void *user_data;
} sce_test_adapter_factory_t;

/** What the runner reports of one test case it ran: its name and its result,
 *  valid for the call, in which its test log is written
 *  (`sce_test_case_result_canonical_log`, given the suite's `name`). */
typedef void (*sce_test_case_result_fn)(void *user_data, const char *test_case, const sce_test_case_result_t *result);

/** An override of a module parameter the suite does not declare: the caller's
 *  mistake, not a verdict of any test case, so no case is run. */
typedef struct sce_test_override_refusal_s {
    char reason[SCE_TEST_MAX_LOG_DETAIL];
} sce_test_override_refusal_t;

/* P13: the test case cannot start, so it ends at once with `error`, the reason
   its log's one line. No component was created, so its test log holds its start
   and its end alone. */
static inline const sce_test_case_result_t *sce_test_suite_refused_(sce_test_case_run_t *run, const char *format, ...) {
    va_list args;
    memset(&run->result, 0, sizeof(run->result));
    run->now = 0;
    run->result.verdict = SCE_TEST_VERDICT_ERROR;
    va_start(args, format);
    sce_test_run_vrecord_(run, SCE_TEST_MTC_NAME, SCE_TEST_LOG_ERROR, format, args);
    va_end(args);
    sce_test_trace_started(&run->result.trace, run->now);
    sce_test_trace_ended(&run->result.trace, run->now, SCE_TEST_VERDICT_ERROR);
    return &run->result;
}

/**
 * The module parameters of a run of `*suite`: its defaults, with the values
 * `*overrides` gives (NULL for none) in their place, in `*parameters`. An
 * override of a module parameter the suite does not declare is the caller's
 * mistake, not a verdict of any test case: false, the reason in `*refusal` where
 * it is not NULL, which a runner reads before it runs any case.
 */
static inline bool sce_test_suite_parameters(const sce_test_suite_t *suite,
                                             const sce_test_module_parameters_t *overrides,
                                             sce_test_module_parameters_t *parameters,
                                             sce_test_override_refusal_t *refusal) {
    *parameters = suite->module_parameters;
    if (overrides == NULL) {
        return true;
    }
    for (size_t i = 0u; i < overrides->count; ++i) {
        const sce_test_module_parameter_t *given = &overrides->entries[i];
        if (!sce_test_module_parameters_has(parameters, given->id)) {
            if (refusal != NULL) {
                (void)snprintf(refusal->reason, sizeof(refusal->reason),
                               "overrides the module parameter '%s', which the suite '%s' does not declare", given->id,
                               suite->name);
            }
            return false;
        }
        (void)sce_test_module_parameters_set(parameters, given->id, given->value);
    }
    return true;
}

/**
 * The adapter a test case runs against, built by the caller of
 * `sce_test_run_selected_` given `user_data` and the test case; NULL, the reason
 * in `*why`, where it has none, which ends the test case with `error` (P13).
 */
typedef sce_test_system_adapter_t *(*sce_test_adapter_of_fn)(void *user_data, const sce_test_case_t *test_case,
                                                             sce_saved_error_t *why);

/* The runner of `sce_test_run_suite` and `sce_test_run_suite_with_traces`: the
   test cases `*world` selects, each against the adapter `adapter_of` builds for
   it, or ended with `error` for the reason it gives where it builds none. */
static inline bool sce_test_run_selected_(const sce_test_suite_t *suite, const sce_test_features_t *world,
                                          const sce_test_module_parameters_t *overrides,
                                          sce_test_adapter_of_fn adapter_of, void *adapter_user,
                                          sce_test_case_run_t *run, sce_test_case_result_fn report, void *user_data,
                                          sce_test_override_refusal_t *refusal) {
    sce_test_module_parameters_t parameters;
    if (!sce_test_suite_parameters(suite, overrides, &parameters, refusal)) {
        return false;
    }
    for (size_t e = 0u; e < suite->execute_count; ++e) {
        const sce_test_execute_t *execute = &suite->executes[e];
        if (execute->selection != NULL && !execute->selection(world)) {
            continue;
        }
        const sce_test_case_t *test_case = &execute->test_case;
        const char *missing = NULL;
        for (size_t p = 0u; p < test_case->module_parameter_count; ++p) {
            if (!sce_test_module_parameters_has(&parameters, test_case->module_parameters[p])) {
                missing = test_case->module_parameters[p];
                break;
            }
        }
        if (missing != NULL) {
            report(user_data, test_case->name,
                   sce_test_suite_refused_(
                       run, "the test case reads the module parameter '%s', which is given no value", missing));
            continue;
        }
        sce_saved_error_t why;
        why.message[0] = '\0';
        sce_test_system_adapter_t *adapter = adapter_of(adapter_user, test_case, &why);
        if (adapter == NULL) {
            report(user_data, test_case->name, sce_test_suite_refused_(run, "%s", why.message));
            continue;
        }
        sce_test_case_run_init(run, test_case->configuration, test_case->machines, adapter,
                               execute->guarded ? &execute->guard : NULL, &parameters);
        report(user_data, test_case->name, sce_test_case_run_run(run));
        if (adapter->vtable->release != NULL) {
            adapter->vtable->release(adapter);
        }
    }
    return true;
}

/* The adapter the factory `*user_data` builds for a test case, by its name. */
static inline sce_test_system_adapter_t *sce_test_adapter_of_factory_(void *user_data, const sce_test_case_t *test_case,
                                                                      sce_saved_error_t *why) {
    const sce_test_adapter_factory_t *adapters = (const sce_test_adapter_factory_t *)user_data;
    sce_test_system_adapter_t *adapter = adapters->make(adapters->user_data, test_case->name);
    if (adapter == NULL) {
        (void)sce_saved_refuse(why, "no system adapter is given for the test case");
    }
    return adapter;
}

/**
 * Runs the test cases of `*suite` that `*world` selects (P11).
 *
 * One at a time, on the calling thread, in the suite's order, each a fresh run
 * in `*run` from `NOW = 0` with fresh entities against the adapter `adapters`
 * builds for it, and each one's result handed to `report` with `user_data` as it
 * ends. Each component reads the suite's module parameters, with the values
 * `*overrides` gives (NULL for none) in place of their defaults.
 *
 * A test case whose components read a module parameter the run has no value
 * for ends with `error` (P13) before any component is created, and before its
 * adapter is built, as does one for which `adapters` builds no adapter. An
 * override of a module parameter the suite does not declare is refused before
 * any case runs: false, the reason in `*refusal` where it is not NULL.
 */
static inline bool sce_test_run_suite(const sce_test_suite_t *suite, const sce_test_features_t *world,
                                      sce_test_adapter_factory_t adapters,
                                      const sce_test_module_parameters_t *overrides, sce_test_case_run_t *run,
                                      sce_test_case_result_fn report, void *user_data,
                                      sce_test_override_refusal_t *refusal) {
    return sce_test_run_selected_(suite, world, overrides, sce_test_adapter_of_factory_, &adapters, run, report,
                                  user_data, refusal);
}

/**
 * Gives the text of the recorded input trace of a test case, given its name:
 * the `*len` bytes at `*text`, lent until the next call; false, naming why in
 * `*err`, where there is none or it is not read. `user_data` is handed back
 * unchanged.
 */
typedef struct sce_test_trace_source_s {
    bool (*read)(void *user_data, const char *test_case, const char **text, size_t *len, sce_saved_error_t *err);
    void *user_data;
} sce_test_trace_source_t;

/**
 * What `sce_test_run_suite_with_traces` holds while it runs: the source of the
 * traces, the suite each is held to, and the trace of the test case running and
 * its player. Placed by the caller, which outlives the run, and reused case
 * after case.
 */
typedef struct sce_test_traces_s {
    sce_test_trace_source_t source;
    const char *suite;
    sce_test_input_trace_t trace;
    sce_test_input_trace_player_t player;
} sce_test_traces_t;

/* The player of the trace the source `*user_data` gives for a test case, read
   by the test case's configuration; NULL, why in `*why`, where the trace is not
   given, is of another suite or test case, or does not fit. */
static inline sce_test_system_adapter_t *sce_test_adapter_of_trace_(void *user_data, const sce_test_case_t *test_case,
                                                                    sce_saved_error_t *why) {
    sce_test_traces_t *traces = (sce_test_traces_t *)user_data;
    const char *text = NULL;
    size_t len = 0u;
    sce_saved_error_t refused;
    refused.message[0] = '\0';
    if (!traces->source.read(traces->source.user_data, test_case->name, &text, &len, &refused) ||
        !sce_test_input_trace_parse(&traces->trace, text, len, &refused)) {
        (void)sce_saved_refuse(why, "the trace is refused: %s", refused.message);
        return NULL;
    }
    if (strcmp(traces->trace.suite, traces->suite) != 0 || strcmp(traces->trace.test_case, test_case->name) != 0) {
        (void)sce_saved_refuse(why, "the trace is of the test case '%s' of the suite '%s'", traces->trace.test_case,
                               traces->trace.suite);
        return NULL;
    }
    if (!sce_test_input_trace_player_init(&traces->player, &traces->trace, test_case->inputs, &refused)) {
        (void)sce_saved_refuse(why, "the trace is refused: %s", refused.message);
        return NULL;
    }
    return &traces->player.adapter;
}

/**
 * Runs the test cases of `*suite` that `*world` selects, each against its
 * recorded input trace (docs/SCE_TTCN3_LANE.md §10.5).
 *
 * `sce_test_run_suite`, with the adapter of each test case a player of the trace
 * `traces` gives for its name (sce/test/input_trace.h), read by the test case's
 * configuration (`sce_test_case_t::inputs`), the trace and its player held in
 * `*storage`.
 *
 * A test case whose trace is not given, is of another suite or test case, or does
 * not fit its configuration (a port that is no system port of it, a type its
 * port does not take, a value that does not belong to its type) ends with
 * `error` (P13) before any component is created, its log line naming why. So
 * does one whose components read a module parameter the run has no value for, as
 * under `sce_test_run_suite`, before its trace is asked for.
 */
static inline bool sce_test_run_suite_with_traces(const sce_test_suite_t *suite, const sce_test_features_t *world,
                                                  sce_test_trace_source_t traces,
                                                  const sce_test_module_parameters_t *overrides,
                                                  sce_test_case_run_t *run, sce_test_traces_t *storage,
                                                  sce_test_case_result_fn report, void *user_data,
                                                  sce_test_override_refusal_t *refusal) {
    storage->source = traces;
    storage->suite = suite->name;
    return sce_test_run_selected_(suite, world, overrides, sce_test_adapter_of_trace_, storage, run, report, user_data,
                                  refusal);
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_TEST_SUITE_H */
