/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/**
 * @file test_log.h
 * @brief The test log of a test case run (docs/SCE_TEST_PROFILE.md §12.5;
 *        docs/adr/0091, C3 and answer 8), and its one canonical text,
 *        `schemas/sce-test-log.v1.schema.json`
 *
 * The records of TCI's test logging interface (ETSI ES 201 873-6), each
 * written where the profile's semantics places its event, in the order the
 * round scheduler produces them: the run's trace, beside the lines of its log,
 * which are for a reader (sce/test/test_case_run.h). The text is the JSON
 * Canonicalization Scheme of the log (`sce_jcs_t`, sce/saved_json.h), and it
 * holds no number: an instant, a duration and every integer of a value are
 * texts of their digits (sce/test/canonical.h). So two runs agree, under C3,
 * exactly when their logs are the same bytes, whichever runtime wrote each. The
 * C11 port of `sce/include/test/TestLog.h` and
 * `backends/rust/runtime/src/test/test_log.rs`, record for record.
 *
 * What is C's own: nothing is allocated. A trace holds at most
 * `SCE_TEST_MAX_TRACE_RECORDS` records, and the texts they hold that the run
 * does not lend for its whole length (a port's or a timer's name, a component
 * type, a reason, a `<log>`'s text, and each message's value in its canonical
 * text, written as the message is sent or received) in one buffer of
 * `SCE_TEST_TRACE_TEXT_SIZE` bytes. A record that does not fit is counted in
 * `lost`, never cut: a run that lost one ends with `error`, and its log is not
 * written, since it no longer says what happened. A component's name is lent by
 * the configuration, and a record type's name is of static storage.
 * Header-only and allocation-free.
 */

#ifndef SCE_TEST_TEST_LOG_H
#define SCE_TEST_TEST_LOG_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include "sce/saved_json.h"
#include "sce/test/canonical.h"
#include "sce/test/port.h"
#include "sce/test/verdict.h"

