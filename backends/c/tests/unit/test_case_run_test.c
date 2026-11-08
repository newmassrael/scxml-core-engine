// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The test profile's runtime on the C11 backend (docs/SCE_TEST_PROFILE.md
// §12.2, sce/test/), held to the rules of §2 to §5 with components written here
// by hand, before any machine is generated: each one keeps its context from
// `start` on, reads its snapshot and calls its context as a generated machine
// does (§12.3). The runner of a suite is held to P11 over test cases made of
// the same components.
//
// Each case is the twin of the Rust case of the same module and name in
// backends/rust/runtime/tests/a_test_case_runs_as_the_test_profile_says.rs,
// named `<module>_<case>` (`test_case_run_test::an_answered_request_passes_
// without_time_moving` is test_case_run_test_an_answered_request_passes_without_
// time_moving), with the same scenario, verdict, `NOW` and log. Of the Rust
// module `the_shared_handle`, the C context holds two cases: a kill of a
// component whose machine runs is refused, and a context outlives no component
// it names, which in C ends the run with `error` where Rust panics. The third,
// a call into the context from a snapshot reader, is Rust's and Go's alone: their
// state is behind a lock a re-entrant call would find held, and the C run holds
// none, so a matcher's call into its context is an ordinary call. The case of
// `the_run_is_run_once` is the Go and Kotlin one, since a C run is not consumed
// by running it either. The case of `the_doors` reaches the run only through the
// two host processors a generated component registers on its engine (§12.3),
// written here with the C host's handler types. The cases of `the_bounds` hold
// what only the C runtime has: every table of a run is bounded at compile time,
// and a run that would pass a bound ends with `error`, naming it.
//
// Header-only code; nothing here allocates.

#include <sce/host_processor.h>
#include <sce/portability.h>
#include <sce/saved_values.h>
#include <sce/test/suite.h>
#include <sce/test/test_case_run.h>

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

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

#define SECOND ((sce_test_duration_t)1000000)

// ── Components written by hand ───────────────────────────────────────────

typedef struct scripted_s scripted_t;

// What a hand-written component does when it starts, or on a snapshot.
typedef void (*step_fn)(const sce_test_component_context_t *context, scripted_t *self);

// A component written by hand: what it does when it starts, and what it does on
// each snapshot. Either may end it, as entering a top-level final does. The
// machine is the first member, which the run holds a pointer to.
struct scripted_s {
    sce_test_component_machine_t machine;
    step_fn on_start;
    step_fn on_snapshot;
    sce_test_component_context_t context;
    bool in_use;
    bool ended;
};

static void idle(const sce_test_component_context_t *context, scripted_t *self) {
    (void)context;
    (void)self;
}

static void scripted_start(sce_test_component_machine_t *machine, const sce_test_component_context_t *context) {
    scripted_t *self = (scripted_t *)machine;
    self->context = *context;
    self->on_start(&self->context, self);
}

static void scripted_deliver_snapshot(sce_test_component_machine_t *machine) {
    scripted_t *self = (scripted_t *)machine;
    self->on_snapshot(&self->context, self);
}

static bool scripted_terminated(const sce_test_component_machine_t *machine) {
    return ((const scripted_t *)machine)->ended;
}

static void scripted_release(sce_test_component_machine_t *machine) {
    ((scripted_t *)machine)->in_use = false;
}

static const sce_test_component_machine_vtable_t SCRIPTED = {scripted_start, scripted_deliver_snapshot,
                                                             scripted_terminated, scripted_release};

// The two steps of the machines of one component type.
typedef struct steps_s {
    const char *component_type;
    step_fn on_start;
    step_fn on_snapshot;
} steps_t;

#define MACHINE_POOL 12u

// The machines of a test, by component type, from storage of their own.
typedef struct machines_s {
    steps_t types[4];
    size_t type_count;
    scripted_t pool[MACHINE_POOL];
} machines_t;

static machines_t *machines_with(machines_t *machines, const char *component_type, step_fn on_start,
                                 step_fn on_snapshot) {
    steps_t *steps = &machines->types[machines->type_count++];
    steps->component_type = component_type;
    steps->on_start = on_start;
    steps->on_snapshot = on_snapshot;
    return machines;
}

static sce_test_component_machine_t *machines_make(void *user_data, const char *component_type) {
    machines_t *machines = (machines_t *)user_data;
    for (size_t t = 0u; t < machines->type_count; ++t) {
        if (strcmp(machines->types[t].component_type, component_type) != 0) {
            continue;
        }
        for (size_t m = 0u; m < MACHINE_POOL; ++m) {
            scripted_t *made = &machines->pool[m];
            if (made->in_use) {
                continue;
            }
            memset(made, 0, sizeof(*made));
            made->machine.vtable = &SCRIPTED;
            made->on_start = machines->types[t].on_start;
            made->on_snapshot = machines->types[t].on_snapshot;
            made->in_use = true;
            return &made->machine;
        }
    }
    return NULL;
}

static sce_test_machine_factory_t factory_of(machines_t *machines) {
    sce_test_machine_factory_t factory;
    factory.make = machines_make;
    factory.user_data = machines;
    return factory;
}

// ── A system adapter that replays a trace ────────────────────────────────

// One input of a trace.
typedef struct input_s {
    sce_test_instant_t at;
    const char *system_port;
    const char *event;
} input_t;

// One send the adapter was handed.
typedef struct sent_s {
    sce_test_instant_t at;
    const char *system_port;
    char event[SCE_MAX_ID_LEN];
} sent_t;

// A system adapter that replays a trace and records what was sent to it.
typedef struct trace_adapter_s {
    sce_test_system_adapter_t adapter;
    const input_t *trace;
    size_t trace_count;
    size_t next;
    sent_t sent[4];
    size_t sent_count;
} trace_adapter_t;

static void trace_send(sce_test_system_adapter_t *adapter, const char *system_port, const sce_test_message_t *message,
                       sce_test_instant_t now) {
    trace_adapter_t *self = (trace_adapter_t *)adapter;
    if (self->sent_count < sizeof(self->sent) / sizeof(self->sent[0])) {
        sent_t *sent = &self->sent[self->sent_count++];
        sent->at = now;
        sent->system_port = system_port;
        (void)snprintf(sent->event, sizeof(sent->event), "%s", message->event);
    }
}

static bool trace_next_input(const sce_test_system_adapter_t *adapter, sce_test_instant_t *at) {
    const trace_adapter_t *self = (const trace_adapter_t *)adapter;
    if (self->next >= self->trace_count) {
        return false;
    }
    *at = self->trace[self->next].at;
    return true;
}

static bool trace_take_input_due(sce_test_system_adapter_t *adapter, sce_test_instant_t now, const char **system_port,
                                 sce_test_message_t *message) {
    trace_adapter_t *self = (trace_adapter_t *)adapter;
    if (self->next >= self->trace_count || self->trace[self->next].at > now) {
        return false;
    }
    const input_t *input = &self->trace[self->next++];
    *system_port = input->system_port;
    return sce_test_message_set(message, input->event, sce_host_value_none()) == SCE_TEST_MESSAGE_HELD;
}

static const sce_test_system_adapter_vtable_t TRACE = {trace_send, trace_next_input, trace_take_input_due, NULL};

static trace_adapter_t *replaying(trace_adapter_t *adapter, const input_t *trace, size_t trace_count) {
    memset(adapter, 0, sizeof(*adapter));
    adapter->adapter.vtable = &TRACE;
    adapter->trace = trace;
    adapter->trace_count = trace_count;
    return adapter;
}

// ── Configurations ────────────────────────────────────────────────────────

static const sce_test_ptc_t ECHO_PTC[] = {{"echo", "Echo"}};
static const sce_test_declared_t TESTER_AND_ECHO_PORTS[] = {{"Tester", "p"}, {"Echo", "q"}};
static const sce_test_declared_t TESTER_REPLY[] = {{"Tester", "reply"}};
static const sce_test_declared_t TESTER_REPLY_AND_ECHO_LATE[] = {{"Tester", "reply"}, {"Echo", "late"}};
static const sce_test_connection_t MTC_P_TO_ECHO_Q[] = {{{"mtc", "p"}, {"echo", "q"}}};
static const sce_test_mapping_t MTC_P_TO_SUT[] = {{{"mtc", "p"}, "sut"}};

// An MTC and one PTC joined by one connection, the MTC's `p` to the PTC's `q`.
static sce_test_configuration_t mtc_and_echo(void) {
    sce_test_configuration_t configuration;
    memset(&configuration, 0, sizeof(configuration));
    configuration.mtc_type = "Tester";
    configuration.ptcs = ECHO_PTC;
    configuration.ptc_count = 1u;
    configuration.ports = TESTER_AND_ECHO_PORTS;
    configuration.port_count = 2u;
    configuration.timers = TESTER_REPLY;
    configuration.timer_count = 1u;
    configuration.connections = MTC_P_TO_ECHO_Q;
    configuration.connection_count = 1u;
    return configuration;
}

// An MTC alone, of the type `component_type`, with no port and no timer.
static sce_test_configuration_t lone(const char *component_type) {
    sce_test_configuration_t configuration;
    memset(&configuration, 0, sizeof(configuration));
    configuration.mtc_type = component_type;
    return configuration;
}

// An MTC alone, of the type the port table names, whose port `p` is mapped to
// the system port `sut`.
static sce_test_configuration_t mapped(const sce_test_declared_t *port) {
    sce_test_configuration_t configuration = lone(port->component_type);
    configuration.ports = port;
    configuration.port_count = 1u;
    configuration.mappings = MTC_P_TO_SUT;
    configuration.mapping_count = 1u;
    return configuration;
}

