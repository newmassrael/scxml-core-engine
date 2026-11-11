// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A test suite generated in C11 (docs/SCE_TEST_PROFILE.md §11.6 to §11.8,
// §12.2, §12.4, §12.6 slice 6), run end to end by the profile's runner,
// `sce_test_run_suite`. The suite, its test cases and their configurations are
// the tables sce-codegen writes at build time from the fixtures under
// sce-build/tests/fixtures/test_profile/runtime/, and the components are the
// machines it writes from the components there. Only the system adapter, which
// plays the system under test, is written here; and the suite is replayed from
// its committed recorded input traces too, through
// `sce_test_run_suite_with_traces` and the configuration's `_input`
// (docs/SCE_TTCN3_LANE.md §10.5).
//
// Each case is the twin of the C++ test of the same name in
// tests/integration/ATestSuiteRunsEndToEndTest.cpp, written `test_<case>`,
// with the same worlds, verdicts, `NOW` and log.
//
// What is C's own: the suite's storage, which holds the suite, its entries and
// the machines of its test cases, is one struct of static duration this file
// places; the runner hands each result to a function here as the case ends,
// which keeps what the checks read of it; and every component a run took is
// handed back to that storage when its run ends. Nothing here allocates.

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include <sce/host_processor.h>
#include <sce/saved_json.h>
#include <sce/test/input_trace.h>
#include <sce/test/runner.h>
#include <sce/test/suite.h>

#include "answers_the_job.h"
#include "asking_configuration.h"
#include "job_reply.h"
#include "job_request.h"
#include "reply_ok.h"
#include "runtime_suite.h"

#ifndef SCE_PROJECT_ROOT
#error "SCE_PROJECT_ROOT names the repository's root, which the shared test log is read from; the build defines it"
#endif

static int failures = 0;

#define CHECK(condition, ...)                                                                                          \
    do {                                                                                                               \
        if (!(condition)) {                                                                                            \
            (void)fprintf(stderr, "FAIL %s:%d: ", __FILE__, __LINE__);                                                 \
            (void)fprintf(stderr, __VA_ARGS__);                                                                        \
            (void)fputc('\n', stderr);                                                                                 \
            ++failures;                                                                                                \
        }                                                                                                              \
    } while (0)

// A millisecond of `NOW`, which counts microseconds.
#define MILLISECOND ((sce_test_duration_t)1000)

// A second of `NOW`.
#define SECOND (1000 * MILLISECOND)

// How long the system under test takes to answer a request.
#define ANSWER_DELAY ((sce_test_duration_t)500)

// How many test cases the suite holds, so how many one run of it reports.
#define MAX_CASES 4u

// How many messages the system under test is sent, and answers, in a test case.
#define MAX_SENT 4u

// ── The system under test ───────────────────────────────────────────────

// The jobs the system under test was asked for, over every test case of a run
// of the suite.
typedef struct requests_s {
    uint32_t jobs[MAX_CASES * MAX_SENT];
    size_t count;
} requests_t;

// The system under test on the system port `sut`: it answers each request,
// ANSWER_DELAY after it, with a reply of `code` for the job the request names,
// and records each job it was asked for.
typedef struct responder_s {
    sce_test_system_adapter_t adapter;
    uint32_t code;
    requests_t *requested;
    sce_test_message_t pending[MAX_SENT];
    sce_test_instant_t pending_at[MAX_SENT];
    size_t pending_head;
    size_t pending_count;
} responder_t;

static void responder_send(sce_test_system_adapter_t *adapter, const char *system_port,
                           const sce_test_message_t *message, sce_test_instant_t now) {
    responder_t *self = (responder_t *)(void *)adapter;
    CHECK(strcmp(system_port, "sut") == 0, "a request is sent on `sut`, not `%s`", system_port);
    const JobRequestPayload_t *request = SCE_TEST_MESSAGE_VALUE_AS(JobRequestPayload_t, message);
    CHECK(request != NULL, "a request travels as its record's value");
    if (request == NULL || self->pending_count >= MAX_SENT ||
        self->requested->count >= sizeof(self->requested->jobs) / sizeof(self->requested->jobs[0])) {
        return;
    }
    self->requested->jobs[self->requested->count++] = request->job;
    JobReplyPayload_t reply;
    memset(&reply, 0, sizeof(reply));
    reply.job = request->job;
    reply.code = self->code;
    const size_t slot = self->pending_count++;
    // The reply's record type in the test log is the one the unit of a
    // template over its record gives.
    CHECK(sce_test_message_set(&self->pending[slot], JOB_REPLY_EVENT_NAME,
                               sce_test_message_value(SCE_HOST_VALUE(JobReplyPayload_t, &reply),
                                                      reply_ok_record_type())) == SCE_TEST_MESSAGE_HELD,
          "the reply fits a message");
    self->pending_at[slot] = now + ANSWER_DELAY;
}

