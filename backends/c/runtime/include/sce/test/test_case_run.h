/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/**
 * @file test_case_run.h
 * @brief The run of one test case of the test profile (docs/SCE_TEST_PROFILE.md
 *        §2 to §5, §12): its entities, its ports and timers, the round scheduler
 *        on the logical clock, and the test case verdict
 *
 * The C11 port of `sce/include/test/TestCaseRun.h` and
 * `sce_rust_runtime::test::TestCaseRun`, the algorithm the document specifies,
 * function for function. A component's machine is generated from its
 * `test-component` and reaches the run only through its context (§12.3): the
 * snapshot its branches' guards read, the statement each taken branch runs
 * first, the statements of the profile, and the doors of a send and of a PTC.
 * The run knows nothing of the engine or of the record types, so it is written
 * once for every suite and tested on its own.
 *
 * What is C's own, against its siblings:
 *
 *   - Nothing is allocated. A run is one struct the caller places (static
 *     storage on a machine without a heap), every table in it bounded at
 *     compile time: the components present at once (`SCE_TEST_MAX_COMPONENTS`),
 *     the ports and the timers of one component (`SCE_TEST_MAX_PORTS`,
 *     `SCE_TEST_MAX_TIMERS`), the messages waiting in its queues
 *     (`SCE_TEST_MAX_QUEUED_MESSAGES`, sce/test/port.h) and the lines of its log
 *     (`SCE_TEST_MAX_LOG_RECORDS`, each of `SCE_TEST_MAX_LOG_DETAIL` bytes), and
 *     the records of its test log (`SCE_TEST_MAX_TRACE_RECORDS`, their texts in
 *     `SCE_TEST_TRACE_TEXT_SIZE` bytes, sce/test/test_log.h). A run that would
 *     pass one of the first four ends with `error`, the bound named in its log,
 *     as a dynamic error does (P13); a log line or a record of the test log that
 *     does not fit is counted in `log_lost` or `trace.lost`, never cut, and a run
 *     that lost one ends with `error` too, since its log no longer says what
 *     happened.
 *   - A machine and a system adapter are a struct whose first member points to
 *     its table of functions, C's form of the interfaces the siblings declare.
 *     The factory of machines hands out a machine from storage of its own and
 *     takes it back through `release` when its component is removed.
 *   - A context is a handle the machine keeps by value: the run, the
 *     component's slot and which creation of it this is. A call through the
 *     handle of a component that was removed reaches no other component: it
 *     ends the run with `error`, as a call the Rust handle refuses panics there.
 *     A kill of a component whose machine is running up the call stack is
 *     refused the same way, since its storage would go from under it.
 *   - Every name the run keeps (a component's, a port's, a timer's, a type's)
 *     is lent by the configuration, which outlives the run.
 *
 * Header-only and allocation-free.
 */

#ifndef SCE_TEST_TEST_CASE_RUN_H
#define SCE_TEST_TEST_CASE_RUN_H

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "sce/host_processor.h"
#include "sce/saved_json.h"
#include "sce/test/module_parameters.h"
#include "sce/test/port.h"
#include "sce/test/test_log.h"
#include "sce/test/timer.h"
#include "sce/test/verdict.h"