static const sce_test_declared_t WAITER_P[] = {{"Waiter", "p"}};
static const sce_test_declared_t TESTER_P[] = {{"Tester", "p"}};
static const sce_test_declared_t READER_P[] = {{"Reader", "p"}};

// ── Reading a result ──────────────────────────────────────────────────────

static bool logged(const sce_test_case_result_t *result, const char *component, sce_test_log_kind_t kind) {
    for (size_t i = 0u; i < result->log_count; ++i) {
        if (strcmp(result->log[i].component, component) == 0 && result->log[i].kind == kind) {
            return true;
        }
    }
    return false;
}

// The index of the first line of `component` and `kind`, or SIZE_MAX.
static size_t position(const sce_test_case_result_t *result, const char *component, sce_test_log_kind_t kind) {
    for (size_t i = 0u; i < result->log_count; ++i) {
        if (strcmp(result->log[i].component, component) == 0 && result->log[i].kind == kind) {
            return i;
        }
    }
    return SIZE_MAX;
}

// Whether the log's last line is an error whose detail holds `part`.
static bool ends_in_error_naming(const sce_test_case_result_t *result, const char *part) {
    return result->log_count > 0u && result->log[result->log_count - 1u].kind == SCE_TEST_LOG_ERROR &&
           strstr(result->log[result->log_count - 1u].detail, part) != NULL;
}

// Whether some error of the log holds `part`, wherever it stands: a run that
// ends in error still removes the PTCs present, after the error's line.
static bool has_error_naming(const sce_test_case_result_t *result, const char *part) {
    for (size_t i = 0u; i < result->log_count; ++i) {
        if (result->log[i].kind == SCE_TEST_LOG_ERROR && strstr(result->log[i].detail, part) != NULL) {
            return true;
        }
    }
    return false;
}

// The run every case of a test case places, too large for a frame.
static sce_test_case_run_t run_storage;

static const sce_test_case_result_t *run_with(const sce_test_configuration_t *configuration, machines_t *machines,
                                              sce_test_system_adapter_t *adapter, const sce_test_duration_t *guard,
                                              const sce_test_module_parameters_t *parameters) {
    sce_test_case_run_init(&run_storage, configuration, factory_of(machines), adapter, guard, parameters);
    return sce_test_case_run_run(&run_storage);
}

static const sce_test_case_result_t *run_case(const sce_test_configuration_t *configuration, machines_t *machines,
                                              sce_test_system_adapter_t *adapter, const sce_test_duration_t *guard) {
    return run_with(configuration, machines, adapter, guard, NULL);
}

// ── The steps the cases share ─────────────────────────────────────────────

// The MTC creates the echo, sends to it, and waits for the answer within a
// second: a receive branch passes, a timeout branch fails.
static void tester_start(const sce_test_component_context_t *context, scripted_t *self) {
    (void)self;
    sce_test_context_create(context, "echo");
    sce_test_context_send(context, "p", "ping", sce_host_value_none());
    sce_test_context_start_timer(context, "reply", SECOND);
}

static void replies_in_time(const sce_test_component_context_t *context, scripted_t *self) {
    if (sce_test_context_snapshot_present(context, "p")) {
        sce_test_message_t message;
        CHECK(sce_test_context_receive(context, "p", &message), "the reply is received");
        CHECK(strcmp(message.event, "pong") == 0, "the reply is '%s'", message.event);
        sce_test_context_stop_timer(context, "reply");
        sce_test_context_setverdict(context, SCE_TEST_VERDICT_PASS, "");
        self->ended = true;
    } else if (sce_test_context_timed_out(context, "reply")) {
        sce_test_context_timeout(context, "reply");
        sce_test_context_setverdict(context, SCE_TEST_VERDICT_FAIL, "no reply in time");
        self->ended = true;
    }
}

static void echo_answers(const sce_test_component_context_t *context, scripted_t *self) {
    (void)self;
    if (sce_test_context_snapshot_present(context, "q")) {
        sce_test_message_t message;
        CHECK(sce_test_context_receive(context, "q", &message), "the request is received");
        CHECK(strcmp(message.event, "ping") == 0, "the request is '%s'", message.event);
        sce_test_context_send(context, "q", "pong", sce_host_value_none());
    }
}

static void sets_pass(const sce_test_component_context_t *context, scripted_t *self) {
    (void)self;
    sce_test_context_setverdict(context, SCE_TEST_VERDICT_PASS, "");
}

static void sets_fail(const sce_test_component_context_t *context, scripted_t *self) {
    (void)self;
    sce_test_context_setverdict(context, SCE_TEST_VERDICT_FAIL, "");
}

static void ends_pass(const sce_test_component_context_t *context, scripted_t *self) {
    sce_test_context_setverdict(context, SCE_TEST_VERDICT_PASS, "");
    self->ended = true;
}

static void ends_fail(const sce_test_component_context_t *context, scripted_t *self) {
    sce_test_context_setverdict(context, SCE_TEST_VERDICT_FAIL, "");
    self->ended = true;
}

static void ends_inconclusive(const sce_test_component_context_t *context, scripted_t *self) {
    sce_test_context_setverdict(context, SCE_TEST_VERDICT_INCONCLUSIVE, "");
    self->ended = true;
}

// What a component read of the module parameter `job`, if it read it.
static uint32_t job_read;
static bool job_was_read;

static bool read_job(void *user_data, const sce_test_module_parameters_t *given, sce_saved_error_t *err) {
    (void)user_data;
    sce_jv_t value;
    uint32_t job = 0u;
    if (!sce_test_module_parameters_of(given, "job", &value, err) || !sce_saved_read_uint32(value, "job", &job, err)) {
        return false;
    }
    job_read = job;
    job_was_read = true;
    return true;
}

// §11.5, §11.8: a component reads the module parameters of its run into its
// variables before it starts, as a generated machine reads each through the
// reader of the type it declares.
static void reads_job(const sce_test_component_context_t *context, scripted_t *self) {
    if (!sce_test_context_read_module_parameters(context, read_job, NULL)) {
        return;
    }
    sce_test_context_setverdict(context, SCE_TEST_VERDICT_PASS, "");
    self->ended = true;
}

static void forget_job(void) {
    job_read = 0u;
    job_was_read = false;
}

// ── test_verdict ──────────────────────────────────────────────────────────

static void test_verdict_only_ever_becomes_worse(void) {
    CHECK(sce_test_worse(SCE_TEST_VERDICT_PASS, SCE_TEST_VERDICT_NONE) == SCE_TEST_VERDICT_PASS, "pass over none");
    CHECK(sce_test_worse(SCE_TEST_VERDICT_PASS, SCE_TEST_VERDICT_INCONCLUSIVE) == SCE_TEST_VERDICT_INCONCLUSIVE,
          "inconclusive over pass");
    CHECK(sce_test_worse(SCE_TEST_VERDICT_FAIL, SCE_TEST_VERDICT_PASS) == SCE_TEST_VERDICT_FAIL, "fail over pass");
    CHECK(sce_test_worse(SCE_TEST_VERDICT_FAIL, SCE_TEST_VERDICT_ERROR) == SCE_TEST_VERDICT_ERROR, "error over fail");
    CHECK(strcmp(sce_test_verdict_name(SCE_TEST_VERDICT_INCONCLUSIVE), "inconc") == 0, "TTCN-3 spells inconc");
}

// ── test_port ─────────────────────────────────────────────────────────────

static sce_test_message_t message_of(const char *event) {
    sce_test_message_t message;
    (void)sce_test_message_set(&message, event, sce_host_value_none());
    return message;
}

static void test_port_a_snapshot_sees_only_what_was_queued_when_it_was_taken(void) {
    static sce_test_message_pool_t pool;
    sce_test_message_pool_init(&pool);
    sce_test_port_t port;
    sce_test_port_init(&port, "p");
    const sce_test_message_t late = message_of("late");
    const sce_test_message_t behind = message_of("behind");
    sce_test_port_take_snapshot(&port);
    CHECK(sce_test_port_enqueue(&port, &pool, &late), "the pool has room");
    CHECK(sce_test_port_snapshot(&port, &pool) == NULL, "a message that arrives after the snapshot is not in it");
    sce_test_port_take_snapshot(&port);
    CHECK(sce_test_port_snapshot(&port, &pool) != NULL, "the next snapshot sees it");
    CHECK(sce_test_port_enqueue(&port, &pool, &behind), "the pool has room");
    CHECK(strcmp(sce_test_port_snapshot(&port, &pool)->event, "late") == 0, "a later message goes behind the head");
    sce_test_message_t head;
    CHECK(sce_test_port_remove_head(&port, &pool, &head) && strcmp(head.event, "late") == 0, "the head leaves first");
    CHECK(sce_test_port_size(&port) == 1u, "one message is left, not %zu", sce_test_port_size(&port));
}

// ── test_timer ────────────────────────────────────────────────────────────