static bool responder_next_input(const sce_test_system_adapter_t *adapter, sce_test_instant_t *at) {
    const responder_t *self = (const responder_t *)(const void *)adapter;
    if (self->pending_head >= self->pending_count) {
        return false;
    }
    *at = self->pending_at[self->pending_head];
    return true;
}

static bool responder_take_input_due(sce_test_system_adapter_t *adapter, sce_test_instant_t now,
                                     const char **system_port, sce_test_message_t *message) {
    responder_t *self = (responder_t *)(void *)adapter;
    if (self->pending_head >= self->pending_count || self->pending_at[self->pending_head] > now) {
        return false;
    }
    *system_port = "sut";
    *message = self->pending[self->pending_head++];
    return true;
}

static const sce_test_system_adapter_vtable_t RESPONDER = {
    .send = responder_send,
    .next_input = responder_next_input,
    .take_input_due = responder_take_input_due,
    .release = NULL,
};

// The responders of one run of the suite, a fresh one for each test case, each
// answering with `code` and recording into `requested`.
typedef struct responders_s {
    responder_t pool[MAX_CASES];
    size_t made;
    uint32_t code;
    requests_t requested;
} responders_t;

static sce_test_system_adapter_t *responder_for(void *user_data, const char *test_case) {
    responders_t *responders = (responders_t *)user_data;
    (void)test_case;
    if (responders->made >= MAX_CASES) {
        return NULL;
    }
    responder_t *made = &responders->pool[responders->made++];
    memset(made, 0, sizeof(*made));
    made->adapter.vtable = &RESPONDER;
    made->code = responders->code;
    made->requested = &responders->requested;
    return &made->adapter;
}

// A fresh responder for each test case of a run, answering with `code`.
static sce_test_adapter_factory_t responders_answering(responders_t *storage, uint32_t code) {
    memset(storage, 0, sizeof(*storage));
    storage->code = code;
    sce_test_adapter_factory_t factory;
    factory.make = responder_for;
    factory.user_data = storage;
    return factory;
}

// ── The results the runner reports ──────────────────────────────────────

// How many bytes the test log of one test case of the suite takes, kept.
#define MAX_TEST_LOG 2048u

// What the checks read of one test case's result, kept as the runner reports it:
// its test log is written in the call that reports it, while the result lives.
typedef struct case_result_s {
    const char *test_case;
    sce_test_verdict_t verdict;
    sce_test_instant_t ended_at;
    bool logged_error;
    bool logged_timeout;
    // The first line of its log, which names why a test case that cannot start
    // ends in error.
    char first_line[SCE_TEST_MAX_LOG_DETAIL];
    sce_test_trace_written_t written;
    char test_log[MAX_TEST_LOG];
    size_t test_log_len;
} case_result_t;

typedef struct results_s {
    // The suite's name, which each test log names.
    const char *suite;
    case_result_t of[MAX_CASES];
    size_t count;
} results_t;

static bool logged(const sce_test_case_result_t *result, sce_test_log_kind_t kind) {
    for (size_t i = 0u; i < result->log_count; ++i) {
        if (result->log[i].kind == kind) {
            return true;
        }
    }
    return false;
}

static void keep(void *user_data, const char *test_case, const sce_test_case_result_t *result) {
    results_t *results = (results_t *)user_data;
    CHECK(results->count < MAX_CASES, "the suite reports no more than its %u test cases", MAX_CASES);
    if (results->count >= MAX_CASES) {
        return;
    }
    case_result_t *kept = &results->of[results->count++];
    kept->test_case = test_case;
    kept->verdict = result->verdict;
    kept->ended_at = result->ended_at;
    kept->logged_error = logged(result, SCE_TEST_LOG_ERROR);
    kept->logged_timeout = logged(result, SCE_TEST_LOG_TIMEOUT);
    kept->first_line[0] = '\0';
    if (result->log_count > 0u) {
        (void)snprintf(kept->first_line, sizeof(kept->first_line), "%s", result->log[0].detail);
    }
    kept->written = sce_test_case_result_canonical_log(result, results->suite, test_case, kept->test_log,
                                                       sizeof(kept->test_log), &kept->test_log_len);
}

