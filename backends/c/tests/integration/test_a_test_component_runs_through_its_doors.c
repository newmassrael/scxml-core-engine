// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Test components generated in C11 (docs/SCE_TEST_PROFILE.md §12.3, §12.6
// slice 6), run end to end by the profile's runtime, `sce_test_case_run_t`.
// The machines are the statecharts sce-codegen writes at build time from the
// fixtures under sce-build/tests/fixtures/test_profile/runtime/, beside the
// units of the records and templates they name, driven only through the doors
// of §12.3; the configurations, the factory of machines and the system adapter
// are written here.
//
// Each case is the twin of the Rust case of the same name in
// backends/rust/tests/tests/a_test_component_runs_through_its_doors.rs, written
// `test_<case>`, itself the twin of the C++ gtest in
// tests/integration/ATestComponentRunsThroughItsDoorsTest.cpp and of the Go,
// Kotlin and Python ones, with the same scenario, verdict, `NOW` and log.
//
// What is C's own: every component lives in storage of static duration this
// file owns and hands the run through `<machine>_component_make`, and takes
// back when the run releases it; a message's value is a record of the event
// schema's unit, named by its C type, which the system adapter here reads and
// writes as the generated components do. Nothing here allocates.

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include <sce/host_processor.h>
#include <sce/test/test_case_run.h>

#include "answerer_sm.h"
#include "asker_sm.h"
#include "coordinator_sm.h"
#include "idler_sm.h"
#include "job_reply.h"
#include "job_request.h"
#include "ptc_keeper_sm.h"

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

// ── The machines, from storage of their own ─────────────────────────────

// How many components of one type a run here holds at once.
#define POOL 2u

static asker_component_t askers[POOL];
static coordinator_component_t coordinators[POOL];
static answerer_component_t answerers[POOL];
static ptc_keeper_component_t ptc_keepers[POOL];
static idler_component_t idlers[POOL];

// The generated machine of each component type a configuration here names, from
// the first component of its type no run holds, and none for another type, which
// ends the run with `error`.
static sce_test_component_machine_t *machine_of(void *user_data, const char *component_type) {
    (void)user_data;
    for (size_t i = 0u; i < POOL; ++i) {
        sce_test_component_machine_t *made = NULL;
        if (strcmp(component_type, "asker") == 0) {
            made = asker_component_make(&askers[i]);
        } else if (strcmp(component_type, "coordinator") == 0) {
            made = coordinator_component_make(&coordinators[i]);
        } else if (strcmp(component_type, "answerer") == 0) {
            made = answerer_component_make(&answerers[i]);
        } else if (strcmp(component_type, "ptc_keeper") == 0) {
            made = ptc_keeper_component_make(&ptc_keepers[i]);
        } else if (strcmp(component_type, "idler") == 0) {
            made = idler_component_make(&idlers[i]);
        } else {
            return NULL;
        }
        if (made != NULL) {
            return made;
        }
    }
    return NULL;
}

// ── The system under test ───────────────────────────────────────────────

// How many messages the system under test here is sent, and answers, in a run.
#define MAX_SENT 4u

// The system under test on the mapped port `sut`: it answers each request,
// ANSWER_DELAY after it, with a reply of `code` for the requested job, or for
// `job` when `answers_job` is set. It records what it was sent.
typedef struct responder_s {
    sce_test_system_adapter_t adapter;
    uint32_t job;
    bool answers_job;
    uint32_t code;
    sce_test_message_t pending[MAX_SENT];
    sce_test_instant_t pending_at[MAX_SENT];
    size_t pending_head;
    size_t pending_count;
    const char *sent_port[MAX_SENT];
    char sent_event[MAX_SENT][SCE_MAX_ID_LEN];
    size_t sent_count;
    uint32_t requested[MAX_SENT];
    size_t requested_count;
} responder_t;

