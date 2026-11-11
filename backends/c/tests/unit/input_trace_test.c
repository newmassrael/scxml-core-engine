// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The recorded input trace of the test profile's C11 runtime
// (docs/SCE_TTCN3_LANE.md §10): the trace's reader and its one canonical text,
// its player (sce/test/input_trace.h), and the readers of the canonical form of
// a value it holds (sce/test/from_canonical.h), each the inverse of a writer of
// sce/test/canonical.h. Each case is the twin of the C++ case of the same
// subject in tests/test_runtime/TraceTest.cpp, and through it of the Rust
// runtime's in backends/rust/runtime/src/test/trace.rs and
// src/test/from_canonical.rs, with the same texts and the same refusals; the
// cases of a text that holds U+0000 or is not UTF-8 hold what only the C reader
// refuses by its own words.
//
// Header-only code; nothing here allocates.

#include <sce/saved_json.h>
#include <sce/test/canonical.h>
#include <sce/test/from_canonical.h>
#include <sce/test/input_trace.h>
#include <sce/test/port.h>
#include <sce/test/test_case_run.h>

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#ifndef SCE_PROJECT_ROOT
#error "SCE_PROJECT_ROOT names the repository's root, which the trace's wire schema is read from; the build defines it"
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

// The value a judged text holds, or `null` where the text is no JSON, which the
// check names.
static sce_jv_t judged(const char *text) {
    static const char null_text[] = "null";
    sce_jv_t value;
    sce_saved_error_t err;
    err.message[0] = '\0';
    if (!sce_json_parse(text, strlen(text), &value, &err)) {
        CHECK(false, "`%s` is JSON: %s", text, err.message);
        value.p = null_text;
        value.end = null_text + 4;
    }
    return value;
}

// Whether `contains` is in `text`.
static bool holds(const char *text, const char *contains) {
    return strstr(text, contains) != NULL;
}

// ── The trace (sce/test/input_trace.h) ──────────────────────────────

static char schema_text[32768];

// The status this runtime declares is the schema file's own, and the schema
// pins the name of the shape it reads. SCE_WIRE_CONTRACTS.md requires one
// commit to move both.
static void the_trace_the_schema_file_declares_the_status_this_runtime_does(void) {
    char path[1024];
    (void)snprintf(path, sizeof(path), "%s/schemas/sce-test-trace.v1.schema.json", SCE_PROJECT_ROOT);
    FILE *file = fopen(path, "rb");
    if (file == NULL) {
        CHECK(false, "the schema is readable at %s", path);
        return;
    }
    const size_t len = fread(schema_text, 1u, sizeof(schema_text) - 1u, file);
    (void)fclose(file);
    schema_text[len] = '\0';
    sce_jv_t root;
    sce_saved_error_t err;
    err.message[0] = '\0';
    if (!sce_json_parse(schema_text, len, &root, &err)) {
        CHECK(false, "the schema is JSON: %s", err.message);
        return;
    }
    sce_jv_t status;
    CHECK(sce_jv_get(root, "x-sce-schema-status", &status) &&
              sce_jv_string_is(status, SCE_TEST_INPUT_TRACE_SCHEMA_STATUS),
          "schemas/sce-test-trace.v1.schema.json x-sce-schema-status disagrees with "
          "SCE_TEST_INPUT_TRACE_SCHEMA_STATUS");
    sce_jv_t properties;
    sce_jv_t shape;
    sce_jv_t named;
    CHECK(sce_jv_get(root, "properties", &properties) && sce_jv_get(properties, "schema", &shape) &&
              sce_jv_get(shape, "const", &named) && sce_jv_string_is(named, SCE_TEST_INPUT_TRACE_SCHEMA),
          "the schema pins the shape SCE_TEST_INPUT_TRACE_SCHEMA names");
}

static const char ONE_INPUT[] = "{\"inputs\":[{\"at\":\"500\",\"port\":\"sut\",\"type\":\"job_reply\","
                                "\"value\":{\"code\":\"0\",\"job\":\"7\"}}],"
                                "\"schema\":\"sce-test-trace.v1\",\"suite\":\"runtime_suite\","
                                "\"test_case\":\"answers_the_job\"}";