static void test_timer_times_out_at_its_instant_and_restarts_when_started_again(void) {
    sce_test_timer_t timer;
    sce_test_timer_init(&timer, "t");
    sce_test_timer_start(&timer, 0, 10);
    sce_test_timer_update(&timer, 9);
    CHECK(timer.status == SCE_TEST_TIMER_RUNNING, "running before its instant");
    sce_test_timer_start(&timer, 5, 10);
    sce_test_instant_t expiry = 0;
    CHECK(sce_test_timer_expiry(&timer, &expiry) && expiry == 15, "a running timer that is started again restarts");
    sce_test_timer_update(&timer, 15);
    CHECK(timer.status == SCE_TEST_TIMER_TIMEOUT, "timed out at its instant");
    sce_test_timer_take_snapshot(&timer);
    CHECK(sce_test_timer_snapshot_timed_out(&timer), "the snapshot sees the timeout");
    sce_test_timer_timeout_taken(&timer);
    CHECK(timer.status == SCE_TEST_TIMER_IDLE, "a taken timeout leaves it idle");
    sce_test_timer_stop(&timer);
    CHECK(timer.status == SCE_TEST_TIMER_IDLE, "stopping a timer that is not running does nothing");
}

// ── test_case_run_test ────────────────────────────────────────────────────

// P6, P9, §3.2: a message sent in a round is taken in a later round, and the
// whole exchange happens at one instant, because a branch fired in each round.
static void test_case_run_test_an_answered_request_passes_without_time_moving(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(machines_with(&machines, "Tester", tester_start, replies_in_time), "Echo", idle, echo_answers);
    trace_adapter_t adapter;
    const sce_test_configuration_t configuration = mtc_and_echo();
    const sce_test_case_result_t *result =
        run_case(&configuration, &machines, &replaying(&adapter, NULL, 0u)->adapter, NULL);
    CHECK(result->verdict == SCE_TEST_VERDICT_PASS, "verdict %d", (int)result->verdict);
    CHECK(result->ended_at == 0, "ended at %lld", (long long)result->ended_at);
    CHECK(logged(result, "echo", SCE_TEST_LOG_KILL), "the MTC's end removes the PTC (P10)");
}

// P7, §3.3: with nothing to receive, time advances to the timer's instant.
static void test_case_run_test_an_unanswered_request_fails_when_its_timer_expires(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(machines_with(&machines, "Tester", tester_start, replies_in_time), "Echo", idle, idle);
    trace_adapter_t adapter;
    const sce_test_configuration_t configuration = mtc_and_echo();
    const sce_test_case_result_t *result =
        run_case(&configuration, &machines, &replaying(&adapter, NULL, 0u)->adapter, NULL);
    CHECK(result->verdict == SCE_TEST_VERDICT_FAIL, "verdict %d", (int)result->verdict);
    CHECK(result->ended_at == SECOND, "ended at %lld", (long long)result->ended_at);
}

// P13: no branch can fire and no instant is pending, so the test case can never
// end; the runtime ends it with error.
static void test_case_run_test_a_test_case_that_can_never_end_is_an_error(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(&machines, "Waiter", idle, idle);
    trace_adapter_t adapter;
    const sce_test_configuration_t configuration = mapped(WAITER_P);
    const sce_test_case_result_t *result =
        run_case(&configuration, &machines, &replaying(&adapter, NULL, 0u)->adapter, NULL);
    CHECK(result->verdict == SCE_TEST_VERDICT_ERROR, "verdict %d", (int)result->verdict);
    CHECK(logged(result, "mtc", SCE_TEST_LOG_ERROR), "the error is the MTC's");
}

// P11: the guard instant ends a test case that has not ended, with error.
static void test_case_run_test_the_guard_ends_a_test_case_with_error(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(&machines, "Waiter", sets_pass, idle);
    trace_adapter_t adapter;
    const sce_test_configuration_t configuration = mapped(WAITER_P);
    const sce_test_duration_t guard = 5 * SECOND;
    const sce_test_case_result_t *result =
        run_case(&configuration, &machines, &replaying(&adapter, NULL, 0u)->adapter, &guard);
    CHECK(result->verdict == SCE_TEST_VERDICT_ERROR, "verdict %d", (int)result->verdict);
    CHECK(result->ended_at == 5 * SECOND, "ended at %lld", (long long)result->ended_at);
}

static void sends_and_waits(const sce_test_component_context_t *context, scripted_t *self) {
    (void)self;
    sce_test_context_send(context, "p", "ping", sce_host_value_none());
    sce_test_context_start_timer(context, "reply", SECOND);
}

// §2.2, §12.2: a mapped port's sends reach the adapter, and the trace's inputs
// reach the port at their instants.
static void test_case_run_test_a_mapped_port_talks_to_the_system_adapter_on_the_logical_clock(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(&machines, "Tester", sends_and_waits, replies_in_time);
    static const input_t trace[] = {{500, "sut", "pong"}};
    trace_adapter_t adapter;
    sce_test_configuration_t configuration = mapped(TESTER_P);
    configuration.timers = TESTER_REPLY;
    configuration.timer_count = 1u;
    const sce_test_case_result_t *result =
        run_case(&configuration, &machines, &replaying(&adapter, trace, 1u)->adapter, NULL);
    CHECK(result->verdict == SCE_TEST_VERDICT_PASS, "verdict %d", (int)result->verdict);
    CHECK(result->ended_at == 500, "ended at %lld", (long long)result->ended_at);
    CHECK(adapter.sent_count == 1u, "%zu sends reached the adapter", adapter.sent_count);
    CHECK(strcmp(adapter.sent[0].system_port, "sut") == 0, "the send went to '%s'", adapter.sent[0].system_port);
    CHECK(strcmp(adapter.sent[0].event, "ping") == 0, "the send was '%s'", adapter.sent[0].event);
}

static void creates_echo_and_waits_ten(const sce_test_component_context_t *context, scripted_t *self) {
    (void)self;
    sce_test_context_create(context, "echo");
    sce_test_context_start_timer(context, "reply", 10);
}

static void kills_echo_at_the_timeout(const sce_test_component_context_t *context, scripted_t *self) {
    if (sce_test_context_timed_out(context, "reply")) {
        sce_test_context_timeout(context, "reply");
        sce_test_context_kill(context, "echo");
        sce_test_context_setverdict(context, SCE_TEST_VERDICT_PASS, "");
        self->ended = true;
    }
}

// P9, P14: a removed PTC's verdict still counts in the test case verdict.
static void test_case_run_test_a_removed_ptcs_verdict_still_counts(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(machines_with(&machines, "Tester", creates_echo_and_waits_ten, kills_echo_at_the_timeout), "Echo",
                  sets_fail, idle);
    trace_adapter_t adapter;
    const sce_test_configuration_t configuration = mtc_and_echo();
    const sce_test_case_result_t *result =
        run_case(&configuration, &machines, &replaying(&adapter, NULL, 0u)->adapter, NULL);
    CHECK(result->verdict == SCE_TEST_VERDICT_FAIL, "verdict %d", (int)result->verdict);
}

static void passes_at_the_timeout(const sce_test_component_context_t *context, scripted_t *self) {
    if (sce_test_context_timed_out(context, "reply")) {
        sce_test_context_timeout(context, "reply");
        sce_test_context_setverdict(context, SCE_TEST_VERDICT_PASS, "");
        self->ended = true;
    }
}

static void starts_late(const sce_test_component_context_t *context, scripted_t *self) {
    (void)self;
    sce_test_context_start_timer(context, "late", 10);
}

static void fails_when_late(const sce_test_component_context_t *context, scripted_t *self) {
    (void)self;
    if (sce_test_context_timed_out(context, "late")) {
        sce_test_context_timeout(context, "late");
        sce_test_context_setverdict(context, SCE_TEST_VERDICT_FAIL, "");
    }
}

// §3.2 step 4: components select one at a time in creation order, so when the
// MTC terminates in a round, a PTC that would have selected after it in that
// round does not.
static void test_case_run_test_nothing_selects_after_the_mtc_terminates_in_the_round(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(machines_with(&machines, "Tester", creates_echo_and_waits_ten, passes_at_the_timeout), "Echo",
                  starts_late, fails_when_late);
    trace_adapter_t adapter;
    sce_test_configuration_t configuration = mtc_and_echo();
    configuration.timers = TESTER_REPLY_AND_ECHO_LATE;
    configuration.timer_count = 2u;
    const sce_test_case_result_t *result =
        run_case(&configuration, &machines, &replaying(&adapter, NULL, 0u)->adapter, NULL);
    CHECK(result->verdict == SCE_TEST_VERDICT_PASS, "verdict %d", (int)result->verdict);
    CHECK(!logged(result, "echo", SCE_TEST_LOG_TIMEOUT), "the echo did not select after the MTC's end");
}

static void creates_echo(const sce_test_component_context_t *context, scripted_t *self) {
    (void)self;
    sce_test_context_create(context, "echo");
}

static void ends_when_echo_is_done(const sce_test_component_context_t *context, scripted_t *self) {
    if (sce_test_context_done(context, "echo")) {
        sce_test_context_done_taken(context, "echo");
        sce_test_context_kill(context, "echo");
        self->ended = true;
    }
}

// P2, P10: a PTC that terminates is in DONE from the next snapshot on, and a
// done branch removes nothing.
static void test_case_run_test_a_done_branch_sees_a_ptc_that_terminated(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(machines_with(&machines, "Tester", creates_echo, ends_when_echo_is_done), "Echo", ends_pass, idle);
    trace_adapter_t adapter;
    const sce_test_configuration_t configuration = mtc_and_echo();
    const sce_test_case_result_t *result =
        run_case(&configuration, &machines, &replaying(&adapter, NULL, 0u)->adapter, NULL);
    CHECK(result->verdict == SCE_TEST_VERDICT_PASS, "verdict %d", (int)result->verdict);
    CHECK(logged(result, "echo", SCE_TEST_LOG_DONE), "the echo's end is logged");
}