// ── The run ─────────────────────────────────────────────────────────────

static runtime_suite_storage_t suite_storage;
static sce_test_case_run_t the_run;
static responders_t the_responders;

// Runs the suite in the world of the features `names`, against responders
// answering with `code`, with `overrides` (NULL for none) in place of the
// defaults, its results in `*results`. Whether the runner ran it.
static bool run_suite(const char *const *names, size_t count, uint32_t code,
                      const sce_test_module_parameters_t *overrides, results_t *results) {
    sce_test_features_t world;
    world.names = names;
    world.count = count;
    memset(results, 0, sizeof(*results));
    const sce_test_suite_t *suite = runtime_suite_suite(&suite_storage);
    results->suite = suite->name;
    return sce_test_run_suite(suite, &world, responders_answering(&the_responders, code), overrides, &the_run, keep,
                              results, NULL);
}

// Whether the run reported exactly the test cases `names`, in that order.
static bool reported(const results_t *results, const char *const *names, size_t count) {
    if (results->count != count) {
        return false;
    }
    for (size_t i = 0u; i < count; ++i) {
        if (strcmp(results->of[i].test_case, names[i]) != 0) {
            return false;
        }
    }
    return true;
}

// Every component a run of the suite took is handed back when its run ends, so
// the storage of each test case's machines is free for the next run.
static void every_component_is_handed_back(void) {
    CHECK(!suite_storage.answers_the_job_machines.param_asker_components[0].held &&
              !suite_storage.answers_before_the_guard_machines.param_asker_components[0].held &&
              !suite_storage.serves_a_ptc_machines.coordinator_components[0].held &&
              !suite_storage.serves_a_ptc_machines.answerer_components[0].held &&
              !suite_storage.keeps_a_ptc_machines.ptc_keeper_components[0].held &&
              !suite_storage.keeps_a_ptc_machines.idler_components[0].held,
          "the run hands every component back when it ends");
}

// docs/adr/0045: each component's session is named by the component, as on every
// other backend: the MTC's is `mtc` and a PTC's the name its configuration gives
// it, not an id the machine drew (`<machine>_session_<n>`). Read off the storage
// the run handed back, which keeps the machine it tore down.
static void each_component_runs_under_its_configuration_name(void) {
    const coordinator_t *served_mtc = &suite_storage.serves_a_ptc_machines.coordinator_components[0].machine;
    const answerer_t *served_ptc = &suite_storage.serves_a_ptc_machines.answerer_components[0].machine;
    const ptc_keeper_t *keeping_mtc = &suite_storage.keeps_a_ptc_machines.ptc_keeper_components[0].machine;
    const idler_t *kept_ptc = &suite_storage.keeps_a_ptc_machines.idler_components[0].machine;
    CHECK(strcmp(answerer_session_id(served_ptc), "answerer") == 0, "the PTC `answerer` ran as the session `%s`",
          answerer_session_id(served_ptc));
    CHECK(strcmp(answerer_scxml_location(served_ptc), "sce://scxml/answerer") == 0, "the PTC `answerer` published `%s`",
          answerer_scxml_location(served_ptc));
    CHECK(strcmp(idler_session_id(kept_ptc), "idler") == 0, "the PTC `idler` ran as the session `%s`",
          idler_session_id(kept_ptc));
    CHECK(strcmp(coordinator_session_id(served_mtc), "mtc") == 0, "the MTC ran as the session `%s`",
          coordinator_session_id(served_mtc));
    CHECK(strcmp(ptc_keeper_session_id(keeping_mtc), "mtc") == 0, "the MTC ran as the session `%s`",
          ptc_keeper_session_id(keeping_mtc));
}

static const char *const EVERY_FEATURE[] = {"ptcs", "removal", "guard"};