// The trace of the case running, and the text it was read from, which it lends
// its values.
static sce_test_input_trace_t the_trace;
static char the_text[4096];
static char written[4096];

// The reason `text` is refused for, which it must be, in `*err`.
static bool parse_refused(const char *text, sce_saved_error_t *err) {
    err->message[0] = '\0';
    const bool accepted = sce_test_input_trace_parse(&the_trace, text, strlen(text), err);
    CHECK(!accepted, "`%s` is read where it should be refused", text);
    return !accepted;
}

// A trace is read from its canonical text, with or without the line feed a file
// ends in, and written back as the same bytes.
static void the_trace_is_read_from_its_canonical_text_and_written_back(void) {
    sce_saved_error_t err;
    err.message[0] = '\0';
    CHECK(sce_test_input_trace_parse(&the_trace, ONE_INPUT, strlen(ONE_INPUT), &err), "read: %s", err.message);
    CHECK(strcmp(the_trace.suite, "runtime_suite") == 0, "suite %s", the_trace.suite);
    CHECK(strcmp(the_trace.test_case, "answers_the_job") == 0, "test case %s", the_trace.test_case);
    CHECK(the_trace.count == 1u, "%lu inputs", (unsigned long)the_trace.count);
    if (the_trace.count == 1u) {
        const sce_test_input_trace_entry_t *input = &the_trace.inputs[0];
        CHECK(input->at == 500, "at %lld", (long long)input->at);
        CHECK(strcmp(input->port, "sut") == 0, "port %s", input->port);
        CHECK(strcmp(input->type_name, "job_reply") == 0, "type %s", input->type_name);
        static const char value[] = "{\"code\":\"0\",\"job\":\"7\"}";
        CHECK((size_t)(input->value.end - input->value.p) == strlen(value) &&
                  memcmp(input->value.p, value, strlen(value)) == 0,
              "the value is its canonical text");
    }
    size_t len = 0u;
    CHECK(sce_test_input_trace_write(&the_trace, written, sizeof(written), &len) && len == strlen(ONE_INPUT) &&
              memcmp(written, ONE_INPUT, len) == 0,
          "written back as %.*s", (int)len, written);
    (void)snprintf(the_text, sizeof(the_text), "%s\n", ONE_INPUT);
    CHECK(sce_test_input_trace_parse(&the_trace, the_text, strlen(the_text), &err), "ended: %s", err.message);
    CHECK(sce_test_input_trace_write(&the_trace, written, sizeof(written), &len) && len == strlen(ONE_INPUT) &&
              memcmp(written, ONE_INPUT, len) == 0,
          "the line feed is no part of the trace");
}

// `from` replaced once by `to` in ONE_INPUT, which holds it, in the_text.
static const char *changed(const char *from, const char *to) {
    const char *at = strstr(ONE_INPUT, from);
    CHECK(at != NULL, "ONE_INPUT holds `%s`", from);
    if (at == NULL) {
        return ONE_INPUT;
    }
    (void)snprintf(the_text, sizeof(the_text), "%.*s%s%s", (int)(at - ONE_INPUT), ONE_INPUT, to, at + strlen(from));
    return the_text;
}