// The SUT's four messages, all at `NOW = 0`: a record of another type, another
// event, another value, and the one the template matches.
typedef struct four_inputs_s {
    sce_test_system_adapter_t adapter;
    size_t given;
} four_inputs_t;

static void four_send(sce_test_system_adapter_t *adapter, const char *system_port, const sce_test_message_t *message,
                      sce_test_instant_t now) {
    (void)adapter;
    (void)system_port;
    (void)message;
    (void)now;
}

static bool four_next_input(const sce_test_system_adapter_t *adapter, sce_test_instant_t *at) {
    (void)adapter;
    (void)at;
    return false;
}

static bool four_take_input_due(sce_test_system_adapter_t *adapter, sce_test_instant_t now, const char **system_port,
                                sce_test_message_t *message) {
    static const int64_t seven_of_another_type = 7;
    static const int32_t seven = 7;
    static const int32_t six = 6;
    four_inputs_t *self = (four_inputs_t *)adapter;
    if (now != 0 || self->given >= 4u) {
        return false;
    }
    *system_port = "sut";
    switch (self->given++) {
    case 0u:
        return sce_test_message_set(message, "number", SCE_HOST_VALUE(int64_t, &seven_of_another_type)) ==
               SCE_TEST_MESSAGE_HELD;
    case 1u:
        return sce_test_message_set(message, "other", SCE_HOST_VALUE(int32_t, &seven)) == SCE_TEST_MESSAGE_HELD;
    case 2u:
        return sce_test_message_set(message, "number", SCE_HOST_VALUE(int32_t, &six)) == SCE_TEST_MESSAGE_HELD;
    default:
        return sce_test_message_set(message, "number", SCE_HOST_VALUE(int32_t, &seven)) == SCE_TEST_MESSAGE_HELD;
    }
}

static const sce_test_system_adapter_vtable_t FOUR_INPUTS = {four_send, four_next_input, four_take_input_due, NULL};

// What the reader saw, in order.
static char seen[8][SCE_MAX_ID_LEN];
static size_t seen_count;

static void saw(const char *what) {
    if (seen_count < sizeof(seen) / sizeof(seen[0])) {
        (void)snprintf(seen[seen_count++], sizeof(seen[0]), "%s", what);
    }
}

static bool is_seven(const void *value, const void *closure) {
    (void)closure;
    return *(const int32_t *)value == 7;
}

static void reads_until_seven(const sce_test_component_context_t *context, scripted_t *self) {
    if (SCE_TEST_RECEIVES(context, "p", "number", int32_t, is_seven, NULL)) {
        saw("seven");
        CHECK(SCE_TEST_RECEIVES(context, "p", "number", int32_t, is_seven, NULL),
              "a guard that matched removed the message");
        (void)sce_test_context_receive(context, "p", NULL);
        self->ended = true;
    } else if (sce_test_context_snapshot_present(context, "p")) {
        saw(sce_test_context_snapshot_of(context, "p")->event);
        (void)sce_test_context_receive(context, "p", NULL);
    }
}

// P2, §12.3: a receive branch with a template matches only a message of its
// record, under its record's event, whose value its `match` holds; the guard
// reads the snapshot and removes nothing.
static void test_case_run_test_a_receive_with_a_template_matches_only_its_records_value(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(&machines, "Reader", idle, reads_until_seven);
    four_inputs_t adapter;
    memset(&adapter, 0, sizeof(adapter));
    adapter.adapter.vtable = &FOUR_INPUTS;
    seen_count = 0u;
    const sce_test_configuration_t configuration = mapped(READER_P);
    const sce_test_case_result_t *result = run_case(&configuration, &machines, &adapter.adapter, NULL);
    CHECK(seen_count == 4u, "the reader saw %zu messages", seen_count);
    const char *const want[] = {"number", "other", "number", "seven"};
    for (size_t i = 0u; i < 4u && i < seen_count; ++i) {
        CHECK(strcmp(seen[i], want[i]) == 0,
              "a value of another type, another event and another value do not match: saw '%s' for '%s'", seen[i],
              want[i]);
    }
    CHECK(result->verdict == SCE_TEST_VERDICT_NONE, "verdict %d", (int)result->verdict);
}

// §11.8: the value the run is given is the one the component reads, and the
// component starts on it.
static void test_case_run_test_a_component_reads_the_module_parameter_its_run_is_given(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(&machines, "Reader", reads_job, idle);
    sce_test_module_parameters_t parameters;
    sce_test_module_parameters_init(&parameters);
    CHECK(sce_test_module_parameters_set(&parameters, "job", "7"), "the table has room");
    forget_job();
    trace_adapter_t adapter;
    const sce_test_configuration_t configuration = lone("Reader");
    const sce_test_case_result_t *result =
        run_with(&configuration, &machines, &replaying(&adapter, NULL, 0u)->adapter, NULL, &parameters);
    CHECK(job_was_read && job_read == 7u, "the component read %u", (unsigned)job_read);
    CHECK(result->verdict == SCE_TEST_VERDICT_PASS, "verdict %d", (int)result->verdict);
}

// P13: a module parameter the run has no value for is a dynamic error, named in
// the log, and the component goes no further than the read.
static void test_case_run_test_a_module_parameter_the_run_is_not_given_is_an_error(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(&machines, "Reader", reads_job, idle);
    forget_job();
    trace_adapter_t adapter;
    const sce_test_configuration_t configuration = lone("Reader");
    const sce_test_case_result_t *result =
        run_case(&configuration, &machines, &replaying(&adapter, NULL, 0u)->adapter, NULL);
    CHECK(result->verdict == SCE_TEST_VERDICT_ERROR, "verdict %d", (int)result->verdict);
    CHECK(!job_was_read, "the component read no value");
    CHECK(ends_in_error_naming(result, "'job'"), "the run logged its error, naming 'job'");
}

// P13: a value that is not of the type the component declares is refused as a
// restored value of another type is, and not converted.
static void test_case_run_test_a_module_parameter_of_another_type_is_an_error(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(&machines, "Reader", reads_job, idle);
    sce_test_module_parameters_t parameters;
    sce_test_module_parameters_init(&parameters);
    CHECK(sce_test_module_parameters_set(&parameters, "job", "\"seven\""), "the table has room");
    forget_job();
    trace_adapter_t adapter;
    const sce_test_configuration_t configuration = lone("Reader");
    const sce_test_case_result_t *result =
        run_with(&configuration, &machines, &replaying(&adapter, NULL, 0u)->adapter, NULL, &parameters);
    CHECK(result->verdict == SCE_TEST_VERDICT_ERROR, "verdict %d", (int)result->verdict);
    CHECK(!job_was_read, "the component read no value");
    CHECK(logged(result, "mtc", SCE_TEST_LOG_ERROR), "the error is the MTC's");
}

// P13: a component type the factory of machines makes no machine for ends the
// run with error where a component of it is created.
static void test_case_run_test_a_component_type_with_no_machine_is_an_error(void) {
    static machines_t none;
    memset(&none, 0, sizeof(none));
    trace_adapter_t adapter;
    const sce_test_configuration_t configuration = lone("Ghost");
    const sce_test_case_result_t *result =
        run_case(&configuration, &none, &replaying(&adapter, NULL, 0u)->adapter, NULL);
    CHECK(result->verdict == SCE_TEST_VERDICT_ERROR, "verdict %d", (int)result->verdict);
    CHECK(logged(result, "mtc", SCE_TEST_LOG_ERROR), "the error is the MTC's");
    CHECK(!logged(result, "mtc", SCE_TEST_LOG_START), "no component of the type was created");
}

static void sends_ping(const sce_test_component_context_t *context, scripted_t *self) {
    (void)self;
    sce_test_context_send(context, "p", "ping", sce_host_value_none());
}

// P6, P13: a send on a port whose peer is not present is a dynamic error.
static void test_case_run_test_a_send_to_an_absent_peer_is_an_error(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(&machines, "Tester", sends_ping, idle);
    trace_adapter_t adapter;
    const sce_test_configuration_t configuration = mtc_and_echo();
    const sce_test_case_result_t *result =
        run_case(&configuration, &machines, &replaying(&adapter, NULL, 0u)->adapter, NULL);
    CHECK(result->verdict == SCE_TEST_VERDICT_ERROR, "verdict %d", (int)result->verdict);
    CHECK(result->ended_at == 0, "ended at %lld", (long long)result->ended_at);
}

// ── test_suite_run ────────────────────────────────────────────────────────

// The adapters of a suite's run, each a fresh trace adapter, recording the test
// case each was built for; none at all where `give_none`.
typedef struct adapters_s {
    trace_adapter_t pool[8];
    size_t used;
    const char *built[8];
    size_t built_count;
    bool give_none;
} adapters_t;

static sce_test_system_adapter_t *adapters_make(void *user_data, const char *test_case) {
    adapters_t *adapters = (adapters_t *)user_data;
    if (adapters->give_none || adapters->used >= sizeof(adapters->pool) / sizeof(adapters->pool[0])) {
        return NULL;
    }
    adapters->built[adapters->built_count++] = test_case;
    return &replaying(&adapters->pool[adapters->used++], NULL, 0u)->adapter;
}

static sce_test_adapter_factory_t tracing(adapters_t *adapters) {
    memset(adapters, 0, sizeof(*adapters));
    sce_test_adapter_factory_t factory;
    factory.make = adapters_make;
    factory.user_data = adapters;
    return factory;
}