// P11, P12, P13, P14: in a world with every feature the suite runs every test
// case in its order, and each ends with the verdict it expects: a job answered
// in time passes, a PTC served passes, a removed PTC's inconclusive outweighs
// its MTC's pass, and a guard shorter than the answer ends in error.
static void test_runs_every_test_case_of_a_world_with_the_verdict_it_expects(void) {
    static results_t results;
    CHECK(run_suite(EVERY_FEATURE, 3u, 0u, NULL, &results), "the suite runs");
    static const char *const ALL[] = {"answers_the_job", "serves_a_ptc", "keeps_a_ptc", "answers_before_the_guard"};
    CHECK(reported(&results, ALL, 4u), "every test case runs, in the suite's order");
    if (results.count != 4u) {
        return;
    }
    CHECK(results.of[0].verdict == SCE_TEST_VERDICT_PASS, "verdict %s", sce_test_verdict_name(results.of[0].verdict));
    CHECK(results.of[0].ended_at == ANSWER_DELAY, "the reply is taken in the round it arrives in: ended at %lld",
          (long long)results.of[0].ended_at);
    CHECK(results.of[1].verdict == SCE_TEST_VERDICT_PASS, "verdict %s", sce_test_verdict_name(results.of[1].verdict));
    CHECK(results.of[1].ended_at == 0, "a branch fired in every round, so time never moved: ended at %lld",
          (long long)results.of[1].ended_at);
    CHECK(results.of[2].verdict == SCE_TEST_VERDICT_INCONCLUSIVE, "verdict %s",
          sce_test_verdict_name(results.of[2].verdict));
    CHECK(results.of[2].ended_at == 15 * MILLISECOND, "ended at %lld", (long long)results.of[2].ended_at);
    CHECK(results.of[3].verdict == SCE_TEST_VERDICT_ERROR, "verdict %s", sce_test_verdict_name(results.of[3].verdict));
    CHECK(results.of[3].ended_at == 100, "the guard, 0.1ms, ends the case before the answer: ended at %lld",
          (long long)results.of[3].ended_at);
    CHECK(results.of[3].logged_error, "the guard's end is logged as an error");
    CHECK(the_responders.requested.count == 2u && the_responders.requested.jobs[0] == 7u &&
              the_responders.requested.jobs[1] == 7u,
          "each asking case asks for the job the suite's module parameter gives");
}

// P2, P7: a system under test that answers with a failure code fails the test
// case when its timer runs out. A world with no feature runs only the test
// case no selection guards.
static void test_fails_a_job_the_system_under_test_answers_with_a_failure(void) {
    static results_t results;
    CHECK(run_suite(NULL, 0u, 3u, NULL, &results), "the suite runs");
    static const char *const ONLY[] = {"answers_the_job"};
    CHECK(reported(&results, ONLY, 1u), "a world with no feature runs the unguarded case alone");
    if (results.count != 1u) {
        return;
    }
    CHECK(results.of[0].verdict == SCE_TEST_VERDICT_FAIL, "verdict %s", sce_test_verdict_name(results.of[0].verdict));
    CHECK(results.of[0].ended_at == SECOND, "time advances to the timer's instant: ended at %lld",
          (long long)results.of[0].ended_at);
    CHECK(results.of[0].logged_timeout, "the timer's timeout is logged");
}

// The test log every runtime writes of `test_case` in a world with no feature,
// answered with success: one file the C11, the C++ and the Rust runs are all
// held to (`a_generated_test_case_writes_its_test_log` in
// backends/rust/tests/tests/a_test_suite_runs_through_its_runner.rs), so the
// agreement of the backends is one set of bytes. The file ends in a newline,
// which no canonical text holds, so it is read without it. False when it cannot
// be read whole, or does not end in one newline.
static bool shared_log(const char *test_case, char *out, size_t cap, size_t *len) {
    char path[1024];
    (void)snprintf(path, sizeof(path), "%s/sce-build/tests/fixtures/test_profile/runtime/expected_logs/%s.json",
                   SCE_PROJECT_ROOT, test_case);
    FILE *file = fopen(path, "rb");
    if (file == NULL) {
        return false;
    }
    const size_t n = fread(out, 1u, cap - 1u, file);
    (void)fclose(file);
    if (n == 0u || n >= cap - 1u || out[n - 1u] != '\n') {
        return false;
    }
    out[n - 1u] = '\0';
    *len = n - 1u;
    return true;
}