#ifdef __cplusplus
extern "C" {
#endif

/* How many records the test log of one run holds. The run of a profile's
   example writes about a dozen. */
#ifndef SCE_TEST_MAX_TRACE_RECORDS
#define SCE_TEST_MAX_TRACE_RECORDS 64u
#endif

/* How many bytes the texts of the test log of one run take, over all its
   records: names, reasons, the texts of `<log>`s and the canonical texts of
   the values sent and received. */
#ifndef SCE_TEST_TRACE_TEXT_SIZE
#define SCE_TEST_TRACE_TEXT_SIZE 2048u
#endif

/** The name of the test log's shape, its `schema`. */
#define SCE_TEST_LOG_SCHEMA "sce-test-log.v1"

/** The stability status of the test log's wire schema, the schema file's own
 *  `x-sce-schema-status`, which `test_case_run_test.c` holds this to.
 *  `SCE_WIRE_CONTRACTS.md` requires one commit to move both. */
#define SCE_TEST_LOG_SCHEMA_STATUS "pre-release"

/** The events of TCI's test logging interface (ES 201 873-6), as the profile's
 *  semantics places them (docs/SCE_TEST_PROFILE.md §12.5). */
typedef enum sce_test_trace_kind_e {
    /** The test case started, at `NOW = 0` (P11). */
    SCE_TEST_TRACE_TEST_CASE_STARTED = 0,
    /** The test case ended with its verdict (P14). */
    SCE_TEST_TRACE_TEST_CASE_ENDED,
    /** A component was created: the MTC as the test case starts, a PTC by the
        invocation that creates it (P9). */
    SCE_TEST_TRACE_COMPONENT_CREATED,
    /** A component terminated, with its local verdict (P10). */
    SCE_TEST_TRACE_COMPONENT_DONE,
    /** A component sent a message on a port (P6). */
    SCE_TEST_TRACE_MESSAGE_SENT,
    /** A receive branch took the head of a port's queue: the message was
        received and matched (P4 step 1). */
    SCE_TEST_TRACE_MESSAGE_RECEIVED,
    /** A component started a timer, for a duration in microseconds (P7). */
    SCE_TEST_TRACE_TIMER_STARTED,
    /** A component stopped a running timer (P7). */
    SCE_TEST_TRACE_TIMER_STOPPED,
    /** A timeout branch took a timer that had timed out (P4 step 1). */
    SCE_TEST_TRACE_TIMER_TIMED_OUT,
    /** A component's local verdict was set: by `<sce:setverdict>` (P12), or to
        `error` by the runtime (P13). */
    SCE_TEST_TRACE_VERDICT_SET,
    /** A `<log>` (P8). */
    SCE_TEST_TRACE_LOG
} sce_test_trace_kind_t;

/** The record's `kind` in the test log: the TCI event in kebab case. */
static inline const char *sce_test_trace_kind_name(sce_test_trace_kind_t kind) {
    switch (kind) {
    case SCE_TEST_TRACE_TEST_CASE_STARTED:
        return "test-case-started";
    case SCE_TEST_TRACE_TEST_CASE_ENDED:
        return "test-case-ended";
    case SCE_TEST_TRACE_COMPONENT_CREATED:
        return "component-created";
    case SCE_TEST_TRACE_COMPONENT_DONE:
        return "component-done";
    case SCE_TEST_TRACE_MESSAGE_SENT:
        return "message-sent";
    case SCE_TEST_TRACE_MESSAGE_RECEIVED:
        return "message-received";
    case SCE_TEST_TRACE_TIMER_STARTED:
        return "timer-started";
    case SCE_TEST_TRACE_TIMER_STOPPED:
        return "timer-stopped";
    case SCE_TEST_TRACE_TIMER_TIMED_OUT:
        return "timer-timed-out";
    case SCE_TEST_TRACE_VERDICT_SET:
        return "verdict-set";
    case SCE_TEST_TRACE_LOG:
        return "log";
    }
    return "log";
}

/** A text a record holds: `len` bytes from `at` of its trace's texts. */
typedef struct sce_test_trace_text_s {
    size_t at;
    size_t len;
} sce_test_trace_text_t;

/**
 * One record of the test log: `NOW` when it happened, what happened, and the
 * members of its kind.
 */
typedef struct sce_test_trace_record_s {
    /** `NOW` when it was recorded. */
    sce_test_instant_t at;
    sce_test_trace_kind_t kind;
    /** The component it is of, a name the configuration lends; NULL for the
        start and the end of the test case. */
    const char *component;
    /** The verdict of an end, a component done or a verdict set. */
    sce_test_verdict_t verdict;
    /** The duration of a timer started, in microseconds. */
    sce_test_duration_t duration;
    /** The record type's name of a message sent or received, of static
        storage. */
    const char *message_type;
    /** The component type of a creation, the port of a message, or the timer. */
    sce_test_trace_text_t name;
    /** The value of a message in its canonical text, the reason of a verdict
        set where it has one, or the text of a `<log>`. */
    sce_test_trace_text_t text;
    /** Whether `text` is given: a verdict set with no reason has none. */
    bool has_text;
} sce_test_trace_record_t;

/** The test log of one run: its records in the order they were written, the
 *  texts they hold, and how many records it could not hold. */
typedef struct sce_test_trace_s {
    sce_test_trace_record_t records[SCE_TEST_MAX_TRACE_RECORDS];
    size_t count;
    /** The records the trace could not hold: past
        `SCE_TEST_MAX_TRACE_RECORDS`, a text past `SCE_TEST_TRACE_TEXT_SIZE`, or
        a value its record type does not write. A run that lost one ends with
        `error`, and its log is not written. */
    size_t lost;
    char text[SCE_TEST_TRACE_TEXT_SIZE];
    size_t text_used;
} sce_test_trace_t;

/* ── Recording ─────────────────────────────────────────────────────────── */

/* The next record, of `kind` at `at` for `component`, its members still to be
   given; NULL, the record counted lost, when the trace holds no more. It is kept
   by `sce_test_trace_keep_`. */
static inline sce_test_trace_record_t *sce_test_trace_next_(sce_test_trace_t *trace, sce_test_instant_t at,
                                                            sce_test_trace_kind_t kind, const char *component) {
    if (trace->count >= SCE_TEST_MAX_TRACE_RECORDS) {
        trace->lost++;
        return NULL;
    }
    sce_test_trace_record_t *record = &trace->records[trace->count];
    memset(record, 0, sizeof(*record));
    record->at = at;
    record->kind = kind;
    record->component = component;
    return record;
}

static inline void sce_test_trace_keep_(sce_test_trace_t *trace) {
    trace->count++;
}

/* The record being written did not fit: the texts it took are given back, and
   it is counted lost. */
static inline void sce_test_trace_lose_(sce_test_trace_t *trace, size_t text_used) {
    trace->text_used = text_used;
    trace->lost++;
}

/* A copy of `s` among the trace's texts, in `*out`; false when it does not fit. */
static inline bool sce_test_trace_copy_(sce_test_trace_t *trace, const char *s, sce_test_trace_text_t *out) {
    const size_t n = strlen(s);
    if (n > SCE_TEST_TRACE_TEXT_SIZE - trace->text_used) {
        return false;
    }
    memcpy(trace->text + trace->text_used, s, n);
    out->at = trace->text_used;
    out->len = n;
    trace->text_used += n;
    return true;
}

/* The canonical text of the value `message` carries, written among the trace's
   texts by its record type, in `*out`; false when the message names no record
   type, its value has no canonical form, or the text does not fit. */
static inline bool sce_test_trace_value_(sce_test_trace_t *trace, const sce_test_message_t *message,
                                         sce_test_trace_text_t *out) {
    if (!sce_test_record_type_complete(message->record_type)) {
        return false;
    }
    sce_jcs_t jcs;
    sce_jcs_init(&jcs, trace->text + trace->text_used, SCE_TEST_TRACE_TEXT_SIZE - trace->text_used);
    message->record_type->canonical(&jcs, message->value.bytes);
    if (!sce_jcs_finish(&jcs)) {
        return false;
    }
    out->at = trace->text_used;
    out->len = jcs.w.len;
    trace->text_used += jcs.w.len;
    return true;
}

/** The test case started (P11). */
static inline void sce_test_trace_started(sce_test_trace_t *trace, sce_test_instant_t at) {
    if (sce_test_trace_next_(trace, at, SCE_TEST_TRACE_TEST_CASE_STARTED, NULL) != NULL) {
        sce_test_trace_keep_(trace);
    }
}

/** The test case ended with `verdict` (P14). */
static inline void sce_test_trace_ended(sce_test_trace_t *trace, sce_test_instant_t at, sce_test_verdict_t verdict) {
    sce_test_trace_record_t *record = sce_test_trace_next_(trace, at, SCE_TEST_TRACE_TEST_CASE_ENDED, NULL);
    if (record != NULL) {
        record->verdict = verdict;
        sce_test_trace_keep_(trace);
    }
}

/** `component` was created, of the type `component_type` (P9). */
static inline void sce_test_trace_created(sce_test_trace_t *trace, sce_test_instant_t at, const char *component,
                                          const char *component_type) {
    const size_t used = trace->text_used;
    sce_test_trace_record_t *record = sce_test_trace_next_(trace, at, SCE_TEST_TRACE_COMPONENT_CREATED, component);
    if (record == NULL) {
        return;
    }
    if (!sce_test_trace_copy_(trace, component_type, &record->name)) {
        sce_test_trace_lose_(trace, used);
        return;
    }
    sce_test_trace_keep_(trace);
}

/** `component` terminated, its local verdict `verdict` (P10). */
static inline void sce_test_trace_done(sce_test_trace_t *trace, sce_test_instant_t at, const char *component,
                                       sce_test_verdict_t verdict) {
    sce_test_trace_record_t *record = sce_test_trace_next_(trace, at, SCE_TEST_TRACE_COMPONENT_DONE, component);
    if (record != NULL) {
        record->verdict = verdict;
        sce_test_trace_keep_(trace);
    }
}

/** `component` sent `message` on `port` (P6, `SCE_TEST_TRACE_MESSAGE_SENT`), or
 *  received it from `port` (P4, `SCE_TEST_TRACE_MESSAGE_RECEIVED`): its record
 *  type's name and its value in its canonical text. */
static inline void sce_test_trace_message(sce_test_trace_t *trace, sce_test_instant_t at, sce_test_trace_kind_t kind,
                                          const char *component, const char *port, const sce_test_message_t *message) {
    const size_t used = trace->text_used;
    sce_test_trace_record_t *record = sce_test_trace_next_(trace, at, kind, component);
    if (record == NULL) {
        return;
    }
    if (!sce_test_trace_copy_(trace, port, &record->name) || !sce_test_trace_value_(trace, message, &record->text)) {
        sce_test_trace_lose_(trace, used);
        return;
    }
    record->message_type = message->record_type->name;
    record->has_text = true;
    sce_test_trace_keep_(trace);
}

/** `component` started `timer` for `duration` microseconds (P7). */
static inline void sce_test_trace_timer_started(sce_test_trace_t *trace, sce_test_instant_t at, const char *component,
                                                const char *timer, sce_test_duration_t duration) {
    const size_t used = trace->text_used;
    sce_test_trace_record_t *record = sce_test_trace_next_(trace, at, SCE_TEST_TRACE_TIMER_STARTED, component);
    if (record == NULL) {
        return;
    }
    if (!sce_test_trace_copy_(trace, timer, &record->name)) {
        sce_test_trace_lose_(trace, used);
        return;
    }
    record->duration = duration;
    sce_test_trace_keep_(trace);
}

/** `component` stopped the running `timer` (P7, `SCE_TEST_TRACE_TIMER_STOPPED`),
 *  or a timeout branch took it (P4, `SCE_TEST_TRACE_TIMER_TIMED_OUT`). */
static inline void sce_test_trace_timer(sce_test_trace_t *trace, sce_test_instant_t at, sce_test_trace_kind_t kind,
                                        const char *component, const char *timer) {
    const size_t used = trace->text_used;
    sce_test_trace_record_t *record = sce_test_trace_next_(trace, at, kind, component);
    if (record == NULL) {
        return;
    }
    if (!sce_test_trace_copy_(trace, timer, &record->name)) {
        sce_test_trace_lose_(trace, used);
        return;
    }
    sce_test_trace_keep_(trace);
}

/** `component`'s local verdict was set to `verdict` (P12, P13), for `reason`:
 *  the document's, none where it is NULL or empty, and none for an error the
 *  runtime sets, whose cause is the runtime's own prose, which another runtime
 *  would not write alike. */
static inline void sce_test_trace_verdict(sce_test_trace_t *trace, sce_test_instant_t at, const char *component,
                                          sce_test_verdict_t verdict, const char *reason) {
    const size_t used = trace->text_used;
    sce_test_trace_record_t *record = sce_test_trace_next_(trace, at, SCE_TEST_TRACE_VERDICT_SET, component);
    if (record == NULL) {
        return;
    }
    record->verdict = verdict;
    if (reason != NULL && reason[0] != '\0') {
        if (!sce_test_trace_copy_(trace, reason, &record->text)) {
            sce_test_trace_lose_(trace, used);
            return;
        }
        record->has_text = true;
    }
    sce_test_trace_keep_(trace);
}

/** `component`'s `<log>` of `text` (P8). */
static inline void sce_test_trace_log(sce_test_trace_t *trace, sce_test_instant_t at, const char *component,
                                      const char *text) {
    const size_t used = trace->text_used;
    sce_test_trace_record_t *record = sce_test_trace_next_(trace, at, SCE_TEST_TRACE_LOG, component);
    if (record == NULL) {
        return;
    }
    if (!sce_test_trace_copy_(trace, text, &record->text)) {
        sce_test_trace_lose_(trace, used);
        return;
    }
    record->has_text = true;
    sce_test_trace_keep_(trace);
}

/* ── Writing ───────────────────────────────────────────────────────────── */

/* The member `name` of the record being written: the text `text` of `trace`. */
static inline void sce_test_trace_member_(sce_jcs_t *jcs, const char *name, const sce_test_trace_t *trace,
                                          sce_test_trace_text_t text) {
    sce_jcs_key(jcs, name);
    sce_jcs_string_n(jcs, trace->text + text.at, text.len);
}

/** `record` as the test log writes it: `at`, `kind`, and the members of its
 *  kind, an absent reason omitted. */
static inline void sce_test_trace_write_record(sce_jcs_t *jcs, const sce_test_trace_t *trace,
                                               const sce_test_trace_record_t *record) {
    sce_jcs_begin_object(jcs);
    sce_jcs_key(jcs, "at");
    sce_test_canonical_signed(jcs, record->at);
    sce_jcs_key(jcs, "kind");
    sce_jcs_string(jcs, sce_test_trace_kind_name(record->kind));
    if (record->component != NULL) {
        sce_jcs_key(jcs, "component");
        sce_jcs_string(jcs, record->component);
    }
    switch (record->kind) {
    case SCE_TEST_TRACE_TEST_CASE_STARTED:
        break;
    case SCE_TEST_TRACE_TEST_CASE_ENDED:
    case SCE_TEST_TRACE_COMPONENT_DONE:
        sce_jcs_key(jcs, "verdict");
        sce_jcs_string(jcs, sce_test_verdict_name(record->verdict));
        break;
    case SCE_TEST_TRACE_COMPONENT_CREATED:
        sce_test_trace_member_(jcs, "type", trace, record->name);
        break;
    case SCE_TEST_TRACE_MESSAGE_SENT:
    case SCE_TEST_TRACE_MESSAGE_RECEIVED:
        sce_test_trace_member_(jcs, "port", trace, record->name);
        sce_jcs_key(jcs, "type");
        sce_jcs_string(jcs, record->message_type != NULL ? record->message_type : "");
        sce_jcs_key(jcs, "value");
        sce_jcs_written(jcs, trace->text + record->text.at, record->text.len);
        break;
    case SCE_TEST_TRACE_TIMER_STARTED:
        sce_test_trace_member_(jcs, "timer", trace, record->name);
        sce_jcs_key(jcs, "duration_us");
        sce_test_canonical_signed(jcs, record->duration);
        break;
    case SCE_TEST_TRACE_TIMER_STOPPED:
    case SCE_TEST_TRACE_TIMER_TIMED_OUT:
        sce_test_trace_member_(jcs, "timer", trace, record->name);
        break;
    case SCE_TEST_TRACE_VERDICT_SET:
        sce_jcs_key(jcs, "verdict");
        sce_jcs_string(jcs, sce_test_verdict_name(record->verdict));
        if (record->has_text) {
            sce_test_trace_member_(jcs, "reason", trace, record->text);
        }
        break;
    case SCE_TEST_TRACE_LOG:
        sce_test_trace_member_(jcs, "text", trace, record->text);
        break;
    }
    sce_jcs_end_object(jcs);
}

/** Whether a test log was written, or why not. */
typedef enum sce_test_trace_written_e {
    /** The whole text, terminated, is in the buffer. */
    SCE_TEST_TRACE_WRITTEN = 0,
    /** The buffer holds less than the text and its terminator: `*len` bytes and
        one more are needed. A call with no buffer measures the text so. */
    SCE_TEST_TRACE_TOO_SMALL,
    /** The run lost a record (`sce_test_trace_t::lost`), so the log would not
        say what happened. */
    SCE_TEST_TRACE_INCOMPLETE,
    /** A value of the log is not in the scheme: an object that names one member
        twice, which a value's record type wrote. */
    SCE_TEST_TRACE_NOT_CANONICAL
} sce_test_trace_written_t;

/**
 * The test log of one run of the test case `test_case` of the suite `suite`,
 * whose verdict is `verdict` and whose records `*trace` holds, in its one
 * canonical text, into the `cap` bytes at `buf` (NULL to measure); `*len`, where
 * `len` is not NULL, the length of the text, whether or not it fit.
 *
 * The JSON Canonicalization Scheme of `schema`, `suite`, `test_case`, `verdict`
 * and `records`, each record in the order the run produced it.
 */
static inline sce_test_trace_written_t sce_test_trace_write(const sce_test_trace_t *trace, const char *suite,
                                                            const char *test_case, sce_test_verdict_t verdict,
                                                            char *buf, size_t cap, size_t *len) {
    if (len != NULL) {
        *len = 0u;
    }
    if (trace->lost > 0u) {
        return SCE_TEST_TRACE_INCOMPLETE;
    }
    sce_jcs_t jcs;
    sce_jcs_init(&jcs, buf, cap);
    sce_jcs_begin_object(&jcs);
    sce_jcs_key(&jcs, "records");
    sce_jcs_begin_array(&jcs);
    for (size_t i = 0u; i < trace->count; ++i) {
        sce_test_trace_write_record(&jcs, trace, &trace->records[i]);
    }
    sce_jcs_end_array(&jcs);
    sce_jcs_key(&jcs, "schema");
    sce_jcs_string(&jcs, SCE_TEST_LOG_SCHEMA);
    sce_jcs_key(&jcs, "suite");
    sce_jcs_string(&jcs, suite);
    sce_jcs_key(&jcs, "test_case");
    sce_jcs_string(&jcs, test_case);
    sce_jcs_key(&jcs, "verdict");
    sce_jcs_string(&jcs, sce_test_verdict_name(verdict));
    sce_jcs_end_object(&jcs);
    const bool whole = sce_jcs_finish(&jcs);
    if (len != NULL) {
        *len = jcs.w.len;
    }
    if (jcs.refusal != NULL) {
        return SCE_TEST_TRACE_NOT_CANONICAL;
    }
    return whole ? SCE_TEST_TRACE_WRITTEN : SCE_TEST_TRACE_TOO_SMALL;
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_TEST_TEST_LOG_H */