static void responder_send(sce_test_system_adapter_t *adapter, const char *system_port,
                           const sce_test_message_t *message, sce_test_instant_t now) {
    responder_t *self = (responder_t *)(void *)adapter;
    if (self->sent_count < MAX_SENT) {
        self->sent_port[self->sent_count] = system_port;
        (void)snprintf(self->sent_event[self->sent_count], sizeof(self->sent_event[0]), "%s", message->event);
        self->sent_count++;
    }
    const JobRequestPayload_t *request = SCE_TEST_MESSAGE_VALUE_AS(JobRequestPayload_t, message);
    if (request == NULL || self->requested_count >= MAX_SENT || self->pending_count >= MAX_SENT) {
        return;
    }
    self->requested[self->requested_count++] = request->job;
    JobReplyPayload_t reply;
    memset(&reply, 0, sizeof(reply));
    reply.job = self->answers_job ? self->job : request->job;
    reply.code = self->code;
    const size_t slot = self->pending_count++;
    CHECK(sce_test_message_set(&self->pending[slot], JOB_REPLY_EVENT_NAME, SCE_HOST_VALUE(JobReplyPayload_t, &reply)) ==
              SCE_TEST_MESSAGE_HELD,
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

static void responder_init(responder_t *responder, uint32_t job, bool answers_job, uint32_t code) {
    memset(responder, 0, sizeof(*responder));
    responder->adapter.vtable = &RESPONDER;
    responder->job = job;
    responder->answers_job = answers_job;
    responder->code = code;
}

// A system under test nothing is mapped to.
static void silent_send(sce_test_system_adapter_t *adapter, const char *system_port, const sce_test_message_t *message,
                        sce_test_instant_t now) {
    (void)adapter;
    (void)now;
    CHECK(false, "no port is mapped, so nothing reaches the system adapter, yet %s was sent on %s", message->event,
          system_port);
}

static bool silent_next_input(const sce_test_system_adapter_t *adapter, sce_test_instant_t *at) {
    (void)adapter;
    (void)at;
    return false;
}

static bool silent_take_input_due(sce_test_system_adapter_t *adapter, sce_test_instant_t now, const char **system_port,
                                  sce_test_message_t *message) {
    (void)adapter;
    (void)now;
    (void)system_port;
    (void)message;
    return false;
}

static const sce_test_system_adapter_vtable_t SILENT = {
    .send = silent_send,
    .next_input = silent_next_input,
    .take_input_due = silent_take_input_due,
    .release = NULL,
};

// ── The run and its log ─────────────────────────────────────────────────

static sce_test_case_run_t the_run;

static const sce_test_case_result_t *run_with(const sce_test_configuration_t *configuration,
                                              sce_test_system_adapter_t *adapter) {
    sce_test_machine_factory_t factory;
    factory.make = machine_of;
    factory.user_data = NULL;
    sce_test_case_run_init(&the_run, configuration, factory, adapter, NULL, NULL);
    return sce_test_case_run_run(&the_run);
}

// The index of the first record of `kind` by `component`, in `*at`; false when
// there is none.
static bool first_record(const sce_test_case_result_t *result, const char *component, sce_test_log_kind_t kind,
                         size_t *at) {
    for (size_t i = 0u; i < result->log_count; ++i) {
        if (strcmp(result->log[i].component, component) == 0 && result->log[i].kind == kind) {
            *at = i;
            return true;
        }
    }
    return false;
}

// The index of the first record of `kind` by `component`, which the log holds.
static size_t record(const sce_test_case_result_t *result, const char *component, sce_test_log_kind_t kind) {
    size_t at = 0u;
    CHECK(first_record(result, component, kind, &at), "%s logs a record of kind %d", component, (int)kind);
    return at;
}

// ── The asker against the system under test ─────────────────────────────

static const sce_test_declared_t ASKER_PORTS[] = {{"asker", "sut"}};
static const sce_test_declared_t ASKER_TIMERS[] = {{"asker", "reply"}};
static const sce_test_mapping_t ASKER_MAPPINGS[] = {{{SCE_TEST_MTC_NAME, "sut"}, "sut"}};

// The asker as the MTC, its port `sut` mapped to the system port `sut`.
static const sce_test_configuration_t ASKER_CONFIGURATION = {
    .mtc_type = "asker",
    .ports = ASKER_PORTS,
    .port_count = 1u,
    .timers = ASKER_TIMERS,
    .timer_count = 1u,
    .mappings = ASKER_MAPPINGS,
    .mapping_count = 1u,
};

// P6, P2, P4: the send carries the template's value bound to the component's
// variable, a reply its receive branch's template matches is taken when it
// arrives, and the content reads the reply as `_event.data`.
static void test_a_test_component_that_is_answered_with_a_matching_reply_passes(void) {
    static responder_t responder;
    responder_init(&responder, 0u, false, 0u);
    const sce_test_case_result_t *result = run_with(&ASKER_CONFIGURATION, &responder.adapter);

    CHECK(result->verdict == SCE_TEST_VERDICT_PASS, "verdict %s", sce_test_verdict_name(result->verdict));
    CHECK(result->ended_at == ANSWER_DELAY, "the reply is taken in the round it arrives in: ended at %lld",
          (long long)result->ended_at);
    CHECK(responder.sent_count == 1u && strcmp(responder.sent_port[0], "sut") == 0 &&
              strcmp(responder.sent_event[0], JOB_REQUEST_EVENT_NAME) == 0,
          "a message travels under its template's record's event");
    CHECK(responder.requested_count == 1u && responder.requested[0] == 7u, "the template's value, given `job`");
    (void)record(result, SCE_TEST_MTC_NAME, SCE_TEST_LOG_RECEIVE);
    // P8: the `<log>` of the branch's content is in the test log, with the
    // instant and the component, its label and the job it read.
    const size_t logged = record(result, SCE_TEST_MTC_NAME, SCE_TEST_LOG_LOG);
    CHECK(strcmp(result->log[logged].detail, "answered: 7") == 0, "the log reads `%s`", result->log[logged].detail);
    CHECK(result->log[logged].at == ANSWER_DELAY, "the log is recorded at %lld", (long long)result->log[logged].at);
}

// P2, P7, §7: a reply the template does not match enables no branch and is not
// removed, and the timer's timeout branch fails the test case.
static void
test_a_test_component_that_is_answered_with_a_reply_its_template_does_not_match_fails_when_its_timer_fires(void) {
    static responder_t responder;
    responder_init(&responder, 0u, false, 3u);
    const sce_test_case_result_t *result = run_with(&ASKER_CONFIGURATION, &responder.adapter);

    CHECK(result->verdict == SCE_TEST_VERDICT_FAIL, "verdict %s", sce_test_verdict_name(result->verdict));
    CHECK(result->ended_at == SECOND, "time advances to the timer's instant: ended at %lld",
          (long long)result->ended_at);
    size_t at = 0u;
    CHECK(!first_record(result, SCE_TEST_MTC_NAME, SCE_TEST_LOG_RECEIVE, &at),
          "a reply that does not match is never taken");
    (void)record(result, SCE_TEST_MTC_NAME, SCE_TEST_LOG_TIMEOUT);
}

// P4: the content reads the message the branch took: a reply of success for
// another job matches the template, which takes any job, and the content's
// comparison of the job it read fails the test case.
static void test_a_test_component_reads_the_reply_it_took_as_the_events_data(void) {
    static responder_t responder;
    responder_init(&responder, 8u, true, 0u);
    const sce_test_case_result_t *result = run_with(&ASKER_CONFIGURATION, &responder.adapter);

    CHECK(result->verdict == SCE_TEST_VERDICT_FAIL, "verdict %s", sce_test_verdict_name(result->verdict));
    CHECK(result->ended_at == ANSWER_DELAY, "ended at %lld", (long long)result->ended_at);
    (void)record(result, SCE_TEST_MTC_NAME, SCE_TEST_LOG_RECEIVE);
}

// ── A PTC created, served and done ──────────────────────────────────────

static const sce_test_ptc_t COORDINATOR_PTCS[] = {{"answerer", "answerer"}};
static const sce_test_declared_t COORDINATOR_PORTS[] = {{"coordinator", "peer"}, {"answerer", "peer"}};
static const sce_test_connection_t COORDINATOR_CONNECTIONS[] = {{{SCE_TEST_MTC_NAME, "peer"}, {"answerer", "peer"}}};

// The coordinator as the MTC and the answerer it invokes as a PTC, their two
// ports connected to each other.
static const sce_test_configuration_t COORDINATOR_CONFIGURATION = {
    .mtc_type = "coordinator",
    .ptcs = COORDINATOR_PTCS,
    .ptc_count = 1u,
    .ports = COORDINATOR_PORTS,
    .port_count = 2u,
    .connections = COORDINATOR_CONNECTIONS,
    .connection_count = 1u,
};

// P9, P6, P2: the MTC creates a PTC connected to it, which asks it for job 42
// as it starts; the MTC answers with the job it read, the PTC passes on the
// reply and terminates, and the MTC's done branch passes on the next snapshot.
// Leaving the invoking state kills the PTC, before the done branch's content
// runs.
static void test_a_test_component_creates_a_ptc_exchanges_a_message_with_it_and_reads_its_done(void) {
    sce_test_system_adapter_t silent = {&SILENT};
    const sce_test_case_result_t *result = run_with(&COORDINATOR_CONFIGURATION, &silent);

    CHECK(result->verdict == SCE_TEST_VERDICT_PASS, "verdict %s", sce_test_verdict_name(result->verdict));
    CHECK(result->ended_at == 0, "a branch fired in every round, so time never moved: ended at %lld",
          (long long)result->ended_at);
    const size_t created = record(result, "answerer", SCE_TEST_LOG_CREATE);
    const size_t asked = record(result, "answerer", SCE_TEST_LOG_SEND);
    const size_t answered = record(result, SCE_TEST_MTC_NAME, SCE_TEST_LOG_SEND);
    const size_t done = record(result, "answerer", SCE_TEST_LOG_DONE);
    const size_t killed = record(result, "answerer", SCE_TEST_LOG_KILL);
    const size_t passed = record(result, SCE_TEST_MTC_NAME, SCE_TEST_LOG_VERDICT);
    CHECK(created < asked, "the PTC is created when the macrostep that entered its state ends");
    // P8: a `<log>` that writes only a label is recorded, by the PTC.
    const size_t noted = record(result, "answerer", SCE_TEST_LOG_LOG);
    CHECK(strcmp(result->log[noted].detail, "asked") == 0, "the PTC's log reads `%s`", result->log[noted].detail);
    CHECK(asked < noted, "the log runs in its place, after the send before it");
    CHECK(strcmp(result->log[asked].detail, "peer job.request") == 0, "the PTC sends `%s`", result->log[asked].detail);
    CHECK(strcmp(result->log[answered].detail, "peer job.reply") == 0, "the MTC sends `%s`",
          result->log[answered].detail);
    CHECK(answered < done, "the MTC answers before the PTC is done");
    CHECK(done < killed, "a done branch fires on a snapshot taken after the PTC terminated");
    CHECK(killed < passed, "the invoking state is exited before the done branch's content runs");
}

// ── A PTC removed while it runs ─────────────────────────────────────────

static const sce_test_ptc_t KEEPING_PTCS[] = {{"idler", "idler"}};
static const sce_test_declared_t KEEPING_PORTS[] = {{"idler", "peer"}};
static const sce_test_declared_t KEEPING_TIMERS[] = {{"ptc_keeper", "patience"}, {"ptc_keeper", "linger"}};

// ptc_keeper as the MTC and the idler it invokes as a PTC, whose port takes
// part in nothing, since nothing is sent on it.
static const sce_test_configuration_t KEEPING_CONFIGURATION = {
    .mtc_type = "ptc_keeper",
    .ptcs = KEEPING_PTCS,
    .ptc_count = 1u,
    .ports = KEEPING_PORTS,
    .port_count = 1u,
    .timers = KEEPING_TIMERS,
    .timer_count = 2u,
};

// P9, P14: leaving the state that invoked a PTC removes it while it still runs,
// at that instant, and its local verdict still counts.
static void test_a_test_component_kills_a_ptc_on_leaving_the_state_that_invoked_it(void) {
    sce_test_system_adapter_t silent = {&SILENT};
    const sce_test_case_result_t *result = run_with(&KEEPING_CONFIGURATION, &silent);

    const size_t killed = record(result, "idler", SCE_TEST_LOG_KILL);
    CHECK(result->log[killed].at == 10 * MILLISECOND, "the PTC is removed when `patience` runs out, not at %lld",
          (long long)result->log[killed].at);
    size_t at = 0u;
    CHECK(!first_record(result, "idler", SCE_TEST_LOG_DONE, &at), "the idler never terminates");
    CHECK(result->ended_at == 15 * MILLISECOND, "ended at %lld", (long long)result->ended_at);
    CHECK(result->verdict == SCE_TEST_VERDICT_INCONCLUSIVE,
          "the removed PTC's inconclusive outweighs the MTC's pass: %s", sce_test_verdict_name(result->verdict));
}

// Every component a run took is handed back when it ends, so the storage of
// each type is free for the next run.
static void every_component_is_handed_back(void) {
    for (size_t i = 0u; i < POOL; ++i) {
        CHECK(!askers[i].held && !coordinators[i].held && !answerers[i].held && !ptc_keepers[i].held && !idlers[i].held,
              "the run hands every component back when it ends");
    }
}

int main(void) {
    test_a_test_component_that_is_answered_with_a_matching_reply_passes();
    every_component_is_handed_back();
    test_a_test_component_that_is_answered_with_a_reply_its_template_does_not_match_fails_when_its_timer_fires();
    every_component_is_handed_back();
    test_a_test_component_reads_the_reply_it_took_as_the_events_data();
    every_component_is_handed_back();
    test_a_test_component_creates_a_ptc_exchanges_a_message_with_it_and_reads_its_done();
    every_component_is_handed_back();
    test_a_test_component_kills_a_ptc_on_leaving_the_state_that_invoked_it();
    every_component_is_handed_back();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)printf("a test component runs through its doors: all checks passed\n");
    return 0;
}