// What the runner reported, case by case.
typedef struct reported_s {
    char test_case[8][32];
    sce_test_verdict_t verdict[8];
    sce_test_instant_t ended_at[8];
    size_t log_count[8];
    char first_detail[8][SCE_TEST_MAX_LOG_DETAIL];
    size_t count;
} reported_t;

static void report(void *user_data, const char *test_case, const sce_test_case_result_t *result) {
    reported_t *reported = (reported_t *)user_data;
    if (reported->count >= 8u) {
        return;
    }
    const size_t at = reported->count++;
    (void)snprintf(reported->test_case[at], sizeof(reported->test_case[at]), "%s", test_case);
    reported->verdict[at] = result->verdict;
    reported->ended_at[at] = result->ended_at;
    reported->log_count[at] = result->log_count;
    (void)snprintf(reported->first_detail[at], sizeof(reported->first_detail[at]), "%s",
                   result->log_count > 0u ? result->log[0].detail : "");
}

static bool selects_x(const sce_test_features_t *world) {
    return sce_test_enabled(world, "x");
}

static bool selects_y(const sce_test_features_t *world) {
    return sce_test_enabled(world, "y");
}

// An execute of the test case `name` over `configuration` and `machines`, with
// no guard, no selection and no module parameter.
static sce_test_execute_t execute_of(const char *name, const sce_test_configuration_t *configuration,
                                     machines_t *machines) {
    sce_test_execute_t made;
    memset(&made, 0, sizeof(made));
    made.test_case.name = name;
    made.test_case.configuration = configuration;
    made.test_case.machines = factory_of(machines);
    return made;
}

static const sce_test_features_t NO_FEATURES = {NULL, 0u};

// P11: the runner runs the test cases a world selects, in the suite's order, and
// skips the others; a test case with no selection runs in every world.
static void test_suite_run_runs_the_test_cases_a_world_selects_in_the_suites_order(void) {
    static machines_t first;
    static machines_t second;
    static machines_t third;
    memset(&first, 0, sizeof(first));
    memset(&second, 0, sizeof(second));
    memset(&third, 0, sizeof(third));
    machines_with(&first, "First", ends_pass, idle);
    machines_with(&second, "Second", ends_fail, idle);
    machines_with(&third, "Third", ends_inconclusive, idle);
    const sce_test_configuration_t first_configuration = lone("First");
    const sce_test_configuration_t second_configuration = lone("Second");
    const sce_test_configuration_t third_configuration = lone("Third");
    sce_test_execute_t executes[3];
    executes[0] = execute_of("first", &first_configuration, &first);
    executes[1] = execute_of("second", &second_configuration, &second);
    executes[1].selection = selects_x;
    executes[2] = execute_of("third", &third_configuration, &third);
    executes[2].selection = selects_y;
    sce_test_suite_t suite;
    memset(&suite, 0, sizeof(suite));
    suite.name = "ordered";
    suite.executes = executes;
    suite.execute_count = 3u;
    static const char *const y[] = {"y"};
    const sce_test_features_t world = {y, 1u};
    adapters_t adapters;
    reported_t reported;
    memset(&reported, 0, sizeof(reported));
    CHECK(sce_test_run_suite(&suite, &world, tracing(&adapters), NULL, &run_storage, report, &reported, NULL),
          "no override is given");
    CHECK(reported.count == 2u, "%zu cases ran", reported.count);
    CHECK(strcmp(reported.test_case[0], "first") == 0 && reported.verdict[0] == SCE_TEST_VERDICT_PASS,
          "the first ran first and passed");
    CHECK(strcmp(reported.test_case[1], "third") == 0 && reported.verdict[1] == SCE_TEST_VERDICT_INCONCLUSIVE,
          "the third ran second, inconclusive");
    CHECK(adapters.built_count == 2u && strcmp(adapters.built[0], "first") == 0 &&
              strcmp(adapters.built[1], "third") == 0,
          "an adapter is built for each case run");
}

// P11: each test case is a fresh run from NOW = 0 under its own guard.
static void test_suite_run_each_test_case_is_a_fresh_run_under_its_own_guard(void) {
    static machines_t waiting;
    static machines_t quick;
    memset(&waiting, 0, sizeof(waiting));
    memset(&quick, 0, sizeof(quick));
    machines_with(&waiting, "Waiter", idle, idle);
    machines_with(&quick, "Quick", ends_pass, idle);
    const sce_test_configuration_t stuck_configuration = lone("Waiter");
    const sce_test_configuration_t quick_configuration = lone("Quick");
    sce_test_execute_t executes[2];
    executes[0] = execute_of("stuck", &stuck_configuration, &waiting);
    executes[0].guarded = true;
    executes[0].guard = 5 * SECOND;
    executes[1] = execute_of("quick", &quick_configuration, &quick);
    sce_test_suite_t suite;
    memset(&suite, 0, sizeof(suite));
    suite.name = "guarded";
    suite.executes = executes;
    suite.execute_count = 2u;
    adapters_t adapters;
    reported_t reported;
    memset(&reported, 0, sizeof(reported));
    CHECK(sce_test_run_suite(&suite, &NO_FEATURES, tracing(&adapters), NULL, &run_storage, report, &reported, NULL),
          "no override is given");
    CHECK(reported.count == 2u, "%zu cases ran", reported.count);
    CHECK(reported.verdict[0] == SCE_TEST_VERDICT_ERROR, "the guard ends the first with error");
    CHECK(reported.ended_at[0] == 5 * SECOND, "the guard ends the first at %lld", (long long)reported.ended_at[0]);
    CHECK(reported.verdict[1] == SCE_TEST_VERDICT_PASS, "the second passes");
    CHECK(reported.ended_at[1] == 0, "the second starts again at NOW = 0, not %lld", (long long)reported.ended_at[1]);
}

// -1 §8.2.1: a value the caller gives a module parameter replaces the suite's
// default, and a parameter the suite does not declare cannot be given one.
static void test_suite_run_an_override_replaces_the_default_of_a_module_parameter_the_suite_declares(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(&machines, "Reader", reads_job, idle);
    const sce_test_configuration_t configuration = lone("Reader");
    static const char *const job[] = {"job"};
    sce_test_execute_t executes[1];
    executes[0] = execute_of("reads", &configuration, &machines);
    executes[0].test_case.module_parameters = job;
    executes[0].test_case.module_parameter_count = 1u;
    sce_test_suite_t suite;
    memset(&suite, 0, sizeof(suite));
    suite.name = "parameters";
    sce_test_module_parameters_init(&suite.module_parameters);
    CHECK(sce_test_module_parameters_set(&suite.module_parameters, "job", "7"), "the table has room");
    suite.executes = executes;
    suite.execute_count = 1u;
    adapters_t adapters;
    reported_t reported;
    memset(&reported, 0, sizeof(reported));
    forget_job();
    CHECK(sce_test_run_suite(&suite, &NO_FEATURES, tracing(&adapters), NULL, &run_storage, report, &reported, NULL),
          "no override is given");
    CHECK(reported.count == 1u && reported.verdict[0] == SCE_TEST_VERDICT_PASS, "the case passes");
    CHECK(job_was_read && job_read == 7u, "the default: %u", (unsigned)job_read);

    sce_test_module_parameters_t overrides;
    sce_test_module_parameters_init(&overrides);
    CHECK(sce_test_module_parameters_set(&overrides, "job", "9"), "the table has room");
    memset(&reported, 0, sizeof(reported));
    forget_job();
    CHECK(
        sce_test_run_suite(&suite, &NO_FEATURES, tracing(&adapters), &overrides, &run_storage, report, &reported, NULL),
        "the suite declares the override's parameter");
    CHECK(reported.count == 1u, "%zu cases ran", reported.count);
    CHECK(job_was_read && job_read == 9u, "the override: %u", (unsigned)job_read);

    sce_test_module_parameters_t stray;
    sce_test_module_parameters_init(&stray);
    CHECK(sce_test_module_parameters_set(&stray, "limit", "1"), "the table has room");
    memset(&reported, 0, sizeof(reported));
    sce_test_override_refusal_t refusal;
    memset(&refusal, 0, sizeof(refusal));
    CHECK(!sce_test_run_suite(&suite, &NO_FEATURES, tracing(&adapters), &stray, &run_storage, report, &reported,
                              &refusal),
          "an override of an undeclared parameter is refused");
    CHECK(strstr(refusal.reason, "'limit'") != NULL, "%s", refusal.reason);
    CHECK(reported.count == 0u, "no case ran");
}

// P13: a test case whose components read a module parameter the run has no
// value for ends with error before anything of it runs, its adapter included.
static void test_suite_run_a_test_case_that_reads_a_module_parameter_with_no_value_is_an_error(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(&machines, "Reader", ends_pass, idle);
    const sce_test_configuration_t configuration = lone("Reader");
    static const char *const job[] = {"job"};
    sce_test_execute_t executes[1];
    executes[0] = execute_of("reads", &configuration, &machines);
    executes[0].test_case.module_parameters = job;
    executes[0].test_case.module_parameter_count = 1u;
    sce_test_suite_t suite;
    memset(&suite, 0, sizeof(suite));
    suite.name = "undeclared";
    suite.executes = executes;
    suite.execute_count = 1u;
    adapters_t adapters;
    reported_t reported;
    memset(&reported, 0, sizeof(reported));
    CHECK(sce_test_run_suite(&suite, &NO_FEATURES, tracing(&adapters), NULL, &run_storage, report, &reported, NULL),
          "no override is given");
    CHECK(reported.count == 1u && reported.verdict[0] == SCE_TEST_VERDICT_ERROR, "the case ends with error");
    CHECK(reported.log_count[0] == 1u, "its log holds %zu lines", reported.log_count[0]);
    CHECK(strstr(reported.first_detail[0], "'job'") != NULL, "%s", reported.first_detail[0]);
    CHECK(adapters.built_count == 0u, "no adapter is built for a case that cannot start");
}