// Each change of one member of a trace it first reads is refused, by name.
static void the_trace_that_is_not_of_its_shape_is_refused_by_name(void) {
    static const struct {
        const char *from;
        const char *to;
        const char *expected;
    } CASES[] = {
        {"sce-test-trace.v1", "sce-test-trace.v2", "schema is not sce-test-trace.v1"},
        {"\"500\"", "500", "the instant of input 0"},
        {"\"500\"", "\"0500\"", "the instant of input 0"},
        {"\"500\"", "\"-5\"", "the instant of input 0"},
        {"\"500\"", "\"9223372036854775808\"", "past the microseconds"},
        {"\"port\":\"sut\"", "\"port\":\"\"", "the port of input 0"},
        {"\"type\"", "\"kind\"", "input 0 has no member 'kind'"},
        {",\"suite\":\"runtime_suite\"", "", "the trace has no 'suite'"},
        {"{\"inputs\"", "{ \"inputs\"", "one canonical text"},
        {"[{", "{", "not JSON"},
    };

    for (size_t i = 0u; i < sizeof(CASES) / sizeof(CASES[0]); ++i) {
        sce_saved_error_t err;
        if (parse_refused(changed(CASES[i].from, CASES[i].to), &err)) {
            CHECK(holds(err.message, CASES[i].expected), "%s: %s", CASES[i].expected, err.message);
        }
    }
    static const char OUT_OF_ORDER[] = "{\"inputs\":[{\"at\":\"5\",\"port\":\"sut\",\"type\":\"t\",\"value\":{}},"
                                       "{\"at\":\"4\",\"port\":\"sut\",\"type\":\"t\",\"value\":{}}],"
                                       "\"schema\":\"sce-test-trace.v1\",\"suite\":\"s\",\"test_case\":\"c\"}";
    sce_saved_error_t err;
    if (parse_refused(OUT_OF_ORDER, &err)) {
        CHECK(holds(err.message, "input 1, at 4, comes after an input at 5"), "%s", err.message);
    }
}

// A number past the range of a double has no canonical text, so a trace whose
// value holds one is refused as not written in it. The twin of the Rust,
// Kotlin and C++ cases of the same subject.
static void the_trace_whose_value_holds_a_number_past_a_double_is_refused(void) {
    sce_saved_error_t err;
    if (parse_refused(changed("{\"code\":\"0\",\"job\":\"7\"}", "1e400"), &err)) {
        CHECK(strcmp(err.message,
                     "the trace is not written in its one canonical text, the JSON Canonicalization Scheme") == 0,
              "%s", err.message);
    }
}

// An input on a port the configuration does not have, or of a type its port
// does not take, is refused, naming which.
static void the_trace_an_input_the_configuration_does_not_take_is_refused_by_name(void) {
    static const char *const SUT[] = {"sut"};
    sce_saved_error_t err;
    err.message[0] = '\0';
    CHECK(!sce_test_not_an_input("sut", "job_request", SUT, 1u, &err) &&
              strcmp(err.message, "'job_request' is not on the `in` list of the type of the system port 'sut'") == 0,
          "%s", err.message);
    CHECK(!sce_test_not_an_input("oracle", "job_reply", SUT, 1u, &err) &&
              strcmp(err.message, "'oracle' is not a system port of the test case's configuration") == 0,
          "%s", err.message);
    CHECK(sce_test_input_trace_parse(&the_trace, ONE_INPUT, strlen(ONE_INPUT), &err), "read: %s", err.message);
    static sce_test_input_trace_player_t player;
    CHECK(!sce_test_input_trace_player_init(&player, &the_trace, NULL, &err) &&
              strcmp(err.message, "input 0, at 500: 'sut' is not a system port of the test case's configuration") == 0,
          "%s", err.message);
}

// The reply ONE_INPUT carries, as a reader of a configuration reads it.
typedef struct reply_s {
    uint32_t code;
    uint32_t job;
} reply_t;

static void reply_canonical(sce_jcs_t *jcs, const void *value) {
    const reply_t *reply = (const reply_t *)value;
    sce_jcs_begin_object(jcs);
    sce_jcs_key(jcs, "code");
    SCE_TEST_CANONICAL_INTEGER(jcs, reply->code);
    sce_jcs_key(jcs, "job");
    SCE_TEST_CANONICAL_INTEGER(jcs, reply->job);
    sce_jcs_end_object(jcs);
}

static const sce_test_record_type_t REPLY_TYPE = {"job_reply", reply_canonical};