// docs/SCE_TEST_PROFILE.md §12.5: the test log of a generated test case, in its
// one canonical text, byte for byte the one the Rust and the C++ runtimes write
// of the same run. The request the component sends carries the value and the
// record type its template's unit gives, the job of the suite's module
// parameter; the reply arrives at 500 and is taken in that round, which stops
// the timer, passes and ends the MTC.
static void test_a_generated_test_case_writes_its_test_log(void) {
    static results_t results;
    CHECK(run_suite(NULL, 0u, 0u, NULL, &results), "the suite runs");
    static const char *const ONLY[] = {"answers_the_job"};
    CHECK(reported(&results, ONLY, 1u), "a world with no feature runs the unguarded case alone");
    if (results.count != 1u) {
        return;
    }
    static char want[MAX_TEST_LOG];
    size_t want_len = 0u;
    if (!shared_log("answers_the_job", want, sizeof(want), &want_len)) {
        CHECK(false, "the shared test log is readable under %s and ends in one newline", SCE_PROJECT_ROOT);
        return;
    }
    const case_result_t *answered = &results.of[0];
    CHECK(answered->written == SCE_TEST_TRACE_WRITTEN, "the test log is written: %d", (int)answered->written);
    CHECK(answered->test_log_len == want_len && memcmp(answered->test_log, want, want_len) == 0,
          "the test log is the shared one:\n  wrote %s\n  want  %s", answered->test_log, want);
}

// ── The suite replayed from its recorded input traces ─────────────────────

// What a run of the suite against traces holds: the trace of the case running
// and its player, and the text of the trace read last.
static sce_test_traces_t the_traces;
static sce_test_trace_directory_t the_trace_directory;

// The directory of the suite's committed traces (docs/SCE_TTCN3_LANE.md
// §10.2), `<test case>.json` each.
static const char *traces_dir(void) {
    static char dir[1024];
    (void)snprintf(dir, sizeof(dir), "%s/sce-build/tests/fixtures/test_profile/runtime/traces", SCE_PROJECT_ROOT);
    return dir;
}

// Runs the suite in a world with no feature against the traces `traces` gives,
// its results in `*results`. Whether the runner ran it.
static bool run_suite_with_traces(sce_test_trace_source_t traces, results_t *results) {
    sce_test_features_t world;
    world.names = NULL;
    world.count = 0u;
    memset(results, 0, sizeof(*results));
    const sce_test_suite_t *suite = runtime_suite_suite(&suite_storage);
    results->suite = suite->name;
    return sce_test_run_suite_with_traces(suite, &world, traces, NULL, &the_run, &the_traces, keep, results, NULL);
}

// docs/SCE_TTCN3_LANE.md §10.5: the generated suite replayed from the committed
// trace of `answers_the_job` writes the test log every runtime writes of it, byte
// for byte: the configuration's header reads the reply from its canonical form
// and writes it back the same, and the clock advances to the trace's instant as
// it did to the responder's. The twin of the C++
// `ARecordedTraceReplaysToTheTestLogEveryRuntimeWrites` and the Rust
// `a_recorded_trace_replays_to_the_test_log_every_runtime_writes`.
static void test_a_recorded_trace_replays_to_the_test_log_every_runtime_writes(void) {
    static results_t results;
    CHECK(run_suite_with_traces(sce_test_trace_directory(&the_trace_directory, traces_dir()), &results),
          "the suite runs");
    static const char *const ONLY[] = {"answers_the_job"};
    CHECK(reported(&results, ONLY, 1u), "a world with no feature runs the unguarded case alone");
    if (results.count != 1u) {
        return;
    }
    CHECK(results.of[0].verdict == SCE_TEST_VERDICT_PASS, "verdict %s: %s",
          sce_test_verdict_name(results.of[0].verdict), results.of[0].first_line);
    static char want[MAX_TEST_LOG];
    size_t want_len = 0u;
    if (!shared_log("answers_the_job", want, sizeof(want), &want_len)) {
        CHECK(false, "the shared test log is readable under %s and ends in one newline", SCE_PROJECT_ROOT);
        return;
    }
    const case_result_t *replayed = &results.of[0];
    CHECK(replayed->written == SCE_TEST_TRACE_WRITTEN, "the test log is written: %d", (int)replayed->written);
    CHECK(replayed->test_log_len == want_len && memcmp(replayed->test_log, want, want_len) == 0,
          "the test log is the shared one:\n  wrote %s\n  want  %s", replayed->test_log, want);
}