// P13: a test case the caller gives no system adapter ends with error.
static void test_suite_run_a_test_case_with_no_system_adapter_is_an_error(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(&machines, "Only", ends_pass, idle);
    const sce_test_configuration_t configuration = lone("Only");
    sce_test_execute_t executes[1];
    executes[0] = execute_of("only", &configuration, &machines);
    sce_test_suite_t suite;
    memset(&suite, 0, sizeof(suite));
    suite.name = "unadapted";
    suite.executes = executes;
    suite.execute_count = 1u;
    adapters_t adapters;
    const sce_test_adapter_factory_t none = tracing(&adapters);
    adapters.give_none = true;
    reported_t reported;
    memset(&reported, 0, sizeof(reported));
    CHECK(sce_test_run_suite(&suite, &NO_FEATURES, none, NULL, &run_storage, report, &reported, NULL),
          "no override is given");
    CHECK(reported.count == 1u && reported.verdict[0] == SCE_TEST_VERDICT_ERROR, "the case ends with error");
}

// ── the_shared_handle ─────────────────────────────────────────────────────

static void creates_echo_and_passes(const sce_test_component_context_t *context, scripted_t *self) {
    (void)self;
    sce_test_context_create(context, "echo");
    sce_test_context_setverdict(context, SCE_TEST_VERDICT_PASS, "");
}

static void kills_the_mtc(const sce_test_component_context_t *context, scripted_t *self) {
    (void)self;
    sce_test_context_kill(context, "mtc");
}

// P9, P13: a component whose machine is running is not removed under it. The
// PTC's start kills the MTC, whose own start is still running; that is a
// dynamic error, and the MTC stays.
static void the_shared_handle_a_kill_of_a_component_whose_machine_runs_is_an_error(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(machines_with(&machines, "Tester", creates_echo_and_passes, idle), "Echo", kills_the_mtc, idle);
    trace_adapter_t adapter;
    const sce_test_configuration_t configuration = mtc_and_echo();
    const sce_test_case_result_t *result =
        run_case(&configuration, &machines, &replaying(&adapter, NULL, 0u)->adapter, NULL);
    CHECK(result->verdict == SCE_TEST_VERDICT_ERROR, "verdict %d", (int)result->verdict);
    CHECK(logged(result, "echo", SCE_TEST_LOG_ERROR), "the error is the echo's");
    CHECK(!logged(result, "mtc", SCE_TEST_LOG_KILL), "the MTC was not removed");
    CHECK(logged(result, "mtc", SCE_TEST_LOG_VERDICT), "the MTC's start ran on after the refused kill");
}

// The context the echo kept.
static sce_test_component_context_t kept;

static void keeps_its_context(const sce_test_component_context_t *context, scripted_t *self) {
    (void)self;
    kept = *context;
}

static void kills_echo_then_calls_its_context(const sce_test_component_context_t *context, scripted_t *self) {
    (void)self;
    sce_test_context_create(context, "echo");
    sce_test_context_kill(context, "echo");
    sce_test_context_setverdict(&kept, SCE_TEST_VERDICT_FAIL, "after its removal");
}

// A context names one creation of its component: once the PTC is removed, a
// call through a context it handed out reaches no other component, and ends
// the run with error where the Rust handle panics.
static void the_shared_handle_a_context_does_not_outlive_its_component(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(machines_with(&machines, "Tester", kills_echo_then_calls_its_context, idle), "Echo",
                  keeps_its_context, idle);
    memset(&kept, 0, sizeof(kept));
    trace_adapter_t adapter;
    const sce_test_configuration_t configuration = mtc_and_echo();
    const sce_test_case_result_t *result =
        run_case(&configuration, &machines, &replaying(&adapter, NULL, 0u)->adapter, NULL);
    CHECK(result->verdict == SCE_TEST_VERDICT_ERROR, "verdict %d", (int)result->verdict);
    CHECK(ends_in_error_naming(result, "the component echo was removed, and its context outlived it"),
          "the stale call is refused by name");
    CHECK(!logged(result, "echo", SCE_TEST_LOG_VERDICT), "the call set no verdict");
}

// ── the_run_is_run_once ───────────────────────────────────────────────────

// A run has read its adapter and holds its components' end, so running it a
// second time is refused rather than run on that state.
static void the_run_is_run_once_a_run_that_has_run_is_not_run_again(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(&machines, "Tester", ends_pass, idle);
    trace_adapter_t adapter;
    const sce_test_configuration_t configuration = lone("Tester");
    const sce_test_case_result_t *result =
        run_case(&configuration, &machines, &replaying(&adapter, NULL, 0u)->adapter, NULL);
    CHECK(result != NULL && result->verdict == SCE_TEST_VERDICT_PASS, "the first run passes");
    CHECK(sce_test_case_run_run(&run_storage) == NULL, "a run is run once");
}

// ── the_doors ─────────────────────────────────────────────────────────────

// The record a template of the port type reads.
typedef struct ping_s {
    uint32_t n;
} ping_t;

// A send (P6): the request carries the template's value, which the run appends
// to the connected port, or hands the system adapter.
static void test_port_processor(void *user_data, const sce_host_send_request_t *request,
                                sce_host_send_response_list_t *out) {
    (void)out;
    const sce_test_component_context_t *context = (const sce_test_component_context_t *)user_data;
    CHECK(request->value.type != NULL, "a test-port send carries its template's value");
    sce_test_context_send(context, request->target, request->event_name, request->value);
}

// A PTC (P9): the start creates it, the cancel removes it.
static void test_component_invoker(void *user_data, const sce_host_invoke_event_t *event,
                                   sce_host_invoke_response_t *out) {
    (void)out;
    const sce_test_component_context_t *context = (const sce_test_component_context_t *)user_data;
    if (event->phase == SCE_HOST_INVOKE_START) {
        sce_test_context_create(context, event->invoke_id);
    } else {
        sce_test_context_kill(context, event->invoke_id);
    }
}

// An MTC wired as a generated one is: it reaches the doors of the run only
// through the two processors, each handed its context. Entering its state
// invokes `echo` and sends it a `ping_t`; a timeout leaves the state, which
// cancels the invoke.
typedef struct door_tester_s {
    sce_test_component_machine_t machine;
    sce_test_component_context_t context;
    bool ended;
} door_tester_t;

static sce_host_invoke_event_t invoke_event(sce_host_invoke_phase_t phase) {
    sce_host_invoke_event_t event;
    memset(&event, 0, sizeof(event));
    event.phase = phase;
    event.processor_type = "http://sce.dev/ext/test-component";
    event.invoke_id = "echo";
    event.src = "";
    event.content = "";
    event.event_data = "";
    event.params = NULL;
    event.param_count = 0;
    event.session_id = "";
    return event;
}

static void door_tester_start(sce_test_component_machine_t *machine, const sce_test_component_context_t *context) {
    door_tester_t *self = (door_tester_t *)machine;
    self->context = *context;
    const sce_host_invoke_event_t start = invoke_event(SCE_HOST_INVOKE_START);
    sce_host_invoke_response_t answer;
    memset(&answer, 0, sizeof(answer));
    test_component_invoker(&self->context, &start, &answer);
    const ping_t ping = {3u};
    sce_host_send_request_t request;
    request.processor_type = "http://sce.dev/ext/test-port";
    request.event_name = "ping";
    request.target = "p";
    request.content = "";
    request.send_id = "";
    request.params = NULL;
    request.param_count = 0;
    request.event_data = "";
    request.invoke_id = "";
    request.value = SCE_HOST_VALUE(ping_t, &ping);
    sce_host_send_response_list_t responses;
    memset(&responses, 0, sizeof(responses));
    test_port_processor(&self->context, &request, &responses);
    sce_test_context_start_timer(&self->context, "reply", 10);
}

static void door_tester_deliver_snapshot(sce_test_component_machine_t *machine) {
    door_tester_t *self = (door_tester_t *)machine;
    if (!sce_test_context_timed_out(&self->context, "reply")) {
        return;
    }
    sce_test_context_timeout(&self->context, "reply");
    const sce_host_invoke_event_t cancel = invoke_event(SCE_HOST_INVOKE_CANCEL);
    sce_host_invoke_response_t answer;
    memset(&answer, 0, sizeof(answer));
    test_component_invoker(&self->context, &cancel, &answer);
    sce_test_context_setverdict(&self->context, SCE_TEST_VERDICT_PASS, "");
    self->ended = true;
}

static bool door_tester_terminated(const sce_test_component_machine_t *machine) {
    return ((const door_tester_t *)machine)->ended;
}

static const sce_test_component_machine_vtable_t DOOR_TESTER = {door_tester_start, door_tester_deliver_snapshot,
                                                                door_tester_terminated, NULL};

static bool ping_is_three(const void *value, const void *closure) {
    (void)closure;
    return ((const ping_t *)value)->n == 3u;
}