// The reader of the one input ONE_INPUT holds: a reply on `sut`, under the event
// `job.reply`.
static bool reply_input(const char *system_port, const char *type_name, sce_jv_t value, sce_test_message_t *message,
                        sce_saved_error_t *err) {
    static const char *const SUT[] = {"sut"};
    if (strcmp(system_port, "sut") != 0 || strcmp(type_name, "job_reply") != 0) {
        return sce_test_not_an_input(system_port, type_name, SUT, 1u, err);
    }
    static const char *const FIELDS[] = {"code", "job"};
    reply_t reply;
    sce_jv_t field;
    uint64_t held = 0u;
    memset(&reply, 0, sizeof(reply));
    if (!sce_test_from_canonical_record(value, FIELDS, 2u, "job_reply", err) ||
        !sce_test_from_canonical_field(value, "code", "job_reply.code", &field, err) ||
        !sce_test_from_canonical_unsigned(field, UINT32_MAX, "job_reply.code", &held, err)) {
        return false;
    }
    reply.code = (uint32_t)held;
    if (!sce_test_from_canonical_field(value, "job", "job_reply.job", &field, err) ||
        !sce_test_from_canonical_unsigned(field, UINT32_MAX, "job_reply.job", &held, err)) {
        return false;
    }
    reply.job = (uint32_t)held;
    return sce_test_input_message(message, "job.reply",
                                  sce_test_message_value(SCE_HOST_VALUE(reply_t, &reply), &REPLY_TYPE), err);
}

// docs/SCE_TEST_PROFILE.md §3.2 step 2: the player announces the instant of its
// next input, delivers the inputs due at NOW and none before, and records what
// the components send.
static void the_trace_is_played_at_the_instants_it_holds(void) {
    static sce_test_input_trace_player_t player;
    sce_saved_error_t err;
    err.message[0] = '\0';
    CHECK(sce_test_input_trace_parse(&the_trace, ONE_INPUT, strlen(ONE_INPUT), &err), "read: %s", err.message);
    CHECK(sce_test_input_trace_player_init(&player, &the_trace, reply_input, &err), "played: %s", err.message);
    sce_test_system_adapter_t *adapter = &player.adapter;
    sce_test_instant_t at = 0;
    CHECK(adapter->vtable->next_input(adapter, &at) && at == 500, "the next input is at %lld", (long long)at);
    const char *system_port = NULL;
    sce_test_message_t message;
    memset(&message, 0, sizeof(message));
    CHECK(!adapter->vtable->take_input_due(adapter, 499, &system_port, &message),
          "an input is not delivered before its instant");
    CHECK(adapter->vtable->take_input_due(adapter, 500, &system_port, &message), "the input is due at its instant");
    CHECK(system_port != NULL && strcmp(system_port, "sut") == 0, "on `sut`");
    CHECK(strcmp(message.event, "job.reply") == 0, "event %s", message.event);
    CHECK(message.record_type != NULL && strcmp(message.record_type->name, "job_reply") == 0,
          "of the record type job_reply");
    char text[64];
    sce_jcs_t jcs;
    sce_jcs_init(&jcs, text, sizeof(text));
    if (message.record_type != NULL) {
        message.record_type->canonical(&jcs, message.value.bytes);
    }
    CHECK(sce_jcs_finish(&jcs) && strcmp(text, "{\"code\":\"0\",\"job\":\"7\"}") == 0, "the value is %s", text);
    CHECK(!adapter->vtable->take_input_due(adapter, 500, &system_port, &message), "each input is delivered once");
    CHECK(!adapter->vtable->next_input(adapter, &at), "no input is left");
    adapter->vtable->send(adapter, "sut", &message, 600);
    CHECK(player.sent_count == 1u && player.sent[0].at == 600 && strcmp(player.sent[0].system_port, "sut") == 0,
          "what a component sends is recorded");
    CHECK(adapter->vtable->release == NULL, "the player holds nothing to give back");
}

// ── The readers of each form (sce/test/from_canonical.h) ────────────

// The value `value` reads as, through the canonical writer of an unsigned
// integer, into `buf`.
static sce_jv_t canonical_unsigned(uint64_t value, char *buf, size_t cap) {
    sce_jcs_t jcs;
    sce_jcs_init(&jcs, buf, cap);
    sce_test_canonical_unsigned(&jcs, value);
    CHECK(sce_jcs_finish(&jcs), "the integer is written");
    return judged(buf);
}