// The one text a fixed source gives every test case.
typedef struct fixed_trace_s {
    const char *text;
} fixed_trace_t;

static bool fixed_trace_read(void *user_data, const char *test_case, const char **text, size_t *len,
                             sce_saved_error_t *err) {
    const fixed_trace_t *fixed = (const fixed_trace_t *)user_data;
    (void)test_case;
    (void)err;
    *text = fixed->text;
    *len = strlen(fixed->text);
    return true;
}

// The source that gives every test case the trace `*fixed` holds.
static sce_test_trace_source_t fixed_trace(fixed_trace_t *fixed) {
    sce_test_trace_source_t source;
    source.read = fixed_trace_read;
    source.user_data = fixed;
    return source;
}

// §10.2, P13: a trace one of whose inputs the configuration does not take, or a
// trace of another test case, ends the test case in error before any component
// is created, naming why; the test case reads its trace through its
// configuration's `_input`. The twin of the C++ `EndsInErrorWhenItsTraceDoesNotFit`.
static void test_ends_in_error_when_its_trace_does_not_fit(void) {
    static answers_the_job_machines_t machines;
    CHECK(answers_the_job_test_case(&machines).inputs == asking_configuration_input,
          "the test case reads its trace through its configuration's input");
    static results_t results;
    static fixed_trace_t unmapped = {"{\"inputs\":[{\"at\":\"500\",\"port\":\"oracle\",\"type\":\"job_reply\","
                                     "\"value\":{\"code\":\"0\",\"job\":\"7\"}}],\"schema\":\"sce-test-trace.v1\","
                                     "\"suite\":\"runtime_suite\",\"test_case\":\"answers_the_job\"}"};
    CHECK(run_suite_with_traces(fixed_trace(&unmapped), &results), "the suite runs");
    CHECK(results.count == 1u, "one test case runs, not %lu", (unsigned long)results.count);
    if (results.count == 1u) {
        CHECK(results.of[0].verdict == SCE_TEST_VERDICT_ERROR, "verdict %s",
              sce_test_verdict_name(results.of[0].verdict));
        CHECK(strcmp(results.of[0].first_line, "the trace is refused: input 0, at 500: 'oracle' is not a system port "
                                               "of the test case's configuration") == 0,
              "%s", results.of[0].first_line);
    }
    static fixed_trace_t other = {"{\"inputs\":[],\"schema\":\"sce-test-trace.v1\",\"suite\":\"runtime_suite\","
                                  "\"test_case\":\"serves_a_ptc\"}"};
    CHECK(run_suite_with_traces(fixed_trace(&other), &results), "the suite runs");
    CHECK(results.count == 1u, "one test case runs, not %lu", (unsigned long)results.count);
    if (results.count == 1u) {
        CHECK(results.of[0].verdict == SCE_TEST_VERDICT_ERROR, "verdict %s",
              sce_test_verdict_name(results.of[0].verdict));
        CHECK(strcmp(results.of[0].first_line,
                     "the trace is of the test case 'serves_a_ptc' of the suite 'runtime_suite'") == 0,
              "%s", results.of[0].first_line);
    }
}

// §11.8: a world runs the test cases whose selection it satisfies, in the
// suite's order: `ptcs` alone selects the served PTC and not the removed one,
// and `guard` selects the guarded case.
static void test_runs_the_test_cases_a_world_selects(void) {
    static results_t results;
    static const char *const PTCS[] = {"ptcs"};
    CHECK(run_suite(PTCS, 1u, 0u, NULL, &results), "the suite runs");
    static const char *const SERVED[] = {"answers_the_job", "serves_a_ptc"};
    CHECK(reported(&results, SERVED, 2u), "`ptcs` selects the served PTC and not the removed one");
    every_component_is_handed_back();
    static const char *const GUARD[] = {"guard"};
    CHECK(run_suite(GUARD, 1u, 0u, NULL, &results), "the suite runs");
    static const char *const GUARDED[] = {"answers_the_job", "answers_before_the_guard"};
    CHECK(reported(&results, GUARDED, 2u), "`guard` selects the guarded case");
}