static void echo_reads_ping(const sce_test_component_context_t *context, scripted_t *self) {
    (void)self;
    if (SCE_TEST_RECEIVES(context, "q", "ping", ping_t, ping_is_three, NULL)) {
        (void)sce_test_context_receive(context, "q", NULL);
        sce_test_context_setverdict(context, SCE_TEST_VERDICT_PASS, "the value arrived");
    } else if (sce_test_context_snapshot_present(context, "q")) {
        (void)sce_test_context_receive(context, "q", NULL);
        sce_test_context_setverdict(context, SCE_TEST_VERDICT_FAIL, "another value arrived");
    }
}

// The machines of the doors: the tester above, and a scripted echo.
typedef struct doors_s {
    door_tester_t tester;
    machines_t echoes;
} doors_t;

static sce_test_component_machine_t *doors_make(void *user_data, const char *component_type) {
    doors_t *doors = (doors_t *)user_data;
    if (strcmp(component_type, "Tester") == 0) {
        memset(&doors->tester, 0, sizeof(doors->tester));
        doors->tester.machine.vtable = &DOOR_TESTER;
        return &doors->tester.machine;
    }
    return machines_make(&doors->echoes, component_type);
}

// §12.3: the PTC the invoker's start creates receives the value the processor's
// send carried, by its type, and the invoker's cancel removes it at the
// timeout, before the MTC's verdict.
static void the_doors_a_send_and_a_ptc_go_through_the_host_processors(void) {
    static doors_t doors;
    memset(&doors, 0, sizeof(doors));
    machines_with(&doors.echoes, "Echo", idle, echo_reads_ping);
    sce_test_machine_factory_t factory;
    factory.make = doors_make;
    factory.user_data = &doors;
    trace_adapter_t adapter;
    const sce_test_configuration_t configuration = mtc_and_echo();
    sce_test_case_run_init(&run_storage, &configuration, factory, &replaying(&adapter, NULL, 0u)->adapter, NULL, NULL);
    const sce_test_case_result_t *result = sce_test_case_run_run(&run_storage);
    CHECK(result->verdict == SCE_TEST_VERDICT_PASS, "verdict %d", (int)result->verdict);
    const size_t created = position(result, "echo", SCE_TEST_LOG_CREATE);
    const size_t sent = position(result, "mtc", SCE_TEST_LOG_SEND);
    const size_t received = position(result, "echo", SCE_TEST_LOG_RECEIVE);
    const size_t killed = position(result, "echo", SCE_TEST_LOG_KILL);
    const size_t verdict = position(result, "mtc", SCE_TEST_LOG_VERDICT);
    CHECK(created != SIZE_MAX && sent != SIZE_MAX && received != SIZE_MAX && killed != SIZE_MAX && verdict != SIZE_MAX,
          "the log holds the creation, the send, the receive, the kill and the verdict");
    if (created == SIZE_MAX || sent == SIZE_MAX || received == SIZE_MAX || killed == SIZE_MAX || verdict == SIZE_MAX) {
        return;
    }
    CHECK(created < sent && sent < received, "created, then sent, then received");
    CHECK(strcmp(result->log[sent].detail, "p ping") == 0, "the send is '%s'", result->log[sent].detail);
    CHECK(strcmp(result->log[received].detail, "q ping") == 0, "the receive is '%s'", result->log[received].detail);
    CHECK(result->log[killed].at == 10, "the cancel came at the timeout, not at %lld",
          (long long)result->log[killed].at);
    CHECK(killed < verdict, "the cancel removed the PTC, not the end");
    CHECK(logged(result, "echo", SCE_TEST_LOG_VERDICT), "the echo set its verdict on the value");
}

// ── the_bounds ────────────────────────────────────────────────────────────

// SCE_TEST_MAX_QUEUED_MESSAGES + 1 inputs at NOW = 0 on the reader's port.
static input_t flood[SCE_TEST_MAX_QUEUED_MESSAGES + 1u];

// A run's queues hold SCE_TEST_MAX_QUEUED_MESSAGES messages at once: the next
// input is refused by name, and the run ends with error, never dropping it.
static void the_bounds_an_input_past_the_queues_of_a_run_is_an_error(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(&machines, "Reader", idle, idle);
    for (size_t i = 0u; i < SCE_TEST_MAX_QUEUED_MESSAGES + 1u; ++i) {
        flood[i].at = 0;
        flood[i].system_port = "sut";
        flood[i].event = "number";
    }
    trace_adapter_t adapter;
    const sce_test_configuration_t configuration = mapped(READER_P);
    const sce_test_case_result_t *result = run_case(
        &configuration, &machines, &replaying(&adapter, flood, SCE_TEST_MAX_QUEUED_MESSAGES + 1u)->adapter, NULL);
    CHECK(result->verdict == SCE_TEST_VERDICT_ERROR, "verdict %d", (int)result->verdict);
    CHECK(ends_in_error_naming(result, "SCE_TEST_MAX_QUEUED_MESSAGES"), "the bound is named");
}

// A value larger than a message holds.
typedef struct oversized_s {
    unsigned char bytes[SCE_TEST_MAX_VALUE_SIZE + 1u];
} oversized_t;

static void sends_an_oversized_value(const sce_test_component_context_t *context, scripted_t *self) {
    (void)self;
    static oversized_t value;
    sce_test_context_send(context, "p", "ping", SCE_HOST_VALUE(oversized_t, &value));
}

// A value larger than SCE_TEST_MAX_VALUE_SIZE is refused by name, never cut.
static void the_bounds_a_value_past_its_bound_is_an_error(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(&machines, "Tester", sends_an_oversized_value, idle);
    trace_adapter_t adapter;
    const sce_test_configuration_t configuration = mapped(TESTER_P);
    const sce_test_case_result_t *result =
        run_case(&configuration, &machines, &replaying(&adapter, NULL, 0u)->adapter, NULL);
    CHECK(result->verdict == SCE_TEST_VERDICT_ERROR, "verdict %d", (int)result->verdict);
    CHECK(ends_in_error_naming(result, "SCE_TEST_MAX_VALUE_SIZE"), "the bound is named");
    CHECK(adapter.sent_count == 0u, "nothing reached the adapter");
}

static void sends_a_long_event(const sce_test_component_context_t *context, scripted_t *self) {
    (void)self;
    char event[SCE_MAX_ID_LEN + 1u];
    memset(event, 'e', sizeof(event) - 1u);
    event[sizeof(event) - 1u] = '\0';
    sce_test_context_send(context, "p", event, sce_host_value_none());
}

// An event name longer than SCE_MAX_ID_LEN holds is refused by name: a cut name
// could match another record's branch.
static void the_bounds_an_event_name_past_its_bound_is_an_error(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(&machines, "Tester", sends_a_long_event, idle);
    trace_adapter_t adapter;
    const sce_test_configuration_t configuration = mapped(TESTER_P);
    const sce_test_case_result_t *result =
        run_case(&configuration, &machines, &replaying(&adapter, NULL, 0u)->adapter, NULL);
    CHECK(result->verdict == SCE_TEST_VERDICT_ERROR, "verdict %d", (int)result->verdict);
    CHECK(ends_in_error_naming(result, "SCE_MAX_ID_LEN"), "the bound is named");
}

// SCE_TEST_MAX_COMPONENTS PTCs, which the MTC creates one after the other.
static const sce_test_ptc_t CROWD[] = {{"w1", "Worker"}, {"w2", "Worker"}, {"w3", "Worker"}, {"w4", "Worker"},
                                       {"w5", "Worker"}, {"w6", "Worker"}, {"w7", "Worker"}, {"w8", "Worker"}};

SCE_STATIC_ASSERT(sizeof(CROWD) / sizeof(CROWD[0]) == SCE_TEST_MAX_COMPONENTS,
                  "the crowd is as many PTCs as a run holds components, the MTC one of them");

static void creates_the_crowd(const sce_test_component_context_t *context, scripted_t *self) {
    (void)self;
    for (size_t i = 0u; i < sizeof(CROWD) / sizeof(CROWD[0]); ++i) {
        sce_test_context_create(context, CROWD[i].name);
    }
}

// A run holds SCE_TEST_MAX_COMPONENTS components at once, the MTC among them:
// the create past it is refused by name.
static void the_bounds_a_create_past_the_components_of_a_run_is_an_error(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(machines_with(&machines, "Crowd", creates_the_crowd, idle), "Worker", idle, idle);
    sce_test_configuration_t configuration = lone("Crowd");
    configuration.ptcs = CROWD;
    configuration.ptc_count = SCE_TEST_MAX_COMPONENTS;
    trace_adapter_t adapter;
    const sce_test_case_result_t *result =
        run_case(&configuration, &machines, &replaying(&adapter, NULL, 0u)->adapter, NULL);
    CHECK(result->verdict == SCE_TEST_VERDICT_ERROR, "verdict %d", (int)result->verdict);
    CHECK(has_error_naming(result, "SCE_TEST_MAX_COMPONENTS"), "the bound is named");
    CHECK(logged(result, "w7", SCE_TEST_LOG_CREATE) && !logged(result, "w8", SCE_TEST_LOG_CREATE),
          "seven PTCs beside the MTC were created, and the eighth was not");
}

// A component type that declares one port past SCE_TEST_MAX_PORTS.
static const sce_test_declared_t TOO_MANY_PORTS[] = {{"Wide", "p1"}, {"Wide", "p2"}, {"Wide", "p3"},
                                                     {"Wide", "p4"}, {"Wide", "p5"}, {"Wide", "p6"},
                                                     {"Wide", "p7"}, {"Wide", "p8"}, {"Wide", "p9"}};