#ifdef __cplusplus
extern "C" {
#endif

/* How many components one run holds at once: the MTC and the PTCs present. The
   profile's examples hold two; eight leaves room for a configuration of a few
   PTCs without reserving much. A create past it ends the run with `error`. */
#ifndef SCE_TEST_MAX_COMPONENTS
#define SCE_TEST_MAX_COMPONENTS 8u
#endif

/* How many ports one component holds. A component type that declares more
   cannot be created, and the run ends with `error` where it would be. */
#ifndef SCE_TEST_MAX_PORTS
#define SCE_TEST_MAX_PORTS 8u
#endif

/* How many timers one component holds: those its type declares and any a
   statement starts by an id of its own. One past it ends the run with `error`. */
#ifndef SCE_TEST_MAX_TIMERS
#define SCE_TEST_MAX_TIMERS 8u
#endif

/* How many lines the log of one run holds. A run of the profile's examples logs
   about a dozen. */
#ifndef SCE_TEST_MAX_LOG_RECORDS
#define SCE_TEST_MAX_LOG_RECORDS 64u
#endif

/* How many bytes the detail of one log line holds, its terminating zero
   included: a port and an event, a verdict and its reason, a `<log>`'s text, or
   the sentence of an error. */
#ifndef SCE_TEST_MAX_LOG_DETAIL
#define SCE_TEST_MAX_LOG_DETAIL 160u
#endif

/** The name of the MTC in a configuration; no PTC takes it (§11.6). */
#define SCE_TEST_MTC_NAME "mtc"

/* ── The configuration (§11.6) ─────────────────────────────────────────── */

/** A component port named in a configuration: the component's name (`mtc` or a
 *  PTC's id) and the port's id. */
typedef struct sce_test_endpoint_s {
    const char *component;
    const char *port;
} sce_test_endpoint_t;

/** A PTC of a configuration: its name and its component type. */
typedef struct sce_test_ptc_s {
    const char *name;
    const char *component_type;
} sce_test_ptc_t;

/** One id a component type declares, a port's or a timer's. */
typedef struct sce_test_declared_s {
    const char *component_type;
    const char *id;
} sce_test_declared_t;

/** A `<sce:connect>` pair (-4 §9.10). */
typedef struct sce_test_connection_s {
    sce_test_endpoint_t a;
    sce_test_endpoint_t b;
} sce_test_connection_t;

/** A `<sce:map>` entry: a component port and its system port (-4 §9.32). */
typedef struct sce_test_mapping_s {
    sce_test_endpoint_t endpoint;
    const char *system_port;
} sce_test_mapping_t;

/**
 * A `test-configuration` as the run reads it.
 *
 * Generated from the configuration document, as tables of constant storage.
 * Each port takes part in one connection or one mapping (§2.2), which the build
 * has checked. The ports and the timers of every component type are one table
 * each, a pair per id, in the order the type declares them.
 */
typedef struct sce_test_configuration_s {
    /** The MTC's component type. */
    const char *mtc_type;
    const sce_test_ptc_t *ptcs;
    size_t ptc_count;
    const sce_test_declared_t *ports;
    size_t port_count;
    const sce_test_declared_t *timers;
    size_t timer_count;
    const sce_test_connection_t *connections;
    size_t connection_count;
    const sce_test_mapping_t *mappings;
    size_t mapping_count;
} sce_test_configuration_t;

/* ── The log and the result (§12.5) ────────────────────────────────────── */

/** What a line of the run's log records (§12.5): the run as a reader follows
 *  it, with the removals of PTCs and the cause of an `error` in the runtime's
 *  words. The test log, which two runtimes compare, is
 *  `sce_test_case_result_t::trace`. */
typedef enum sce_test_log_kind_e {
    /** The MTC was created and started. */
    SCE_TEST_LOG_START = 0,
    /** A PTC was created and started. */
    SCE_TEST_LOG_CREATE,
    /** A component sent on a port. */
    SCE_TEST_LOG_SEND,
    /** A receive branch removed the head of a port. */
    SCE_TEST_LOG_RECEIVE,
    /** A timeout branch took a timer. */
    SCE_TEST_LOG_TIMEOUT,
    /** A PTC terminated (P10). */
    SCE_TEST_LOG_DONE,
    /** A PTC was removed. */
    SCE_TEST_LOG_KILL,
    /** A component set its local verdict. */
    SCE_TEST_LOG_VERDICT,
    /** A `<log>`. */
    SCE_TEST_LOG_LOG,
    /** A dynamic error ended the run (P13). */
    SCE_TEST_LOG_ERROR
} sce_test_log_kind_t;

/** One line of the run's log. */
typedef struct sce_test_log_record_s {
    /** `NOW` when it was recorded. */
    sce_test_instant_t at;
    /** The component it is of, a name the configuration lends. */
    const char *component;
    /** What it records. */
    sce_test_log_kind_t kind;
    /** The port and event, the timer, the verdict and reason, the text or the
        error, by kind; empty for a kind that has none. */
    char detail[SCE_TEST_MAX_LOG_DETAIL];
} sce_test_log_record_t;

/** The outcome of one test case: its verdict, the instant it ended at, the
 *  lines of its log, and its test log. */
typedef struct sce_test_case_result_s {
    /** The test case verdict (P14). */
    sce_test_verdict_t verdict;
    /** `NOW` when the run ended. */
    sce_test_instant_t ended_at;
    /** The lines of the run's log, for a reader, in the order they were
        recorded. */
    sce_test_log_record_t log[SCE_TEST_MAX_LOG_RECORDS];
    size_t log_count;
    /** The lines the log could not hold: past `SCE_TEST_MAX_LOG_RECORDS`, or a
        detail longer than `SCE_TEST_MAX_LOG_DETAIL`. A run that lost one ends
        with `error`. */
    size_t log_lost;
    /** The test log (docs/adr/0091, C3): the records of TCI's test logging
        interface, in the order the run produced them, which
        `sce_test_case_result_canonical_log` writes. */
    sce_test_trace_t trace;
} sce_test_case_result_t;

/**
 * The test log of this run of the test case `test_case` of the suite `suite`,
 * in its one canonical text (`sce_test_trace_write`), into the `cap` bytes at
 * `buf`, or measured where `buf` is NULL; `*len`, where `len` is not NULL, the
 * length of the text whether or not it fit. A caller with an allocator measures,
 * then writes into `*len + 1` bytes.
 */
static inline sce_test_trace_written_t sce_test_case_result_canonical_log(const sce_test_case_result_t *result,
                                                                          const char *suite, const char *test_case,
                                                                          char *buf, size_t cap, size_t *len) {
    return sce_test_trace_write(&result->trace, suite, test_case, result->verdict, buf, cap, len);
}

/* ── The SUT's side, and a component's machine (§12.2, §12.3) ──────────── */

typedef struct sce_test_system_adapter_s sce_test_system_adapter_t;

/**
 * The functions of a system adapter: the SUT's side of the mapped ports.
 *
 * Shaped after TRI (ES 201 873-5): `send` is `triSend`, and the inputs
 * `take_input_due` hands out are what `triEnqueueMsg` would enqueue. The
 * adapter owns the codec and the transport; the run sees values only (ADR 0091
 * C2). The instants are the logical clock's, so a recorded trace replays to the
 * same run (§3.1).
 */
typedef struct sce_test_system_adapter_vtable_s {
    /** A component sent `message` on a port mapped to `system_port`. */
    void (*send)(sce_test_system_adapter_t *self, const char *system_port, const sce_test_message_t *message,
                 sce_test_instant_t now);
    /** The instant of the earliest input not yet taken, in `*at`; false when
        there is none. */
    bool (*next_input)(const sce_test_system_adapter_t *self, sce_test_instant_t *at);
    /** The earliest input due at or before `now`, taken: its system port in
        `*system_port` (a name that outlives the call) and its message in
        `*message`; false when no input is due. The run asks until it answers
        false, so the inputs arrive in their order. */
    bool (*take_input_due)(sce_test_system_adapter_t *self, sce_test_instant_t now, const char **system_port,
                           sce_test_message_t *message);
    /** The adapter is done with, after the test case it was built for; NULL
        for an adapter that holds nothing to give back. */
    void (*release)(sce_test_system_adapter_t *self);
} sce_test_system_adapter_vtable_t;

/** A system adapter: the first member of the struct of each kind of adapter. */
struct sce_test_system_adapter_s {
    const sce_test_system_adapter_vtable_t *vtable;
};

struct sce_test_case_run_s;

/**
 * What a component's machine reads and does through the run (§12.3): a handle,
 * which the machine keeps by value from `start` on. Every call takes a pointer
 * to the handle the machine keeps.
 */
typedef struct sce_test_component_context_s {
    struct sce_test_case_run_s *run;
    /** The component's name: `mtc`, or the PTC's id. */
    const char *name;
    size_t slot;
    uint64_t serial;
} sce_test_component_context_t;

typedef struct sce_test_component_machine_s sce_test_component_machine_t;

/**
 * The functions of a component's machine, as the run drives it.
 *
 * Generated from a `test-component`: the backend's statechart machine, with
 * each branch a transition on the round's snapshot event.
 */
typedef struct sce_test_component_machine_vtable_s {
    /** Enter the initial configuration and complete the first macrostep. The
        machine keeps `*context`, a copy of it, and every call it makes, now
        and on each later delivery, goes to it. */
    void (*start)(sce_test_component_machine_t *self, const sce_test_component_context_t *context);
    /** Deliver the round's snapshot event: the machine takes the first branch
        whose guard holds on the snapshot (P3) and completes its macrostep
        (P4), or takes none. */
    void (*deliver_snapshot)(sce_test_component_machine_t *self);
    /** The machine has entered a top-level `<final>` (P10). */
    bool (*terminated)(const sce_test_component_machine_t *self);
    /** The machine's component is removed, or its run has ended: the factory
        that made it may hand its storage out again. NULL for a machine whose
        storage is given back by no call. */
    void (*release)(sce_test_component_machine_t *self);
} sce_test_component_machine_vtable_t;

/** A component's machine: the first member of the struct of each kind. */
struct sce_test_component_machine_s {
    const sce_test_component_machine_vtable_t *vtable;
};

/**
 * Builds the machine of a component type, from storage of its own, or none
 * (NULL) for a type it does not make, which ends the run with `error` where a
 * component of that type is created. `user_data` is handed back unchanged.
 */
typedef struct sce_test_machine_factory_s {
    sce_test_component_machine_t *(*make)(void *user_data, const char *component_type);
    void *user_data;
} sce_test_machine_factory_t;

/* ── The run (§2.3, §3, §4.9 to §4.11, §5) ─────────────────────────────── */

/** What one component holds in the run. */
typedef struct sce_test_entity_s {
    bool in_use;
    /** Which creation this is: a PTC removed and created again in one round is
        a new component, which took no snapshot in it. */
    uint64_t serial;
    const char *name;
    const char *component_type;
    sce_test_component_machine_t *machine;
    /** Its machine is being called, up the call stack. */
    bool running;
    bool terminated;
    /** In `DONE`: a PTC that terminated, until it is removed (P10). */
    bool done;
    sce_test_verdict_t verdict;
    bool branch_taken;
    sce_test_port_t ports[SCE_TEST_MAX_PORTS];
    size_t port_count;
    sce_test_timer_t timers[SCE_TEST_MAX_TIMERS];
    size_t timer_count;
    /** The PTCs the last snapshot saw in `DONE`, by name. */
    const char *done_snapshot[SCE_TEST_MAX_COMPONENTS];
    size_t done_snapshot_count;
} sce_test_entity_t;

/**
 * One run of one test case.
 *
 * The components run on the calling thread, in the order they were created;
 * §3.2 shows that the order within a round does not change the result. Placed
 * by the caller and set up by `sce_test_case_run_init`; its members are the
 * run's own.
 */
typedef struct sce_test_case_run_s {
    const sce_test_configuration_t *configuration;
    sce_test_machine_factory_t factory;
    sce_test_system_adapter_t *adapter;
    bool guarded;
    sce_test_duration_t guard;
    sce_test_module_parameters_t module_parameters;

    sce_test_instant_t now;
    uint64_t created;
    sce_test_entity_t entities[SCE_TEST_MAX_COMPONENTS];
    /** The slots of the components present, in the order they were created. */
    size_t order[SCE_TEST_MAX_COMPONENTS];
    size_t order_count;
    sce_test_message_pool_t pool;
    sce_test_verdict_t removed_verdict;
    bool mtc_terminated;
    bool error;
    bool has_run;
    sce_test_case_result_t result;
} sce_test_case_run_t;

/* ── The verdict and the log (§5, §12.5) ───────────────────────────────── */

static inline void sce_test_run_vrecord_(sce_test_case_run_t *run, const char *component, sce_test_log_kind_t kind,
                                         const char *format, va_list args) {
    sce_test_case_result_t *result = &run->result;
    if (result->log_count >= SCE_TEST_MAX_LOG_RECORDS) {
        result->log_lost++;
        return;
    }
    sce_test_log_record_t *record = &result->log[result->log_count];
    const int written = vsnprintf(record->detail, sizeof(record->detail), format, args);
    if (written < 0 || (size_t)written >= sizeof(record->detail)) {
        record->detail[0] = '\0';
        result->log_lost++;
        return;
    }
    record->at = run->now;
    record->component = component;
    record->kind = kind;
    result->log_count++;
}

/* One line of the log, its detail written by `format`. */
static inline void sce_test_run_record_(sce_test_case_run_t *run, const char *component, sce_test_log_kind_t kind,
                                        const char *format, ...) {
    va_list args;
    va_start(args, format);
    sce_test_run_vrecord_(run, component, kind, format, args);
    va_end(args);
}

/* A dynamic error (P13): the run ends with `error`, the reason in the log's
   line. The test log records the verdict without the reason, which is this
   runtime's own prose. */
static inline void sce_test_run_fail_(sce_test_case_run_t *run, const char *component, const char *format, ...) {
    va_list args;
    run->error = true;
    va_start(args, format);
    sce_test_run_vrecord_(run, component, SCE_TEST_LOG_ERROR, format, args);
    va_end(args);
    sce_test_trace_verdict(&run->result.trace, run->now, component, SCE_TEST_VERDICT_ERROR, NULL);
}

static inline bool sce_test_run_ended_(const sce_test_case_run_t *run) {
    return run->mtc_terminated || run->error;
}

/* ── Entities (§2.1, P9) ───────────────────────────────────────────────── */

/* The component present by the name `name`, or NULL. */
static inline sce_test_entity_t *sce_test_run_find_(sce_test_case_run_t *run, const char *name) {
    for (size_t slot = 0u; slot < SCE_TEST_MAX_COMPONENTS; ++slot) {
        sce_test_entity_t *entity = &run->entities[slot];
        if (entity->in_use && strcmp(entity->name, name) == 0) {
            return entity;
        }
    }
    return NULL;
}

/* The port `id` of `entity`, or NULL; one is added where `add` and there is room. */
static inline sce_test_port_t *sce_test_entity_port_(sce_test_entity_t *entity, const char *id, bool add) {
    for (size_t i = 0u; i < entity->port_count; ++i) {
        if (strcmp(entity->ports[i].id, id) == 0) {
            return &entity->ports[i];
        }
    }
    if (!add || entity->port_count >= SCE_TEST_MAX_PORTS) {
        return NULL;
    }
    sce_test_port_t *port = &entity->ports[entity->port_count++];
    sce_test_port_init(port, id);
    return port;
}

/* The timer `id` of `entity`, or NULL; one is added where `add` and there is room. */
static inline sce_test_timer_t *sce_test_entity_timer_(sce_test_entity_t *entity, const char *id, bool add) {
    for (size_t i = 0u; i < entity->timer_count; ++i) {
        if (strcmp(entity->timers[i].id, id) == 0) {
            return &entity->timers[i];
        }
    }
    if (!add || entity->timer_count >= SCE_TEST_MAX_TIMERS) {
        return NULL;
    }
    sce_test_timer_t *timer = &entity->timers[entity->timer_count++];
    sce_test_timer_init(timer, id);
    return timer;
}

/* A component that has just run: if it terminated, P10 applies. */
static inline void sce_test_run_settle_(sce_test_case_run_t *run, sce_test_entity_t *entity) {
    if (!entity->in_use || entity->terminated || !entity->machine->vtable->terminated(entity->machine)) {
        return;
    }
    entity->terminated = true;
    sce_test_trace_done(&run->result.trace, run->now, entity->name, entity->verdict);
    if (strcmp(entity->name, SCE_TEST_MTC_NAME) == 0) {
        run->mtc_terminated = true;
        return;
    }
    entity->done = true;
    sce_test_run_record_(run, entity->name, SCE_TEST_LOG_DONE, "");
}

/* Removes a component: its local verdict still counts (-4 §9.29a.2), and its
   ports are gone, so a send to one of them is an error. Its machine is handed
   back to its factory last, once the run no longer names it. */
static inline void sce_test_run_remove_(sce_test_case_run_t *run, sce_test_entity_t *entity) {
    sce_test_component_machine_t *machine = entity->machine;
    run->removed_verdict = sce_test_worse(run->removed_verdict, entity->verdict);
    sce_test_run_record_(run, entity->name, SCE_TEST_LOG_KILL, "");
    for (size_t i = 0u; i < entity->port_count; ++i) {
        sce_test_port_clear(&entity->ports[i], &run->pool);
    }
    const size_t slot = (size_t)(entity - run->entities);
    size_t kept = 0u;
    for (size_t i = 0u; i < run->order_count; ++i) {
        if (run->order[i] != slot) {
            run->order[kept++] = run->order[i];
        }
    }
    run->order_count = kept;
    entity->in_use = false;
    entity->machine = NULL;
    if (machine != NULL && machine->vtable->release != NULL) {
        machine->vtable->release(machine);
    }
}

/* Enters a new component of the type `component_type` into a free slot, with
   the ports and the timers its type declares, and logs its creation; NULL, with
   the dynamic error, when the run holds no room for it. */
static inline sce_test_entity_t *sce_test_run_enter_(sce_test_case_run_t *run, const char *name,
                                                     const char *component_type) {
    sce_test_entity_t *entity = NULL;
    for (size_t slot = 0u; slot < SCE_TEST_MAX_COMPONENTS; ++slot) {
        if (!run->entities[slot].in_use) {
            entity = &run->entities[slot];
            break;
        }
    }
    if (entity == NULL) {
        sce_test_run_fail_(run, name,
                           "is created, but a run holds no more than %u components (SCE_TEST_MAX_COMPONENTS)",
                           (unsigned)SCE_TEST_MAX_COMPONENTS);
        return NULL;
    }
    memset(entity, 0, sizeof(*entity));
    entity->name = name;
    entity->component_type = component_type;
    const sce_test_configuration_t *configuration = run->configuration;
    for (size_t i = 0u; i < configuration->port_count; ++i) {
        if (strcmp(configuration->ports[i].component_type, component_type) != 0) {
            continue;
        }
        if (sce_test_entity_port_(entity, configuration->ports[i].id, true) == NULL) {
            sce_test_run_fail_(run, name,
                               "is of the component type '%s', which declares more ports than a component holds (%u, "
                               "SCE_TEST_MAX_PORTS)",
                               component_type, (unsigned)SCE_TEST_MAX_PORTS);
            return NULL;
        }
    }
    for (size_t i = 0u; i < configuration->timer_count; ++i) {
        if (strcmp(configuration->timers[i].component_type, component_type) != 0) {
            continue;
        }
        if (sce_test_entity_timer_(entity, configuration->timers[i].id, true) == NULL) {
            sce_test_run_fail_(run, name,
                               "is of the component type '%s', which declares more timers than a component holds (%u, "
                               "SCE_TEST_MAX_TIMERS)",
                               component_type, (unsigned)SCE_TEST_MAX_TIMERS);
            return NULL;
        }
    }
    entity->in_use = true;
    entity->serial = ++run->created;
    run->order[run->order_count++] = (size_t)(entity - run->entities);
    sce_test_run_record_(run, name, strcmp(name, SCE_TEST_MTC_NAME) == 0 ? SCE_TEST_LOG_START : SCE_TEST_LOG_CREATE,
                         "%s", component_type);
    sce_test_trace_created(&run->result.trace, run->now, name, component_type);
    return entity;
}

/* Creates the component `name` of the type `component_type` and starts its
   machine, which may create, send and kill before it returns. */
static inline void sce_test_run_create_entity_(sce_test_case_run_t *run, const char *name, const char *component_type) {
    sce_test_component_machine_t *machine = run->factory.make(run->factory.user_data, component_type);
    if (machine == NULL) {
        sce_test_run_fail_(run, name, "is of the component type '%s', which the factory of machines does not make",
                           component_type);
        return;
    }
    sce_test_entity_t *entity = sce_test_run_enter_(run, name, component_type);
    if (entity == NULL) {
        if (machine->vtable->release != NULL) {
            machine->vtable->release(machine);
        }
        return;
    }
    entity->machine = machine;
    sce_test_component_context_t context;
    context.run = run;
    context.name = name;
    context.slot = (size_t)(entity - run->entities);
    context.serial = entity->serial;
    entity->running = true;
    machine->vtable->start(machine, &context);
    entity->running = false;
    sce_test_run_settle_(run, entity);
}

/* The start of the invocation that creates the PTC `ptc`, by `creator` (P9). */
static inline void sce_test_run_create_(sce_test_case_run_t *run, const char *creator, const char *ptc) {
    const sce_test_ptc_t *declared = NULL;
    for (size_t i = 0u; i < run->configuration->ptc_count; ++i) {
        if (strcmp(run->configuration->ptcs[i].name, ptc) == 0) {
            declared = &run->configuration->ptcs[i];
            break;
        }
    }
    if (declared == NULL) {
        sce_test_run_fail_(run, creator, "creates '%s', which the configuration does not declare", ptc);
        return;
    }
    if (sce_test_run_find_(run, ptc) != NULL) {
        sce_test_run_fail_(run, creator, "creates '%s', which is already present", ptc);
        return;
    }
    sce_test_run_create_entity_(run, declared->name, declared->component_type);
}

/* The kill of the PTC `ptc` by `killer` (P9). A component whose machine is
   running, up the call stack of this one, is not removed under it: that is a
   dynamic error. */
static inline void sce_test_run_kill_(sce_test_case_run_t *run, const char *killer, const char *ptc) {
    sce_test_entity_t *entity = sce_test_run_find_(run, ptc);
    if (entity == NULL) {
        return;
    }
    if (entity->running) {
        sce_test_run_fail_(run, killer, "kills '%s', whose machine is running", ptc);
        return;
    }
    sce_test_run_remove_(run, entity);
}

/* ── Ports (§2.2, P6) ──────────────────────────────────────────────────── */

/* `message` into the port `port` of `entity`, for `actor`, or the dynamic error
   of a bound it meets. */
static inline void sce_test_run_enqueue_(sce_test_case_run_t *run, const char *actor, sce_test_entity_t *entity,
                                         const char *port, const sce_test_message_t *message) {
    sce_test_port_t *queue = sce_test_entity_port_(entity, port, true);
    if (queue == NULL) {
        sce_test_run_fail_(run, actor,
                           "reaches the port '%s' of '%s', past the ports a component holds (%u, "
                           "SCE_TEST_MAX_PORTS)",
                           port, entity->name, (unsigned)SCE_TEST_MAX_PORTS);
        return;
    }
    if (!sce_test_port_enqueue(queue, &run->pool, message)) {
        sce_test_run_fail_(run, actor,
                           "queues '%s' on the port '%s' of '%s', but the queues of a run hold no more than %u "
                           "messages (SCE_TEST_MAX_QUEUED_MESSAGES)",
                           message->event, port, entity->name, (unsigned)SCE_TEST_MAX_QUEUED_MESSAGES);
    }
}

static inline void sce_test_run_send_(sce_test_case_run_t *run, const char *sender, const char *port, const char *event,
                                      sce_test_message_value_t value) {
    sce_test_message_t message;
    switch (sce_test_message_set(&message, event, value)) {
    case SCE_TEST_MESSAGE_HELD:
        break;
    case SCE_TEST_MESSAGE_EVENT_TOO_LONG:
        sce_test_run_fail_(run, sender, "sends on '%s' an event whose name is longer than SCE_MAX_ID_LEN (%u) holds",
                           port, (unsigned)SCE_MAX_ID_LEN);
        return;
    case SCE_TEST_MESSAGE_VALUE_TOO_LARGE:
        sce_test_run_fail_(run, sender, "sends '%s' on '%s', a value of %lu bytes, past SCE_TEST_MAX_VALUE_SIZE (%u)",
                           event, port, (unsigned long)value.value.size, (unsigned)SCE_TEST_MAX_VALUE_SIZE);
        return;
    case SCE_TEST_MESSAGE_UNTYPED:
        sce_test_run_fail_(run, sender,
                           "sends '%s' on '%s', a value that names no record type the test log names and writes it by",
                           event, port);
        return;
    }
    sce_test_run_record_(run, sender, SCE_TEST_LOG_SEND, "%s %s", port, event);
    sce_test_trace_message(&run->result.trace, run->now, SCE_TEST_TRACE_MESSAGE_SENT, sender, port, &message);
    const sce_test_configuration_t *configuration = run->configuration;
    for (size_t i = 0u; i < configuration->connection_count; ++i) {
        const sce_test_connection_t *connection = &configuration->connections[i];
        const sce_test_endpoint_t *to = NULL;
        if (strcmp(connection->a.component, sender) == 0 && strcmp(connection->a.port, port) == 0) {
            to = &connection->b;
        } else if (strcmp(connection->b.component, sender) == 0 && strcmp(connection->b.port, port) == 0) {
            to = &connection->a;
        } else {
            continue;
        }
        sce_test_entity_t *peer = sce_test_run_find_(run, to->component);
        if (peer == NULL) {
            sce_test_run_fail_(run, sender, "sends on '%s', whose peer '%s' is not present", port, to->component);
            return;
        }
        sce_test_run_enqueue_(run, sender, peer, to->port, &message);
        return;
    }
    for (size_t i = 0u; i < configuration->mapping_count; ++i) {
        const sce_test_mapping_t *mapping = &configuration->mappings[i];
        if (strcmp(mapping->endpoint.component, sender) == 0 && strcmp(mapping->endpoint.port, port) == 0) {
            run->adapter->vtable->send(run->adapter, mapping->system_port, &message, run->now);
            return;
        }
    }
    sce_test_run_fail_(run, sender, "sends on '%s', which is neither connected nor mapped", port);
}

static inline void sce_test_run_deliver_input_(sce_test_case_run_t *run, const char *system_port,
                                               const sce_test_message_t *message) {
    if (!sce_test_record_type_complete(message->record_type)) {
        sce_test_run_fail_(run, SCE_TEST_MTC_NAME,
                           "an input '%s' on system port '%s' names no record type the test log names and writes it by",
                           message->event, system_port);
        return;
    }
    const sce_test_configuration_t *configuration = run->configuration;
    for (size_t i = 0u; i < configuration->mapping_count; ++i) {
        const sce_test_mapping_t *mapping = &configuration->mappings[i];
        if (strcmp(mapping->system_port, system_port) != 0) {
            continue;
        }
        sce_test_entity_t *component = sce_test_run_find_(run, mapping->endpoint.component);
        if (component == NULL) {
            sce_test_run_fail_(run, mapping->endpoint.component, "an input on system port '%s' found no component",
                               system_port);
            return;
        }
        sce_test_run_enqueue_(run, SCE_TEST_MTC_NAME, component, mapping->endpoint.port, message);
        return;
    }
    sce_test_run_fail_(run, SCE_TEST_MTC_NAME, "an input on system port '%s', which no port maps", system_port);
}

/* ── The round (§3.2, §12.4) ───────────────────────────────────────────── */

/* No branch fired: time advances to the next instant something can happen at
   (§3.3), or the test case can never end (P13). */
static inline void sce_test_run_advance_(sce_test_case_run_t *run) {
    sce_test_instant_t next = 0;
    bool pending = run->adapter->vtable->next_input(run->adapter, &next);
    for (size_t slot = 0u; slot < SCE_TEST_MAX_COMPONENTS; ++slot) {
        const sce_test_entity_t *entity = &run->entities[slot];
        if (!entity->in_use || entity->terminated) {
            continue;
        }
        for (size_t i = 0u; i < entity->timer_count; ++i) {
            sce_test_instant_t expiry = 0;
            if (sce_test_timer_expiry(&entity->timers[i], &expiry) && (!pending || expiry < next)) {
                next = expiry;
                pending = true;
            }
        }
    }
    if (run->guarded && (!pending || run->guard < next)) {
        next = run->guard;
        pending = true;
    }
    if (!pending) {
        sce_test_run_fail_(run, SCE_TEST_MTC_NAME,
                           "no branch can fire and no instant is pending: the test case can never end");
        return;
    }
    if (next > run->now) {
        run->now = next;
    }
    if (run->guarded && run->now >= run->guard) {
        sce_test_run_fail_(run, SCE_TEST_MTC_NAME, "the guard of the test case was reached");
    }
}

/* One round. */
static inline void sce_test_run_round_(sce_test_case_run_t *run) {
    /* Step 2, the update: inputs and timer expiries due at NOW. */
    const char *system_port = NULL;
    sce_test_message_t input;
    while (run->adapter->vtable->take_input_due(run->adapter, run->now, &system_port, &input)) {
        sce_test_run_deliver_input_(run, system_port, &input);
    }
    for (size_t slot = 0u; slot < SCE_TEST_MAX_COMPONENTS; ++slot) {
        sce_test_entity_t *entity = &run->entities[slot];
        if (!entity->in_use) {
            continue;
        }
        for (size_t i = 0u; i < entity->timer_count; ++i) {
            sce_test_timer_update(&entity->timers[i], run->now);
        }
    }
    /* Step 3, the snapshot: every waiting component at once. */
    size_t waiting_slot[SCE_TEST_MAX_COMPONENTS];
    uint64_t waiting_serial[SCE_TEST_MAX_COMPONENTS];
    size_t waiting = 0u;
    for (size_t at = 0u; at < run->order_count; ++at) {
        sce_test_entity_t *entity = &run->entities[run->order[at]];
        if (!entity->in_use || entity->terminated) {
            continue;
        }
        for (size_t i = 0u; i < entity->port_count; ++i) {
            sce_test_port_take_snapshot(&entity->ports[i]);
        }
        for (size_t i = 0u; i < entity->timer_count; ++i) {
            sce_test_timer_take_snapshot(&entity->timers[i]);
        }
        entity->done_snapshot_count = 0u;
        for (size_t slot = 0u; slot < SCE_TEST_MAX_COMPONENTS; ++slot) {
            const sce_test_entity_t *other = &run->entities[slot];
            if (other->in_use && other->done) {
                entity->done_snapshot[entity->done_snapshot_count++] = other->name;
            }
        }
        waiting_slot[waiting] = run->order[at];
        waiting_serial[waiting] = entity->serial;
        waiting++;
    }
    /* Steps 4 and 5: one component at a time, in creation order, each selects
       on its own snapshot and runs the macrostep of the branch it took. A
       component removed or terminated earlier in the round does not select,
       and nothing does once the MTC has terminated (§3.2). */
    bool any_taken = false;
    for (size_t i = 0u; i < waiting; ++i) {
        if (sce_test_run_ended_(run)) {
            break;
        }
        sce_test_entity_t *entity = &run->entities[waiting_slot[i]];
        if (!entity->in_use || entity->serial != waiting_serial[i] || entity->terminated) {
            continue;
        }
        entity->branch_taken = false;
        entity->running = true;
        entity->machine->vtable->deliver_snapshot(entity->machine);
        entity->running = false;
        any_taken = any_taken || entity->branch_taken;
        sce_test_run_settle_(run, entity);
    }
    if (sce_test_run_ended_(run) || any_taken) {
        return;
    }
    sce_test_run_advance_(run);
}

/* The MTC terminated or the run ended in error: every PTC still present is
   removed, in the order of their names, and the test case verdict is the worst
   local verdict (P10, P14). The MTC's machine is handed back too, so that no
   machine outlives its run holding a context of it. */
static inline void sce_test_run_finish_(sce_test_case_run_t *run) {
    for (;;) {
        sce_test_entity_t *first = NULL;
        for (size_t slot = 0u; slot < SCE_TEST_MAX_COMPONENTS; ++slot) {
            sce_test_entity_t *entity = &run->entities[slot];
            if (entity->in_use && strcmp(entity->name, SCE_TEST_MTC_NAME) != 0 &&
                (first == NULL || strcmp(entity->name, first->name) < 0)) {
                first = entity;
            }
        }
        if (first == NULL) {
            break;
        }
        sce_test_run_remove_(run, first);
    }
    sce_test_verdict_t verdict = run->removed_verdict;
    sce_test_entity_t *mtc = sce_test_run_find_(run, SCE_TEST_MTC_NAME);
    if (mtc != NULL) {
        verdict = sce_test_worse(verdict, mtc->verdict);
        sce_test_component_machine_t *machine = mtc->machine;
        mtc->machine = NULL;
        mtc->in_use = false;
        if (machine != NULL && machine->vtable->release != NULL) {
            machine->vtable->release(machine);
        }
    }
    const bool lost = run->result.log_lost > 0u || run->result.trace.lost > 0u;
    if (lost && !run->error) {
        /* A line or a record past this runtime's bounds: its own error (P13). */
        sce_test_trace_verdict(&run->result.trace, run->now, SCE_TEST_MTC_NAME, SCE_TEST_VERDICT_ERROR, NULL);
    }
    if (run->error || lost) {
        verdict = SCE_TEST_VERDICT_ERROR;
    }
    run->result.verdict = verdict;
    run->result.ended_at = run->now;
    sce_test_trace_ended(&run->result.trace, run->now, verdict);
}

/**
 * A run of the test case whose configuration is `*configuration`, whose
 * machines `factory` builds, against `*adapter`, in `*run`.
 *
 * `guard` is the test case's guard duration (P11), or NULL for none, and
 * `module_parameters` the values of the module parameters the components read
 * (§11.8): the suite's, with any the caller overrides, or NULL for none. The
 * configuration and the adapter are lent and outlive the run; the module
 * parameters are copied, and lend their ids and texts.
 */
static inline void sce_test_case_run_init(sce_test_case_run_t *run, const sce_test_configuration_t *configuration,
                                          sce_test_machine_factory_t factory, sce_test_system_adapter_t *adapter,
                                          const sce_test_duration_t *guard,
                                          const sce_test_module_parameters_t *module_parameters) {
    memset(run, 0, sizeof(*run));
    run->configuration = configuration;
    run->factory = factory;
    run->adapter = adapter;
    run->guarded = guard != NULL;
    run->guard = guard != NULL ? *guard : 0;
    if (module_parameters != NULL) {
        run->module_parameters = *module_parameters;
    } else {
        sce_test_module_parameters_init(&run->module_parameters);
    }
    sce_test_message_pool_init(&run->pool);
    run->removed_verdict = SCE_TEST_VERDICT_NONE;
}

/**
 * Runs the test case to its end and returns its verdict and log, which live in
 * `*run`. A run is run once: a run that has run already returns NULL, since it
 * has read its adapter and holds its components' end; initialize it again to
 * run the test case again.
 */
static inline const sce_test_case_result_t *sce_test_case_run_run(sce_test_case_run_t *run) {
    if (run->has_run) {
        return NULL;
    }
    run->has_run = true;
    run->now = 0;
    sce_test_trace_started(&run->result.trace, run->now);
    sce_test_run_create_entity_(run, SCE_TEST_MTC_NAME, run->configuration->mtc_type);
    while (!sce_test_run_ended_(run)) {
        sce_test_run_round_(run);
    }
    sce_test_run_finish_(run);
    return &run->result;
}

/* ── The context, which forwards to its run (§12.3) ────────────────────── */

/* The component `context` names, or NULL, with the dynamic error, when it was
   removed: a context names one creation of its component and reaches no other. */
static inline sce_test_entity_t *sce_test_context_entity_(const sce_test_component_context_t *context) {
    sce_test_case_run_t *run = context->run;
    if (context->slot < SCE_TEST_MAX_COMPONENTS) {
        sce_test_entity_t *entity = &run->entities[context->slot];
        if (entity->in_use && entity->serial == context->serial) {
            return entity;
        }
    }
    sce_test_run_fail_(run, context->name, "the component %s was removed, and its context outlived it", context->name);
    return NULL;
}

/** The component's name: `mtc`, or the PTC's id. */
static inline const char *sce_test_context_name(const sce_test_component_context_t *context) {
    return context->name;
}

/** `NOW`. */
static inline sce_test_instant_t sce_test_context_now(const sce_test_component_context_t *context) {
    return context->run->now;
}

/* ── The snapshot, read by the branches' guards (P2) ── */

/** The message the snapshot saw at the head of `port`, or NULL. */
static inline const sce_test_message_t *sce_test_context_snapshot_of(const sce_test_component_context_t *context,
                                                                     const char *port) {
    sce_test_entity_t *entity = sce_test_context_entity_(context);
    if (entity == NULL) {
        return NULL;
    }
    const sce_test_port_t *queue = sce_test_entity_port_(entity, port, false);
    return queue == NULL ? NULL : sce_test_port_snapshot(queue, &context->run->pool);
}

/** The snapshot saw a message at the head of `port`: the guard of a receive any
 *  branch. */
static inline bool sce_test_context_snapshot_present(const sce_test_component_context_t *context, const char *port) {
    return sce_test_context_snapshot_of(context, port) != NULL;
}

/** The snapshot saw `timer` timed out. */
static inline bool sce_test_context_timed_out(const sce_test_component_context_t *context, const char *timer) {
    sce_test_entity_t *entity = sce_test_context_entity_(context);
    if (entity == NULL) {
        return false;
    }
    const sce_test_timer_t *binding = sce_test_entity_timer_(entity, timer, false);
    return binding != NULL && sce_test_timer_snapshot_timed_out(binding);
}

/** The snapshot saw the PTC `ptc` in `DONE`. */
static inline bool sce_test_context_done(const sce_test_component_context_t *context, const char *ptc) {
    const sce_test_entity_t *entity = sce_test_context_entity_(context);
    if (entity == NULL) {
        return false;
    }
    for (size_t i = 0u; i < entity->done_snapshot_count; ++i) {
        if (strcmp(entity->done_snapshot[i], ptc) == 0) {
            return true;
        }
    }
    return false;
}

/* ── Taking a branch (P4 step 1), the first statement of each ── */

/**
 * A receive branch: remove the head of `port`, which its content reads as
 * `_event.data`, into `*out` where `out` is not NULL. False, with the dynamic
 * error, when the component type declares no port `port` or its queue is
 * empty: a receive branch is taken only on a snapshot that saw a message there.
 */
static inline bool sce_test_context_receive(const sce_test_component_context_t *context, const char *port,
                                            sce_test_message_t *out) {
    sce_test_case_run_t *run = context->run;
    sce_test_entity_t *entity = sce_test_context_entity_(context);
    if (entity == NULL) {
        return false;
    }
    entity->branch_taken = true;
    sce_test_port_t *queue = sce_test_entity_port_(entity, port, false);
    if (queue == NULL) {
        sce_test_run_fail_(run, context->name, "receives on '%s', which its component type does not declare", port);
        return false;
    }
    const sce_test_message_t *head = sce_test_port_head(queue, &run->pool);
    if (head == NULL) {
        sce_test_run_fail_(run, context->name,
                           "receives on '%s', whose queue is empty: a receive branch is taken only on a snapshot "
                           "that saw a message",
                           port);
        return false;
    }
    sce_test_run_record_(run, context->name, SCE_TEST_LOG_RECEIVE, "%s %s", port, head->event);
    sce_test_trace_message(&run->result.trace, run->now, SCE_TEST_TRACE_MESSAGE_RECEIVED, context->name, port, head);
    return sce_test_port_remove_head(queue, &run->pool, out);
}

/** A timeout branch: `timer` becomes idle. The dynamic error where the
 *  component type declares no timer `timer`. */
static inline void sce_test_context_timeout(const sce_test_component_context_t *context, const char *timer) {
    sce_test_case_run_t *run = context->run;
    sce_test_entity_t *entity = sce_test_context_entity_(context);
    if (entity == NULL) {
        return;
    }
    entity->branch_taken = true;
    sce_test_timer_t *binding = sce_test_entity_timer_(entity, timer, false);
    if (binding == NULL) {
        sce_test_run_fail_(run, context->name, "times out on '%s', which its component type does not declare", timer);
        return;
    }
    sce_test_timer_timeout_taken(binding);
    sce_test_run_record_(run, context->name, SCE_TEST_LOG_TIMEOUT, "%s", timer);
    sce_test_trace_timer(&run->result.trace, run->now, SCE_TEST_TRACE_TIMER_TIMED_OUT, context->name, timer);
}

/** A done branch on the PTC `ptc`: nothing is removed (-4 §9.16); the component
 *  took a branch in this round. */
static inline void sce_test_context_done_taken(const sce_test_component_context_t *context, const char *ptc) {
    (void)ptc;
    sce_test_entity_t *entity = sce_test_context_entity_(context);
    if (entity != NULL) {
        entity->branch_taken = true;
    }
}

/* ── Statements of the profile ── */

/** `<sce:setverdict>` (P12): the local verdict becomes the worse of the two,
 *  and the verdict and its reason (NULL or empty for none) are logged. */
static inline void sce_test_context_setverdict(const sce_test_component_context_t *context, sce_test_verdict_t verdict,
                                               const char *reason) {
    sce_test_entity_t *entity = sce_test_context_entity_(context);
    if (entity == NULL) {
        return;
    }
    entity->verdict = sce_test_worse(entity->verdict, verdict);
    if (reason == NULL || reason[0] == '\0') {
        sce_test_run_record_(context->run, context->name, SCE_TEST_LOG_VERDICT, "%s", sce_test_verdict_name(verdict));
    } else {
        sce_test_run_record_(context->run, context->name, SCE_TEST_LOG_VERDICT, "%s %s", sce_test_verdict_name(verdict),
                             reason);
    }
    sce_test_trace_verdict(&context->run->result.trace, context->run->now, context->name, verdict, reason);
}

/** `<sce:start-timer>` (P7), for `duration` microseconds from `NOW`; `timer` is
 *  lent for as long as the component lives. */
static inline void sce_test_context_start_timer(const sce_test_component_context_t *context, const char *timer,
                                                sce_test_duration_t duration) {
    sce_test_entity_t *entity = sce_test_context_entity_(context);
    if (entity == NULL) {
        return;
    }
    sce_test_timer_t *binding = sce_test_entity_timer_(entity, timer, true);
    if (binding == NULL) {
        sce_test_run_fail_(context->run, context->name,
                           "starts the timer '%s', past the timers a component holds (%u, SCE_TEST_MAX_TIMERS)", timer,
                           (unsigned)SCE_TEST_MAX_TIMERS);
        return;
    }
    sce_test_timer_start(binding, context->run->now, duration);
    sce_test_trace_timer_started(&context->run->result.trace, context->run->now, context->name, timer, duration);
}

/** `<sce:stop-timer>` (P7). A timer never started is idle, and stays so. The
 *  test log records the stop of a running timer; the stop of one that is not
 *  running changes nothing. */
static inline void sce_test_context_stop_timer(const sce_test_component_context_t *context, const char *timer) {
    sce_test_entity_t *entity = sce_test_context_entity_(context);
    if (entity == NULL) {
        return;
    }
    sce_test_timer_t *binding = sce_test_entity_timer_(entity, timer, false);
    if (binding == NULL || binding->status != SCE_TEST_TIMER_RUNNING) {
        return;
    }
    sce_test_timer_stop(binding);
    sce_test_trace_timer(&context->run->result.trace, context->run->now, SCE_TEST_TRACE_TIMER_STOPPED, context->name,
                         timer);
}

/** A `<log>` (P8): recorded in the test log with `NOW` and the component. */
static inline void sce_test_context_log(const sce_test_component_context_t *context, const char *text) {
    if (sce_test_context_entity_(context) != NULL) {
        sce_test_run_record_(context->run, context->name, SCE_TEST_LOG_LOG, "%s", text);
        sce_test_trace_log(&context->run->result.trace, context->run->now, context->name, text);
    }
}

/* ── Doors (§12.3) ── */

/** A send on `port` (P6) of the record `event` whose value is `value`, which
 *  names the record type the test log writes it by: to the queue of the
 *  connected port, or to the system adapter when the port is mapped. The value
 *  is copied before this returns. */
static inline void sce_test_context_send(const sce_test_component_context_t *context, const char *port,
                                         const char *event, sce_test_message_value_t value) {
    if (sce_test_context_entity_(context) != NULL) {
        sce_test_run_send_(context->run, context->name, port, event, value);
    }
}

/** The start of the invocation that creates the PTC `ptc` (P9): the PTC is
 *  created and its machine started before this returns. */
static inline void sce_test_context_create(const sce_test_component_context_t *context, const char *ptc) {
    if (sce_test_context_entity_(context) != NULL) {
        sce_test_run_create_(context->run, context->name, ptc);
    }
}

/** The cancel of that invocation: the PTC is removed (P9). */
static inline void sce_test_context_kill(const sce_test_component_context_t *context, const char *ptc) {
    if (sce_test_context_entity_(context) != NULL) {
        sce_test_run_kill_(context->run, context->name, ptc);
    }
}

/* ── Module parameters (§11.5, §11.8) ── */

/**
 * A dynamic error the component's machine met where nothing in the run's
 * doors reports it (P13): the run ends with `error`, `why` in its log under
 * the component's name. What a generated component reports of a start its
 * machine refused, such as a session id its name gives that the build cannot
 * hold, so the machine is not run under an id made up in its place.
 */
static inline void sce_test_context_error(const sce_test_component_context_t *context, const char *why) {
    if (sce_test_context_entity_(context) == NULL) {
        return;
    }
    sce_test_run_fail_(context->run, context->name, "%s", why);
}

/**
 * Reads the module parameters of the run into the variables the component
 * declares as module parameters, before its machine starts: `read` is handed
 * them and `user_data`, and answers whether it read them. A value the run was
 * not given (`sce_test_module_parameters_of`), or one `read` refuses as not of
 * the type the component declares, is a dynamic error (P13): the run ends with
 * `error`, the refusal's words in its log, and false is returned, so the machine
 * is not started on a value made up in its place.
 */
typedef bool (*sce_test_module_parameters_read_fn)(void *user_data, const sce_test_module_parameters_t *given,
                                                   sce_saved_error_t *err);

static inline bool sce_test_context_read_module_parameters(const sce_test_component_context_t *context,
                                                           sce_test_module_parameters_read_fn read, void *user_data) {
    if (sce_test_context_entity_(context) == NULL) {
        return false;
    }
    sce_saved_error_t err;
    err.message[0] = '\0';
    if (read(user_data, &context->run->module_parameters, &err)) {
        return true;
    }
    sce_test_run_fail_(context->run, context->name, "%s", err.message);
    return false;
}

/**
 * Whether a receive branch with a template matches the snapshot of `context`
 * (P2, docs/SCE_TEST_PROFILE.md §12.3).
 *
 * The snapshot saw a message at the head of `port`, under `event`, the event
 * name of the template's record, whose value is of the C type named `type`,
 * `size` bytes long, and `matches`, the template's `match` bound to its
 * arguments through `closure`, holds of that value. A message of another record,
 * or one whose value is of another type, does not match. Nothing is removed.
 */
typedef bool (*sce_test_matches_fn)(const void *value, const void *closure);

static inline bool sce_test_receives(const sce_test_component_context_t *context, const char *port, const char *event,
                                     const char *type, size_t size, sce_test_matches_fn matches, const void *closure) {
    const sce_test_message_t *head = sce_test_context_snapshot_of(context, port);
    if (head == NULL || strcmp(head->event, event) != 0) {
        return false;
    }
    const void *value = sce_test_message_value_as(head, type, size);
    return value != NULL && matches(value, closure);
}

/** `sce_test_receives` of a template whose record is of the C type `Type`. */
#define SCE_TEST_RECEIVES(context, port, event, Type, matches, closure)                                                \
    sce_test_receives((context), (port), (event), #Type, sizeof(Type), (matches), (closure))

#ifdef __cplusplus
}
#endif

#endif /* SCE_TEST_TEST_CASE_RUN_H */
