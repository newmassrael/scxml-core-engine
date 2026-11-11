/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/**
 * @file input_trace.h
 * @brief The recorded input trace of a test case and its replay
 *        (docs/SCE_TTCN3_LANE.md §10; docs/SCE_TEST_PROFILE.md §3.2, §3.3,
 *        §12.4): the inputs of one run from the system under test, each at its
 *        logical instant, `schemas/sce-test-trace.v1.schema.json`, and the
 *        system adapter that plays them
 *
 * A trace is one text of the JSON Canonicalization Scheme, so that it has one
 * spelling and two traces are equal exactly when their bytes are:
 * `{"inputs":[{"at","port","type","value"}, ...],"schema","suite","test_case"}`.
 * `at` is `NOW` in microseconds as the text of its digits, `port` a system port
 * of the test case's configuration, and `type` and `value` the record type's
 * name and the value in the canonical form a message record of the test log
 * writes (§12.5). The inputs stand in order of `at`; the inputs of one instant in
 * the order they are appended to their ports' queues (§3.2 step 2).
 *
 * The value of an input is read by its type: the reader of the test case's
 * configuration (`sce_test_input_reader_fn`, generated as `<configuration>_input`
 * in the configuration's unit) knows which records each system port takes and
 * reads each with the reader of its record (sce/test/from_canonical.h). A trace
 * that does not fit is refused whole before the run starts, which then ends with
 * `error` (P13). Every refusal is a sentence in a `sce_saved_error_t`, in the
 * words of the Rust reader `sce_rust_runtime::test::trace`, which is the
 * authoritative one, word for word. The C11 port of `sce/include/test/Trace.h`
 * and `sce/include/test/TraceRefusal.h`.
 *
 * What is C's own, against its siblings:
 *
 *   - The names. This runtime's test log is `sce_test_trace_t` (sce/test/test_log.h),
 *     TCI's trace of a run, so the trace a run replays is the input trace,
 *     `sce_test_input_trace_t`, and every name of this header says so.
 *   - Nothing is allocated. A trace holds at most
 *     `SCE_TEST_MAX_INPUT_TRACE_INPUTS` inputs, each value a span of the text it
 *     was read from, which the caller keeps until the trace is played; each name
 *     in a buffer of `SCE_MAX_ID_LEN` bytes; and a text of at most
 *     `SCE_TEST_INPUT_TRACE_TEXT_SIZE` bytes, which it writes again in its
 *     canonical text to hold the text read to it. The player holds the message
 *     of each input, read before the run, and what the components send to the
 *     system ports, up to `SCE_TEST_MAX_INPUT_TRACE_OUTPUTS`. A trace past one of
 *     these is refused by name, never cut.
 *   - A number in a value, which no canonical value holds but the scheme writes,
 *     is held to the spelling ECMAScript gives the double it reads as
 *     (`sce_number_text`, sce/number_text.h), read by `strtod` in the C locale a
 *     program starts in; one past the range of a double has no canonical text.
 *
 * Header-only and allocation-free.
 */

#ifndef SCE_TEST_INPUT_TRACE_H
#define SCE_TEST_INPUT_TRACE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sce/number_text.h"
#include "sce/saved_json.h"
#include "sce/test/canonical.h"
#include "sce/test/from_canonical.h"
#include "sce/test/test_case_run.h"
#include "sce/types.h"