SCE_STATIC_ASSERT(sizeof(TOO_MANY_PORTS) / sizeof(TOO_MANY_PORTS[0]) == SCE_TEST_MAX_PORTS + 1u,
                  "the wide type declares one port past the bound");

// A component holds SCE_TEST_MAX_PORTS ports: one of a type that declares more
// is not created, and the run ends with error, naming the bound.
static void the_bounds_a_component_type_of_more_ports_than_a_component_holds_is_an_error(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(&machines, "Wide", ends_pass, idle);
    sce_test_configuration_t configuration = lone("Wide");
    configuration.ports = TOO_MANY_PORTS;
    configuration.port_count = SCE_TEST_MAX_PORTS + 1u;
    trace_adapter_t adapter;
    const sce_test_case_result_t *result =
        run_case(&configuration, &machines, &replaying(&adapter, NULL, 0u)->adapter, NULL);
    CHECK(result->verdict == SCE_TEST_VERDICT_ERROR, "verdict %d", (int)result->verdict);
    CHECK(ends_in_error_naming(result, "SCE_TEST_MAX_PORTS"), "the bound is named");
    CHECK(!logged(result, "mtc", SCE_TEST_LOG_START), "no component of the type was created");
}

static const char *const TIMER_IDS[] = {"t1", "t2", "t3", "t4", "t5", "t6", "t7", "t8", "t9"};

SCE_STATIC_ASSERT(sizeof(TIMER_IDS) / sizeof(TIMER_IDS[0]) == SCE_TEST_MAX_TIMERS + 1u, "one timer id past the bound");

// One module parameter id past the bound.
static const char *const PARAMETER_IDS[] = {"a", "b", "c", "d", "e", "f", "g", "h", "i"};

SCE_STATIC_ASSERT(sizeof(PARAMETER_IDS) / sizeof(PARAMETER_IDS[0]) == SCE_TEST_MAX_MODULE_PARAMETERS + 1u,
                  "one module parameter id past the bound");

static void starts_too_many_timers(const sce_test_component_context_t *context, scripted_t *self) {
    (void)self;
    for (size_t i = 0u; i < SCE_TEST_MAX_TIMERS + 1u; ++i) {
        sce_test_context_start_timer(context, TIMER_IDS[i], SECOND);
    }
}

// A component holds SCE_TEST_MAX_TIMERS timers: a start of one more is refused
// by name.
static void the_bounds_a_timer_past_the_timers_of_a_component_is_an_error(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(&machines, "Timed", starts_too_many_timers, idle);
    trace_adapter_t adapter;
    const sce_test_configuration_t configuration = lone("Timed");
    const sce_test_case_result_t *result =
        run_case(&configuration, &machines, &replaying(&adapter, NULL, 0u)->adapter, NULL);
    CHECK(result->verdict == SCE_TEST_VERDICT_ERROR, "verdict %d", (int)result->verdict);
    CHECK(ends_in_error_naming(result, "SCE_TEST_MAX_TIMERS"), "the bound is named");
}

static void logs_past_the_log(const sce_test_component_context_t *context, scripted_t *self) {
    for (size_t i = 0u; i < SCE_TEST_MAX_LOG_RECORDS; ++i) {
        sce_test_context_log(context, "a line");
    }
    sce_test_context_setverdict(context, SCE_TEST_VERDICT_PASS, "");
    self->ended = true;
}

// A log holds SCE_TEST_MAX_LOG_RECORDS lines: the lines past it are counted,
// not kept, and a run that lost one ends with error whatever its verdicts.
static void the_bounds_a_line_past_the_log_is_counted_and_ends_with_error(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(&machines, "Chatty", logs_past_the_log, idle);
    trace_adapter_t adapter;
    const sce_test_configuration_t configuration = lone("Chatty");
    const sce_test_case_result_t *result =
        run_case(&configuration, &machines, &replaying(&adapter, NULL, 0u)->adapter, NULL);
    CHECK(result->log_count == SCE_TEST_MAX_LOG_RECORDS, "the log holds %zu lines", result->log_count);
    // The start's line, then the logs: two lines (the last log and the verdict) are lost.
    CHECK(result->log_lost == 2u, "%zu lines were lost", result->log_lost);
    CHECK(result->verdict == SCE_TEST_VERDICT_ERROR, "verdict %d", (int)result->verdict);
}

static void logs_a_long_line(const sce_test_component_context_t *context, scripted_t *self) {
    char text[SCE_TEST_MAX_LOG_DETAIL + 1u];
    memset(text, 'x', sizeof(text) - 1u);
    text[sizeof(text) - 1u] = '\0';
    sce_test_context_log(context, text);
    sce_test_context_setverdict(context, SCE_TEST_VERDICT_PASS, "");
    self->ended = true;
}

// A line whose detail is longer than SCE_TEST_MAX_LOG_DETAIL is counted, never
// cut, and ends the run with error.
static void the_bounds_a_line_past_its_detail_is_counted_and_ends_with_error(void) {
    static machines_t machines;
    memset(&machines, 0, sizeof(machines));
    machines_with(&machines, "Chatty", logs_a_long_line, idle);
    trace_adapter_t adapter;
    const sce_test_configuration_t configuration = lone("Chatty");
    const sce_test_case_result_t *result =
        run_case(&configuration, &machines, &replaying(&adapter, NULL, 0u)->adapter, NULL);
    CHECK(result->log_lost == 1u, "%zu lines were lost", result->log_lost);
    CHECK(!logged(result, "mtc", SCE_TEST_LOG_LOG), "no line was cut");
    CHECK(result->verdict == SCE_TEST_VERDICT_ERROR, "verdict %d", (int)result->verdict);
}

// A table of module parameters holds SCE_TEST_MAX_MODULE_PARAMETERS: one more id
// is refused, and a value for an id it holds still replaces the old one.
static void the_bounds_a_module_parameter_past_the_table_is_refused(void) {
    sce_test_module_parameters_t parameters;
    sce_test_module_parameters_init(&parameters);
    for (size_t i = 0u; i < SCE_TEST_MAX_MODULE_PARAMETERS; ++i) {
        CHECK(sce_test_module_parameters_set(&parameters, PARAMETER_IDS[i], "1"), "'%s' fits", PARAMETER_IDS[i]);
    }
    CHECK(!sce_test_module_parameters_set(&parameters, PARAMETER_IDS[SCE_TEST_MAX_MODULE_PARAMETERS], "1"),
          "one id past the table is refused");
    CHECK(sce_test_module_parameters_set(&parameters, "a", "2"), "a held id takes another value");
    sce_jv_t value;
    sce_saved_error_t err;
    CHECK(sce_test_module_parameters_of(&parameters, "a", &value, &err) && sce_jv_number_len(value) == 1u &&
              value.p[0] == '2',
          "the value replaced the old one");
}

int main(void) {
    test_verdict_only_ever_becomes_worse();
    test_port_a_snapshot_sees_only_what_was_queued_when_it_was_taken();
    test_timer_times_out_at_its_instant_and_restarts_when_started_again();
    test_case_run_test_an_answered_request_passes_without_time_moving();
    test_case_run_test_an_unanswered_request_fails_when_its_timer_expires();
    test_case_run_test_a_test_case_that_can_never_end_is_an_error();
    test_case_run_test_the_guard_ends_a_test_case_with_error();
    test_case_run_test_a_mapped_port_talks_to_the_system_adapter_on_the_logical_clock();
    test_case_run_test_a_removed_ptcs_verdict_still_counts();
    test_case_run_test_nothing_selects_after_the_mtc_terminates_in_the_round();
    test_case_run_test_a_done_branch_sees_a_ptc_that_terminated();
    test_case_run_test_a_receive_with_a_template_matches_only_its_records_value();
    test_case_run_test_a_component_reads_the_module_parameter_its_run_is_given();
    test_case_run_test_a_module_parameter_the_run_is_not_given_is_an_error();
    test_case_run_test_a_module_parameter_of_another_type_is_an_error();
    test_case_run_test_a_component_type_with_no_machine_is_an_error();
    test_case_run_test_a_send_to_an_absent_peer_is_an_error();
    test_suite_run_runs_the_test_cases_a_world_selects_in_the_suites_order();
    test_suite_run_each_test_case_is_a_fresh_run_under_its_own_guard();
    test_suite_run_an_override_replaces_the_default_of_a_module_parameter_the_suite_declares();
    test_suite_run_a_test_case_that_reads_a_module_parameter_with_no_value_is_an_error();
    test_suite_run_a_test_case_with_no_system_adapter_is_an_error();
    the_shared_handle_a_kill_of_a_component_whose_machine_runs_is_an_error();
    the_shared_handle_a_context_does_not_outlive_its_component();
    the_run_is_run_once_a_run_that_has_run_is_not_run_again();
    the_doors_a_send_and_a_ptc_go_through_the_host_processors();
    the_bounds_an_input_past_the_queues_of_a_run_is_an_error();
    the_bounds_a_value_past_its_bound_is_an_error();
    the_bounds_an_event_name_past_its_bound_is_an_error();
    the_bounds_a_create_past_the_components_of_a_run_is_an_error();
    the_bounds_a_component_type_of_more_ports_than_a_component_holds_is_an_error();
    the_bounds_a_timer_past_the_timers_of_a_component_is_an_error();
    the_bounds_a_line_past_the_log_is_counted_and_ends_with_error();
    the_bounds_a_line_past_its_detail_is_counted_and_ends_with_error();
    the_bounds_a_module_parameter_past_the_table_is_refused();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)puts("test case run: ok");
    return 0;
}