// An integer is read from the one spelling the writer gives it, at every width,
// and any other spelling, or a value past its type, is refused, naming where it
// stands.
static void the_canonical_readers_read_an_integer_from_the_text_of_its_digits_alone(void) {
    char buf[32];
    sce_saved_error_t err;
    uint64_t unsigned_value = 1u;
    int64_t signed_value = 0;
    err.message[0] = '\0';
    CHECK(sce_test_from_canonical_unsigned(canonical_unsigned(0u, buf, sizeof(buf)), UINT8_MAX, "n", &unsigned_value,
                                           &err) &&
              unsigned_value == 0u,
          "%s", err.message);
    CHECK(sce_test_from_canonical_unsigned(canonical_unsigned(UINT64_MAX, buf, sizeof(buf)), UINT64_MAX, "n",
                                           &unsigned_value, &err) &&
              unsigned_value == UINT64_MAX,
          "%s", err.message);
    CHECK(sce_test_from_canonical_signed(judged("\"-9223372036854775808\""), INT64_MIN, INT64_MAX, "n", &signed_value,
                                         &err) &&
              signed_value == INT64_MIN,
          "%s", err.message);
    static const char *const SPELLINGS[] = {"\"007\"", "\"+7\"", "\"-0\"", "\"7.0\"", "\"\"", "\" 7\""};
    for (size_t i = 0u; i < sizeof(SPELLINGS) / sizeof(SPELLINGS[0]); ++i) {
        CHECK(!sce_test_from_canonical_unsigned(judged(SPELLINGS[i]), UINT32_MAX, "job_reply.code", &unsigned_value,
                                                &err) &&
                  strncmp(err.message, "job_reply.code: ", 16u) == 0 && holds(err.message, "decimal digits"),
              "%s: %s", SPELLINGS[i], err.message);
    }
    CHECK(!sce_test_from_canonical_unsigned(judged("\"256\""), UINT8_MAX, "n", &unsigned_value, &err) &&
              holds(err.message, "past the range"),
          "%s", err.message);
    CHECK(!sce_test_from_canonical_unsigned(judged("\"-1\""), UINT32_MAX, "n", &unsigned_value, &err) &&
              holds(err.message, "past the range"),
          "%s", err.message);
    CHECK(!sce_test_from_canonical_signed(judged("\"-9223372036854775809\""), INT64_MIN, INT64_MAX, "n", &signed_value,
                                          &err) &&
              holds(err.message, "past the range"),
          "%s", err.message);
    CHECK(!sce_test_from_canonical_unsigned(judged("7"), UINT32_MAX, "n", &unsigned_value, &err) &&
              holds(err.message, "a number is not an integer"),
          "%s", err.message);
}

static void the_canonical_readers_read_a_bool_as_a_truth_value(void) {
    bool read = false;
    sce_saved_error_t err;
    err.message[0] = '\0';
    CHECK(sce_test_from_canonical_boolean(judged("true"), "b", &read, &err) && read, "%s", err.message);
    CHECK(!sce_test_from_canonical_boolean(judged("\"true\""), "b", &read, &err) &&
              holds(err.message, "a text is not a truth value"),
          "%s", err.message);
}