#ifdef __cplusplus
extern "C" {
#endif

/** The name of the trace's shape, its `schema`. */
#define SCE_TEST_INPUT_TRACE_SCHEMA "sce-test-trace.v1"

/** The stability status of the trace's wire schema, the schema file's own
 *  `x-sce-schema-status`, which `input_trace_test.c` holds this to.
 *  `SCE_WIRE_CONTRACTS.md` requires one commit to move both, and the Rust
 *  runtime's `TEST_TRACE_SCHEMA_STATUS` with them. */
#define SCE_TEST_INPUT_TRACE_SCHEMA_STATUS "pre-release"

/* How many inputs one trace holds. The profile's traces hold a handful; a run
   that is fed more raises it (`-DSCE_TEST_MAX_INPUT_TRACE_INPUTS=N`). */
#ifndef SCE_TEST_MAX_INPUT_TRACE_INPUTS
#define SCE_TEST_MAX_INPUT_TRACE_INPUTS 32u
#endif

/* How many bytes the text of one trace takes, a line feed after it aside. */
#ifndef SCE_TEST_INPUT_TRACE_TEXT_SIZE
#define SCE_TEST_INPUT_TRACE_TEXT_SIZE 8192u
#endif

/* How many messages to the system ports the player of one trace records. One
   past it is counted in `sent_lost`, never recorded in another's place. */
#ifndef SCE_TEST_MAX_INPUT_TRACE_OUTPUTS
#define SCE_TEST_MAX_INPUT_TRACE_OUTPUTS 16u
#endif

/**
 * The reader of the inputs of a test case's configuration: the message an input
 * of the record type `type_name` on the system port `system_port` carries, its
 * value read from the canonical form `value`, into `*message`; false, naming why
 * in `*err`, where it is not one.
 *
 * Generated as `<configuration>_input` in the unit of a test configuration,
 * which names the records each of its system ports takes. NULL in a test case is
 * the reader of a configuration with no system port, which refuses every input.
 */
typedef bool (*sce_test_input_reader_fn)(const char *system_port, const char *type_name, sce_jv_t value,
                                         sce_test_message_t *message, sce_saved_error_t *err);

/**
 * The refusal of an input of the record type `type_name` on `system_port`, which
 * the configuration whose system ports are the `count` names `system_ports` does
 * not take: the port is none of them, or its type's `in` list does not name the
 * type (docs/SCE_TTCN3_LANE.md §10.2). Always false.
 */
static inline bool sce_test_not_an_input(const char *system_port, const char *type_name,
                                         const char *const *system_ports, size_t count, sce_saved_error_t *err) {
    for (size_t i = 0u; i < count; ++i) {
        if (strcmp(system_ports[i], system_port) == 0) {
            return sce_saved_refuse(err, "'%s' is not on the `in` list of the type of the system port '%s'", type_name,
                                    system_port);
        }
    }
    return sce_saved_refuse(err, "'%s' is not a system port of the test case's configuration", system_port);
}

/**
 * The message of the record `event` whose value is `value` in `*message`, as an
 * input a trace hands the run: `sce_test_message_set`, with a bound it meets
 * refused by name.
 */
static inline bool sce_test_input_message(sce_test_message_t *message, const char *event,
                                          sce_test_message_value_t value, sce_saved_error_t *err) {
    switch (sce_test_message_set(message, event, value)) {
    case SCE_TEST_MESSAGE_HELD:
        return true;
    case SCE_TEST_MESSAGE_EVENT_TOO_LONG:
        return sce_saved_refuse(err, "the event '%s' is longer than SCE_MAX_ID_LEN (%u) holds", event,
                                (unsigned)SCE_MAX_ID_LEN);
    case SCE_TEST_MESSAGE_VALUE_TOO_LARGE:
        return sce_saved_refuse(err, "a value of %lu bytes is past SCE_TEST_MAX_VALUE_SIZE (%u)",
                                (unsigned long)value.value.size, (unsigned)SCE_TEST_MAX_VALUE_SIZE);
    case SCE_TEST_MESSAGE_UNTYPED:
        break;
    }
    return sce_saved_refuse(err, "the value names no record type the test log names and writes it by");
}

/** One input of a trace. */
typedef struct sce_test_input_trace_entry_s {
    /** `NOW` when it is due. */
    sce_test_instant_t at;
    /** The system port it arrives on. */
    char port[SCE_MAX_ID_LEN];
    /** The name of its record type, as the test log names it. */
    char type_name[SCE_MAX_ID_LEN];
    /** Its value, in the canonical form of the test log: a span of the text the
        trace was read from. */
    sce_jv_t value;
} sce_test_input_trace_entry_t;

/** A recorded input trace of one test case of one suite. */
typedef struct sce_test_input_trace_s {
    /** The suite's name. */
    char suite[SCE_MAX_ID_LEN];
    /** The test case's name. */
    char test_case[SCE_MAX_ID_LEN];
    /** The inputs, in order of `at`, those of one instant in the order they are
        appended to their ports. */
    sce_test_input_trace_entry_t inputs[SCE_TEST_MAX_INPUT_TRACE_INPUTS];
    size_t count;
    /** Where the reader writes the trace again in its canonical text, to hold the
        text it read to it. */
    char canonical[SCE_TEST_INPUT_TRACE_TEXT_SIZE];
} sce_test_input_trace_t;

/* `value`, a value of a judged text, written as the JSON Canonicalization Scheme
   writes it: a text re-escaped from what it decodes to, each object's members
   sorted by the writer, and a number in ECMAScript's spelling of the double it
   reads as. A number past the range of a double has no such spelling, which
   refuses the text. */
static inline void sce_test_input_trace_value_(sce_jcs_t *jcs, sce_jv_t value) {
    switch (sce_jv_kind(value)) {
    case SCE_JSON_NULL:
        sce_jcs_null(jcs);
        return;
    case SCE_JSON_BOOL:
        sce_jcs_bool(jcs, sce_jv_as_bool(value));
        return;
    case SCE_JSON_NUMBER: {
        char spelled[64];
        char shortest[64];
        const size_t n = sce_jv_number_len(value);
        if (n >= sizeof(spelled)) {
            /* Longer than the longest shortest spelling of a double. */
            sce_jcs_refuse(jcs, "a number is not spelled as the scheme spells it");
            return;
        }
        memcpy(spelled, value.p, n);
        spelled[n] = '\0';
        const double real = strtod(spelled, NULL);
        if (real != real || real - real != 0.0) {
            sce_jcs_refuse(jcs, "a number past the range of a double has no canonical text");
            return;
        }
        const size_t written = sce_number_text(real, shortest, sizeof(shortest));
        sce_jcs_written(jcs, shortest, written);
        return;
    }
    case SCE_JSON_STRING: {
        sce_jstr_t text;
        unsigned char byte = 0u;
        sce_jcs_place_(jcs);
        sce_jw_put_(&jcs->w, "\"", 1u);
        sce_jstr_init(&text, value);
        while (sce_jstr_next(&text, &byte)) {
            const char one = (char)byte;
            sce_jw_escaped_(&jcs->w, &one, 1u);
        }
        sce_jw_put_(&jcs->w, "\"", 1u);
        return;
    }
    case SCE_JSON_ARRAY: {
        sce_jiter_t elements;
        sce_jv_t element;
        sce_jcs_begin_array(jcs);
        (void)sce_jv_iter(value, &elements);
        while (sce_jiter_element(&elements, &element)) {
            sce_test_input_trace_value_(jcs, element);
        }
        sce_jcs_end_array(jcs);
        return;
    }
    case SCE_JSON_OBJECT: {
        sce_jiter_t members;
        sce_jv_t key;
        sce_jv_t member;
        /* The member's name, which the writer is lent until the member's value
           is placed, the first thing writing the value does. */
        char name[SCE_SAVED_ERROR_LEN];
        sce_jcs_begin_object(jcs);
        (void)sce_jv_iter(value, &members);
        while (sce_jiter_member(&members, &key, &member)) {
            size_t n = 0u;
            if (!sce_jv_string(key, name, sizeof(name), &n) || memchr(name, '\0', n) != NULL) {
                sce_jcs_refuse(jcs, "a member name is past what the reader of a trace holds");
                return;
            }
            sce_jcs_key(jcs, name);
            sce_test_input_trace_value_(jcs, member);
        }
        sce_jcs_end_object(jcs);
        return;
    }
    }
}

/**
 * `*trace` in its one canonical text, the JSON Canonicalization Scheme, with no
 * line feed after it, into the `cap` bytes at `buf` (NULL to measure); `*len`,
 * where `len` is not NULL, the length of the text whether or not it fit. True
 * when the whole text fit and is in the scheme: false where a value holds a
 * number past the range of a double, which no canonical value holds and
 * `sce_test_input_trace_parse` refuses.
 */
static inline bool sce_test_input_trace_write(const sce_test_input_trace_t *trace, char *buf, size_t cap, size_t *len) {
    sce_jcs_t jcs;
    sce_jcs_init(&jcs, buf, cap);
    sce_jcs_begin_object(&jcs);
    sce_jcs_key(&jcs, "inputs");
    sce_jcs_begin_array(&jcs);
    for (size_t i = 0u; i < trace->count; ++i) {
        const sce_test_input_trace_entry_t *input = &trace->inputs[i];
        sce_jcs_begin_object(&jcs);
        sce_jcs_key(&jcs, "at");
        sce_test_canonical_signed(&jcs, input->at);
        sce_jcs_key(&jcs, "port");
        sce_jcs_string(&jcs, input->port);
        sce_jcs_key(&jcs, "type");
        sce_jcs_string(&jcs, input->type_name);
        sce_jcs_key(&jcs, "value");
        sce_test_input_trace_value_(&jcs, input->value);
        sce_jcs_end_object(&jcs);
    }
    sce_jcs_end_array(&jcs);
    sce_jcs_key(&jcs, "schema");
    sce_jcs_string(&jcs, SCE_TEST_INPUT_TRACE_SCHEMA);
    sce_jcs_key(&jcs, "suite");
    sce_jcs_string(&jcs, trace->suite);
    sce_jcs_key(&jcs, "test_case");
    sce_jcs_string(&jcs, trace->test_case);
    sce_jcs_end_object(&jcs);
    const bool whole = sce_jcs_finish(&jcs);
    if (len != NULL) {
        *len = jcs.w.len;
    }
    return whole;
}

/* The members of `value`, an object whose members are the `count` names
   `names` exactly, each in `found` in the order of `names`; refused, naming
   `what`, otherwise. */
static inline bool sce_test_input_trace_members_(sce_jv_t value, const char *const *names, size_t count,
                                                 sce_jv_t *found, const char *what, sce_saved_error_t *err) {
    if (sce_jv_kind(value) != SCE_JSON_OBJECT) {
        return sce_saved_refuse(err, "%s is not an object", what);
    }
    sce_jiter_t members;
    sce_jv_t key;
    sce_jv_t member;
    (void)sce_jv_iter(value, &members);
    while (sce_jiter_member(&members, &key, &member)) {
        bool named = false;
        for (size_t i = 0u; i < count && !named; ++i) {
            named = sce_jv_string_is(key, names[i]);
        }
        if (!named) {
            char name[SCE_SAVED_ERROR_LEN];
            (void)sce_jv_string(key, name, sizeof(name), NULL);
            return sce_saved_refuse(err, "%s has no member '%s'", what, name);
        }
    }
    for (size_t i = 0u; i < count; ++i) {
        if (!sce_jv_get(value, names[i], &found[i])) {
            return sce_saved_refuse(err, "%s has no '%s'", what, names[i]);
        }
    }
    return true;
}

/* The text `value`, which is not empty, at `what`, into the `SCE_MAX_ID_LEN`
   bytes at `out`. */
static inline bool sce_test_input_trace_name_(sce_jv_t value, const char *what, char *out, sce_saved_error_t *err) {
    if (sce_jv_kind(value) != SCE_JSON_STRING || sce_jv_string_len(value) == 0u) {
        return sce_saved_refuse(err, "%s is not a name", what);
    }
    size_t n = 0u;
    if (!sce_jv_string(value, out, SCE_MAX_ID_LEN, &n) || memchr(out, '\0', n) != NULL) {
        return sce_saved_refuse(err, "%s is past the name of at most %u bytes a trace holds (SCE_MAX_ID_LEN)", what,
                                (unsigned)(SCE_MAX_ID_LEN - 1));
    }
    return true;
}

/* An instant: the text of the decimal digits of a count of microseconds from the
   start of the test case, with no leading zero, within `NOW`'s range. */
static inline bool sce_test_input_trace_instant_(sce_jv_t value, const char *what, sce_test_instant_t *out,
                                                 sce_saved_error_t *err) {
    char digits[SCE_SAVED_ERROR_LEN];
    size_t n = 0u;
    sce_test_decimal_t count;
    int64_t instant = 0;
    if (sce_jv_kind(value) != SCE_JSON_STRING) {
        return sce_saved_refuse(err, "%s is not an instant, the text of the digits of a count of microseconds", what);
    }
    (void)sce_jv_string(value, digits, sizeof(digits), &n);
    if (!sce_test_canonical_decimal(digits, n, &count) || count.negative) {
        return sce_saved_refuse(err, "%s is not an instant, the text of the digits of a count of microseconds", what);
    }
    if (!sce_test_decimal_signed(&count, 0, INT64_MAX, &instant)) {
        return sce_saved_refuse(err, "%s (%s) is past the microseconds the clock counts to", what, digits);
    }
    *out = instant;
    return true;
}

/**
 * The trace the `len` bytes at `text` hold, in `*trace`: the JSON
 * Canonicalization Scheme of one trace, which a file may follow with one line
 * feed, as a text file ends. Each value is a span of `text`, which the caller
 * keeps until the trace is played.
 *
 * Refused, naming why in `*err`, where the text is not JSON, the shape is not
 * `sce-test-trace.v1`, the inputs are out of the order of their instants, or the
 * text is not the trace's one canonical spelling; and where it is past a bound
 * of this runtime. Whether each input fits the suite is the configuration's to
 * read (`sce_test_input_trace_player_init`).
 */
static inline bool sce_test_input_trace_parse(sce_test_input_trace_t *trace, const char *text, size_t len,
                                              sce_saved_error_t *err) {
    static const char *const TRACE_MEMBERS[] = {"schema", "suite", "test_case", "inputs"};
    static const char *const INPUT_MEMBERS[] = {"at", "port", "type", "value"};
    size_t body = len;
    if (body > 0u && text[body - 1u] == '\n') {
        --body;
    }
    memset(trace, 0, sizeof(*trace));
    if (body >= sizeof(trace->canonical)) {
        return sce_saved_refuse(err,
                                "the trace of %lu bytes is past the %u a trace holds (SCE_TEST_INPUT_TRACE_TEXT_SIZE)",
                                (unsigned long)body, (unsigned)(SCE_TEST_INPUT_TRACE_TEXT_SIZE - 1u));
    }
    sce_jv_t root;
    sce_saved_error_t why;
    why.message[0] = '\0';
    if (!sce_json_parse(text, body, &root, &why)) {
        return sce_saved_refuse(err, "the trace is %s", why.message);
    }
    sce_jv_t members[4];
    if (!sce_test_input_trace_members_(root, TRACE_MEMBERS, 4u, members, "the trace", err)) {
        return false;
    }
    if (sce_jv_kind(members[0]) != SCE_JSON_STRING || !sce_jv_string_is(members[0], SCE_TEST_INPUT_TRACE_SCHEMA)) {
        return sce_saved_refuse(err, "the trace's schema is not %s", SCE_TEST_INPUT_TRACE_SCHEMA);
    }
    if (sce_jv_kind(members[3]) != SCE_JSON_ARRAY) {
        return sce_saved_refuse(err, "the trace's inputs are not an array");
    }
    sce_jiter_t items;
    sce_jv_t item;
    (void)sce_jv_iter(members[3], &items);
    while (sce_jiter_element(&items, &item)) {
        const size_t index = trace->count;
        if (index >= SCE_TEST_MAX_INPUT_TRACE_INPUTS) {
            return sce_saved_refuse(err,
                                    "the trace holds more than the %u inputs a trace holds "
                                    "(SCE_TEST_MAX_INPUT_TRACE_INPUTS)",
                                    (unsigned)SCE_TEST_MAX_INPUT_TRACE_INPUTS);
        }
        char what[64];
        char of[96];
        sce_jv_t fields[4];
        sce_test_input_trace_entry_t *input = &trace->inputs[index];
        (void)snprintf(what, sizeof(what), "input %lu", (unsigned long)index);
        if (!sce_test_input_trace_members_(item, INPUT_MEMBERS, 4u, fields, what, err)) {
            return false;
        }
        (void)snprintf(of, sizeof(of), "the instant of %s", what);
        if (!sce_test_input_trace_instant_(fields[0], of, &input->at, err)) {
            return false;
        }
        if (index > 0u && trace->inputs[index - 1u].at > input->at) {
            return sce_saved_refuse(err,
                                    "%s, at %lld, comes after an input at %lld: the inputs stand in order of their "
                                    "instants",
                                    what, (long long)input->at, (long long)trace->inputs[index - 1u].at);
        }
        (void)snprintf(of, sizeof(of), "the port of %s", what);
        if (!sce_test_input_trace_name_(fields[1], of, input->port, err)) {
            return false;
        }
        (void)snprintf(of, sizeof(of), "the type of %s", what);
        if (!sce_test_input_trace_name_(fields[2], of, input->type_name, err)) {
            return false;
        }
        input->value = fields[3];
        trace->count++;
    }
    if (!sce_test_input_trace_name_(members[1], "the trace's suite", trace->suite, err) ||
        !sce_test_input_trace_name_(members[2], "the trace's test case", trace->test_case, err)) {
        return false;
    }
    size_t written = 0u;
    if (!sce_test_input_trace_write(trace, trace->canonical, sizeof(trace->canonical), &written) || written != body ||
        memcmp(trace->canonical, text, body) != 0) {
        return sce_saved_refuse(err,
                                "the trace is not written in its one canonical text, the JSON Canonicalization Scheme");
    }
    return true;
}

/** What a component sent to a system port, as the player records it. */
typedef struct sce_test_input_trace_output_s {
    /** `NOW` when it was sent. */
    sce_test_instant_t at;
    /** The system port it was sent to, a name the configuration lends. */
    const char *system_port;
    /** The message. */
    sce_test_message_t message;
} sce_test_input_trace_output_t;

/**
 * A system adapter that plays a trace (docs/SCE_TTCN3_LANE.md §10.5): the C11
 * form of the Rust and C++ `TraceAdapter`.
 *
 * At each instant it delivers the inputs due then, in the trace's order, to their
 * system ports (docs/SCE_TEST_PROFILE.md §3.2 step 2), and it records what the
 * components send to the system ports. It announces the instant of its next
 * input (`next_input`), so when no branch can fire the run's clock advances to
 * the earlier of that instant and the next timer expiry or the guard (§3.3,
 * §12.4 step 4), and the run is the same whenever it is played. Placed by the
 * caller; `adapter` is its first member, which the run is handed.
 */
typedef struct sce_test_input_trace_player_s {
    sce_test_system_adapter_t adapter;
    /** Each input's instant, system port and message, read before the run. */
    sce_test_instant_t at[SCE_TEST_MAX_INPUT_TRACE_INPUTS];
    char ports[SCE_TEST_MAX_INPUT_TRACE_INPUTS][SCE_MAX_ID_LEN];
    sce_test_message_t messages[SCE_TEST_MAX_INPUT_TRACE_INPUTS];
    size_t count;
    /** The first input not yet delivered. */
    size_t next;
    /** What the components sent to the system ports, in the order they sent it. */
    sce_test_input_trace_output_t sent[SCE_TEST_MAX_INPUT_TRACE_OUTPUTS];
    size_t sent_count;
    /** The messages past `SCE_TEST_MAX_INPUT_TRACE_OUTPUTS`, not recorded. */
    size_t sent_lost;
} sce_test_input_trace_player_t;

static inline void sce_test_input_trace_player_send_(sce_test_system_adapter_t *adapter, const char *system_port,
                                                     const sce_test_message_t *message, sce_test_instant_t now) {
    sce_test_input_trace_player_t *self = (sce_test_input_trace_player_t *)(void *)adapter;
    if (self->sent_count >= SCE_TEST_MAX_INPUT_TRACE_OUTPUTS) {
        self->sent_lost++;
        return;
    }
    sce_test_input_trace_output_t *output = &self->sent[self->sent_count++];
    output->at = now;
    output->system_port = system_port;
    output->message = *message;
}

static inline bool sce_test_input_trace_player_next_input_(const sce_test_system_adapter_t *adapter,
                                                           sce_test_instant_t *at) {
    const sce_test_input_trace_player_t *self = (const sce_test_input_trace_player_t *)(const void *)adapter;
    if (self->next >= self->count) {
        return false;
    }
    *at = self->at[self->next];
    return true;
}

static inline bool sce_test_input_trace_player_take_input_due_(sce_test_system_adapter_t *adapter,
                                                               sce_test_instant_t now, const char **system_port,
                                                               sce_test_message_t *message) {
    sce_test_input_trace_player_t *self = (sce_test_input_trace_player_t *)(void *)adapter;
    if (self->next >= self->count || self->at[self->next] > now) {
        return false;
    }
    *system_port = self->ports[self->next];
    *message = self->messages[self->next];
    self->next++;
    return true;
}

/* The functions of the player, as the run calls a system adapter's. */
static inline const sce_test_system_adapter_vtable_t *sce_test_input_trace_player_vtable_(void) {
    static const sce_test_system_adapter_vtable_t vtable = {sce_test_input_trace_player_send_,
                                                            sce_test_input_trace_player_next_input_,
                                                            sce_test_input_trace_player_take_input_due_, NULL};
    return &vtable;
}

/**
 * The player of `*trace` in `*player`, each input's value read by `reader`, the
 * reader of the test case's configuration (NULL for one with no system port).
 * Refused whole, naming the input in `*err`, where one does not fit the
 * configuration: its port is no system port of it, its type is not on the `in`
 * list of that port's type, or its value does not belong to its type
 * (docs/SCE_TTCN3_LANE.md §10.2). The trace and the text it lends are not read
 * again once this returns.
 */
static inline bool sce_test_input_trace_player_init(sce_test_input_trace_player_t *player,
                                                    const sce_test_input_trace_t *trace,
                                                    sce_test_input_reader_fn reader, sce_saved_error_t *err) {
    memset(player, 0, sizeof(*player));
    player->adapter.vtable = sce_test_input_trace_player_vtable_();
    for (size_t i = 0u; i < trace->count; ++i) {
        const sce_test_input_trace_entry_t *input = &trace->inputs[i];
        sce_saved_error_t why;
        why.message[0] = '\0';
        const bool held = reader != NULL
                              ? reader(input->port, input->type_name, input->value, &player->messages[i], &why)
                              : sce_test_not_an_input(input->port, input->type_name, NULL, 0u, &why);
        if (!held) {
            return sce_saved_refuse(err, "input %lu, at %lld: %s", (unsigned long)i, (long long)input->at, why.message);
        }
        player->at[i] = input->at;
        memcpy(player->ports[i], input->port, sizeof(player->ports[i]));
        player->count++;
    }
    return true;
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_TEST_INPUT_TRACE_H */