// -1 §8.2.1, §11.5: a value the caller gives the module parameter replaces the
// suite's default, and the component asks for the job it names.
static void test_asks_for_the_job_an_override_of_its_module_parameter_names(void) {
    static results_t results;
    sce_test_module_parameters_t overrides;
    sce_test_module_parameters_init(&overrides);
    CHECK(sce_test_module_parameters_set(&overrides, "job", "9"), "the table has room");
    CHECK(run_suite(NULL, 0u, 0u, &overrides, &results), "the suite declares the override's parameter");
    CHECK(results.count == 1u, "one test case runs, not %zu", results.count);
    if (results.count != 1u) {
        return;
    }
    CHECK(results.of[0].verdict == SCE_TEST_VERDICT_PASS, "verdict %s", sce_test_verdict_name(results.of[0].verdict));
    CHECK(the_responders.requested.count == 1u && the_responders.requested.jobs[0] == 9u,
          "the component asks for the job the override names");
}

// P13: a value of the module parameter that is not of the type the component
// declares ends the test case in error before the component starts.
static void test_ends_in_error_when_a_module_parameter_is_not_of_its_type(void) {
    static results_t results;
    sce_test_module_parameters_t overrides;
    sce_test_module_parameters_init(&overrides);
    CHECK(sce_test_module_parameters_set(&overrides, "job", "\"seven\""), "the table has room");
    CHECK(run_suite(NULL, 0u, 0u, &overrides, &results), "the suite declares the override's parameter");
    CHECK(results.count == 1u, "one test case runs, not %zu", results.count);
    if (results.count != 1u) {
        return;
    }
    CHECK(results.of[0].verdict == SCE_TEST_VERDICT_ERROR, "verdict %s", sce_test_verdict_name(results.of[0].verdict));
    CHECK(results.of[0].logged_error, "the refused read is logged as an error");
    CHECK(the_responders.requested.count == 0u, "the component did not start, so it sent nothing");
}

// -1 §8.2.1: an override of a module parameter the suite does not declare is
// the caller's mistake, refused before any test case runs.
static void test_refuses_an_override_of_a_module_parameter_the_suite_does_not_declare(void) {
    static results_t results;
    sce_test_module_parameters_t overrides;
    sce_test_module_parameters_init(&overrides);
    CHECK(sce_test_module_parameters_set(&overrides, "limit", "1"), "the table has room");
    CHECK(!run_suite(EVERY_FEATURE, 3u, 0u, &overrides, &results), "the override is refused");
    CHECK(results.count == 0u, "no test case runs");
}

// §11.6: the configuration's tables name the module parameters its components
// read, which a suite that runs it must give, and its factory makes no machine
// of a type it does not name.
static void test_reads_the_module_parameters_its_configuration_names(void) {
    CHECK(asking_configuration_module_parameter_count() == 1u &&
              strcmp(asking_configuration_module_parameters()[0], "job") == 0,
          "the configuration's component reads `job`");
    CHECK(strcmp(asking_configuration_configuration()->mtc_type, "param_asker") == 0, "the MTC is the param_asker");
    static asking_configuration_machines_t machines;
    const sce_test_machine_factory_t factory = asking_configuration_machines(&machines);
    CHECK(factory.make(factory.user_data, "nobody") == NULL, "no machine of a type the configuration does not name");
}

int main(void) {
    test_runs_every_test_case_of_a_world_with_the_verdict_it_expects();
    every_component_is_handed_back();
    each_component_runs_under_its_configuration_name();
    test_fails_a_job_the_system_under_test_answers_with_a_failure();
    every_component_is_handed_back();
    test_a_generated_test_case_writes_its_test_log();
    every_component_is_handed_back();
    test_a_recorded_trace_replays_to_the_test_log_every_runtime_writes();
    every_component_is_handed_back();
    test_ends_in_error_when_its_trace_does_not_fit();
    every_component_is_handed_back();
    test_runs_the_test_cases_a_world_selects();
    every_component_is_handed_back();
    test_asks_for_the_job_an_override_of_its_module_parameter_names();
    every_component_is_handed_back();
    test_ends_in_error_when_a_module_parameter_is_not_of_its_type();
    every_component_is_handed_back();
    test_refuses_an_override_of_a_module_parameter_the_suite_does_not_declare();
    test_reads_the_module_parameters_its_configuration_names();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)printf("a test suite runs through its runner: all checks passed\n");
    return 0;
}