// A text is read within its bound in bytes, into the buffer of that bound; one
// that holds U+0000, where a C text ends, or that is not UTF-8, is refused.
static void the_canonical_readers_read_a_text_within_its_bound(void) {
    char text[4];
    char short_buffer[3];
    sce_saved_error_t err;
    err.message[0] = '\0';
    CHECK(sce_test_from_canonical_text(judged("\"abc\""), 3u, "t", text, sizeof(text), &err) &&
              strcmp(text, "abc") == 0,
          "%s", err.message);
    CHECK(!sce_test_from_canonical_text(judged("\"abcd\""), 3u, "t", text, sizeof(text), &err) &&
              holds(err.message, "past the 3"),
          "%s", err.message);
    CHECK(!sce_test_from_canonical_text(judged("\"abc\""), 3u, "t", short_buffer, sizeof(short_buffer), &err) &&
              holds(err.message, "past what its type holds"),
          "%s", err.message);
    CHECK(!sce_test_from_canonical_text(judged("\"a\\u0000b\""), 3u, "t", text, sizeof(text), &err) &&
              strcmp(err.message, "t: a text of 3 bytes holds U+0000, at which a C text of its type ends") == 0,
          "%s", err.message);
    CHECK(!sce_test_from_canonical_text(judged("\"a\xff\""), 3u, "t", text, sizeof(text), &err) &&
              strcmp(err.message, "t: a text of 2 bytes is not well-formed UTF-8") == 0,
          "%s", err.message);
}

// A byte string is read from upper-case hex alone, the writer's spelling, and
// back to the same text.
static void the_canonical_readers_read_a_byte_string_from_upper_case_hex(void) {
    static const uint8_t OCTETS[] = {0x00u, 0xabu, 0x0fu, 0xffu};
    char hex[16];
    uint8_t read[4];
    size_t len = 0u;
    sce_saved_error_t err;
    sce_jcs_t jcs;
    err.message[0] = '\0';
    sce_jcs_init(&jcs, hex, sizeof(hex));
    sce_test_canonical_bytes(&jcs, OCTETS, sizeof(OCTETS));
    CHECK(sce_jcs_finish(&jcs) && strcmp(hex, "\"00AB0FFF\"") == 0, "written as %s", hex);
    CHECK(sce_test_from_canonical_bytes(judged(hex), 4u, "b", read, sizeof(read), &len, &err) && len == 4u &&
              memcmp(read, OCTETS, sizeof(OCTETS)) == 0,
          "%s", err.message);
    static const char *const SPELLINGS[] = {"\"00ab\"", "\"0\"", "\"0G\""};
    for (size_t i = 0u; i < sizeof(SPELLINGS) / sizeof(SPELLINGS[0]); ++i) {
        CHECK(!sce_test_from_canonical_bytes(judged(SPELLINGS[i]), 4u, "b", read, sizeof(read), &len, &err) &&
                  holds(err.message, "upper-case hex"),
              "%s: %s", SPELLINGS[i], err.message);
    }
    CHECK(!sce_test_from_canonical_bytes(judged(hex), 3u, "b", read, sizeof(read), &len, &err) &&
              holds(err.message, "past the 3"),
          "%s", err.message);
}

// A closed enum is read by its declared names; an open one by its names and by
// the numbers no variant declares, a declared number being the second spelling
// of its variant.
static void the_canonical_readers_read_an_enum_by_its_declared_name(void) {
    static const char *const NAMES[] = {"red", "green"};
    static const uint64_t NUMBERS[] = {0u, 1u};
    size_t index = 0u;
    uint64_t number = 9u;
    sce_saved_error_t err;
    err.message[0] = '\0';
    CHECK(sce_test_from_canonical_enumerated(judged("\"green\""), NAMES, 2u, "c", &index, &err) && index == 1u, "%s",
          err.message);
    CHECK(!sce_test_from_canonical_enumerated(judged("\"blue\""), NAMES, 2u, "c", &index, &err) &&
              holds(err.message, "'blue' names no"),
          "%s", err.message);
    CHECK(sce_test_from_canonical_open_unsigned(judged("\"red\""), NAMES, NUMBERS, 2u, UINT8_MAX, "c", &number, &err) &&
              number == 0u,
          "%s", err.message);
    CHECK(sce_test_from_canonical_open_unsigned(judged("\"7\""), NAMES, NUMBERS, 2u, UINT8_MAX, "c", &number, &err) &&
              number == 7u,
          "%s", err.message);
    CHECK(!sce_test_from_canonical_open_unsigned(judged("\"1\""), NAMES, NUMBERS, 2u, UINT8_MAX, "c", &number, &err) &&
              holds(err.message, "written by its name"),
          "%s", err.message);
    CHECK(
        !sce_test_from_canonical_open_unsigned(judged("\"300\""), NAMES, NUMBERS, 2u, UINT8_MAX, "c", &number, &err) &&
            holds(err.message, "past the range"),
        "%s", err.message);
}

// A record holds only the fields it declares; a field whose type is not
// optional is present.
static void the_canonical_readers_read_a_record_of_its_declared_fields(void) {
    static const char *const FIELDS[] = {"job", "code"};
    sce_jv_t field;
    uint64_t job = 0u;
    sce_saved_error_t err;
    err.message[0] = '\0';
    const sce_jv_t value = judged("{\"code\":\"0\",\"job\":\"7\"}");
    CHECK(sce_test_from_canonical_record(value, FIELDS, 2u, "job_reply", &err), "%s", err.message);
    CHECK(sce_test_from_canonical_field(value, "job", "job_reply.job", &field, &err) &&
              sce_test_from_canonical_unsigned(field, UINT32_MAX, "job", &job, &err) && job == 7u,
          "%s", err.message);
    CHECK(!sce_test_from_canonical_optional_field(value, "note", &field), "an absent field is no member");
    CHECK(!sce_test_from_canonical_record(judged("{\"job\":\"7\",\"jobs\":\"1\"}"), FIELDS, 2u, "job_reply", &err) &&
              holds(err.message, "'jobs' is no field"),
          "%s", err.message);
    const sce_jv_t partial = judged("{\"job\":\"7\"}");
    CHECK(sce_test_from_canonical_record(partial, FIELDS, 2u, "job_reply", &err), "%s", err.message);
    CHECK(!sce_test_from_canonical_field(partial, "code", "job_reply.code", &field, &err) &&
              strcmp(err.message, "job_reply.code: the field is absent, and its type is not optional") == 0,
          "%s", err.message);
}

// A list is read within its bounds, an absent element as null.
static void the_canonical_readers_read_a_list_within_its_bounds(void) {
    sce_jiter_t items;
    sce_jv_t item;
    sce_saved_error_t err;
    err.message[0] = '\0';
    const sce_jv_t value = judged("[\"1\",null]");
    bool present[2] = {false, true};
    uint64_t first = 0u;
    size_t count = 0u;
    CHECK(sce_test_from_canonical_elements(value, 1u, 2u, "l", &items, &err), "%s", err.message);
    while (sce_jiter_element(&items, &item) && count < 2u) {
        present[count] = sce_test_from_canonical_present(item);
        if (count == 0u) {
            CHECK(sce_test_from_canonical_unsigned(item, UINT8_MAX, "l[]", &first, &err), "%s", err.message);
        }
        ++count;
    }
    CHECK(count == 2u && present[0] && !present[1] && first == 1u, "the elements read are 1 and an absent one");
    CHECK(!sce_test_from_canonical_elements(value, 3u, 4u, "l", &items, &err) &&
              holds(err.message, "outside its bounds, 3 to 4"),
          "%s", err.message);
}

int main(void) {
    the_trace_the_schema_file_declares_the_status_this_runtime_does();
    the_trace_is_read_from_its_canonical_text_and_written_back();
    the_trace_that_is_not_of_its_shape_is_refused_by_name();
    the_trace_whose_value_holds_a_number_past_a_double_is_refused();
    the_trace_an_input_the_configuration_does_not_take_is_refused_by_name();
    the_trace_is_played_at_the_instants_it_holds();
    the_canonical_readers_read_an_integer_from_the_text_of_its_digits_alone();
    the_canonical_readers_read_a_bool_as_a_truth_value();
    the_canonical_readers_read_a_text_within_its_bound();
    the_canonical_readers_read_a_byte_string_from_upper_case_hex();
    the_canonical_readers_read_an_enum_by_its_declared_name();
    the_canonical_readers_read_a_record_of_its_declared_fields();
    the_canonical_readers_read_a_list_within_its_bounds();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)puts("input trace: ok");
    return 0;
}
