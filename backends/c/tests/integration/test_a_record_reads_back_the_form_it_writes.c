// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The reader and the writer of the canonical form a test configuration's
// header generates for each record its system ports take
// (docs/SCE_TTCN3_LANE.md §10.3, §10.5; docs/SCE_TEST_PROFILE.md §12.5),
// compiled here over records of every form C11 holds: order_taken (a list of
// records with a closed enum, an optional text and integer, a record with a
// truth value, a byte string), job_status (a list of integers, optional
// integers) and form_extras (an enum of an open set, a list of optional
// elements, a 64-bit integer, a list with a lower count, an optional list and a
// list of lists). The headers are generated from
// sce-build/tests/fixtures/test_profile/trace_forms/ (backends/c/tests/CMakeLists.txt).
//
// The twin of tests/integration/ARecordReadsBackTheFormItWritesTest.cpp, and
// through it of backends/rust/tests/tests/a_record_reads_back_the_form_it_writes.rs,
// with the same values and the same refusals, but for the union the Rust
// configuration's order_placed holds, which C11 does not lower. A value written
// by `_canonical_<record>` and read back by `_read_<record>` is the value it
// was, through the canonical text; each malformed spelling of a form is
// refused, naming where it stands; and a trace of one record of each type
// replays to a log that holds each value as the trace does.
//
// What is C's own: every value, text and run lives in storage of static
// duration this file places. Nothing here allocates.

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include <sce/saved_json.h>
#include <sce/test/input_trace.h>
#include <sce/test/suite.h>
#include <sce/test/test_case_run.h>

#include "replays_every_unionless_form.h"
#include "shape_kind_open.h"
#include "unionless_forms_configuration.h"

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

// The most bytes the canonical text of one value of these records takes.
#define MAX_TEXT 1024u

// ── The values ──────────────────────────────────────────────────────────

// The writer of a record's canonical form, and its reader, over a value lent.
typedef void (*writer_fn)(sce_jcs_t *jcs, const void *value);
typedef bool (*reader_fn)(sce_jv_t value, void *out, sce_saved_error_t *err);

static bool read_order(sce_jv_t value, void *out, sce_saved_error_t *err) {
    return unionless_forms_configuration_read_order_taken(value, (OrderTakenPayload_t *)out, err);
}

static bool read_status(sce_jv_t value, void *out, sce_saved_error_t *err) {
    return unionless_forms_configuration_read_job_status(value, (JobStatusPayload_t *)out, err);
}

static bool read_extras(sce_jv_t value, void *out, sce_saved_error_t *err) {
    return unionless_forms_configuration_read_form_extras(value, (FormExtrasPayload_t *)out, err);
}

static const writer_fn WRITE_ORDER = unionless_forms_configuration_canonical_order_taken_lent;
static const writer_fn WRITE_STATUS = unionless_forms_configuration_canonical_job_status_lent;
static const writer_fn WRITE_EXTRAS = unionless_forms_configuration_canonical_form_extras_lent;

// A line of an order.
static OrderLinePayload_t line(const char *sku, uint16_t quantity, PaintColor_t color) {
    OrderLinePayload_t made;
    memset(&made, 0, sizeof(made));
    made.sku = sce_static_string_16_of(sku);
    made.quantity = quantity;
    made.color = color;
    return made;
}

// An order of two lines, one with a text of a character of two bytes and an
// escaped quote, the widest order id, a note and no priority, paid with the
// `count` octets of `voucher`.
static OrderTakenPayload_t order(const uint8_t *voucher, size_t count) {
    OrderTakenPayload_t made;
    memset(&made, 0, sizeof(made));
    made.order_id = UINT32_MAX;
    made.lines.len = 2u;
    made.lines.items[0] = line("A-1", 2u, PAINT_COLOR_GREEN);
    made.lines.items[1] = line("\"q\" \xC3\xA9", UINT16_MAX, PAINT_COLOR_BLUE);
    made.note.present = true;
    made.note.value = sce_static_string_32_of("ring twice");
    made.priority.present = false;
    made.customer.full_name = sce_static_string_24_of("Ada");
    made.customer.vip = true;
    for (size_t i = 0u; i < count; ++i) {
        made.voucher_code.data[i] = voucher[i];
    }
    made.voucher_code.len = count;
    return made;
}

// The voucher the order is paid with: three octets, and none.
static const uint8_t VOUCHER[] = {0x00u, 0xabu, 0xffu};

static JobStatusPayload_t status(bool first) {
    JobStatusPayload_t made;
    memset(&made, 0, sizeof(made));
    if (first) {
        made.elapsed_ms = 0u;
        made.note.present = false;
        made.codes.len = 0u;
        made.tag.present = true;
        made.tag.value = UINT8_MAX;
        made.retry.present = false;
        made.count = 7u;
    } else {
        made.elapsed_ms = 12u;
        made.note.present = true;
        made.note.value = sce_static_string_32_of("late");
        made.codes.len = 4u;
        made.codes.items[0] = 3u;
        made.codes.items[1] = 9u;
        made.codes.items[2] = 5u;
        made.codes.items[3] = 0u;
        made.tag.present = false;
        made.retry.present = true;
        made.retry.value = 0u;
        made.count = 0u;
    }
    return made;
}

// The extras: a declared variant and a number no variant declares, a list with
// an absent element, the least and the greatest 64-bit integer, a list at its
// lower count and at its bound, an optional list present and absent, and a list
// of lists with an empty one.
static FormExtrasPayload_t extras(bool first) {
    FormExtrasPayload_t made;
    memset(&made, 0, sizeof(made));
    if (first) {
        made.shape = SHAPE_KIND_OPEN_CIRCLE;
        made.slots.len = 3u;
        made.slots.items[0].present = true;
        made.slots.items[0].value = 1u;
        made.slots.items[1].present = false;
        made.slots.items[2].present = true;
        made.slots.items[2].value = UINT16_MAX;
        made.least = INT64_MIN;
        made.flags.len = 1u;
        made.flags.items[0] = true;
        made.spare.present = true;
        made.spare.value.len = 2u;
        made.spare.value.items[0] = 0u;
        made.spare.value.items[1] = 255u;
        made.grid.len = 2u;
        made.grid.items[0].len = 2u;
        made.grid.items[0].items[0] = 1u;
        made.grid.items[0].items[1] = 2u;
        made.grid.items[1].len = 0u;
    } else {
        made.shape = shape_kind_open_from_underlying(9u);
        made.slots.len = 0u;
        made.least = INT64_MAX;
        made.flags.len = 2u;
        made.flags.items[0] = false;
        made.flags.items[1] = true;
        made.spare.present = false;
        made.grid.len = 0u;
    }
    return made;
}

// `value` in its canonical text, written by `writer` into the MAX_TEXT bytes at
// `text`.
static void written(writer_fn writer, const void *value, char *text) {
    sce_jcs_t jcs;
    sce_jcs_init(&jcs, text, MAX_TEXT);
    writer(&jcs, value);
    CHECK(sce_jcs_finish(&jcs), "the value is written in the scheme: %s", jcs.refusal != NULL ? jcs.refusal : "");
}

// `value` written by `writer` in its canonical text, read back from that text by
// `reader` into `scratch`, and written again: the same text, so the reader holds
// every field the writer wrote, as the writer wrote it.
static void round_trip(const char *what, const void *value, writer_fn writer, reader_fn reader, void *scratch) {
    static char first[MAX_TEXT];
    static char second[MAX_TEXT];
    written(writer, value, first);
    sce_jv_t judged;
    sce_saved_error_t err;
    err.message[0] = '\0';
    CHECK(sce_json_parse(first, strlen(first), &judged, &err), "%s is JSON: %s", what, err.message);
    CHECK(reader(judged, scratch, &err), "%s reads back from %s: %s", what, first, err.message);
    written(writer, scratch, second);
    CHECK(strcmp(first, second) == 0, "%s reads back as it was written:\n  wrote %s\n  read  %s", what, first, second);
}

static OrderTakenPayload_t read_order_value;
static JobStatusPayload_t read_status_value;
static FormExtrasPayload_t read_extras_value;

// docs/SCE_TEST_PROFILE.md §12.5: a value of each form reads back from the text
// its writer gives as the value it was.
static void a_record_reads_back_a_value_of_each_form_from_its_canonical_text(void) {
    const OrderTakenPayload_t paid = order(VOUCHER, sizeof(VOUCHER));
    const OrderTakenPayload_t unpaid = order(VOUCHER, 0u);
    const JobStatusPayload_t first_status = status(true);
    const JobStatusPayload_t second_status = status(false);
    const FormExtrasPayload_t first_extras = extras(true);
    const FormExtrasPayload_t second_extras = extras(false);
    round_trip("an order paid with a voucher", &paid, WRITE_ORDER, read_order, &read_order_value);
    round_trip("an order paid with no voucher", &unpaid, WRITE_ORDER, read_order, &read_order_value);
    round_trip("the first status", &first_status, WRITE_STATUS, read_status, &read_status_value);
    round_trip("the second status", &second_status, WRITE_STATUS, read_status, &read_status_value);
    round_trip("the first extras", &first_extras, WRITE_EXTRAS, read_extras, &read_extras_value);
    round_trip("the second extras", &second_extras, WRITE_EXTRAS, read_extras, &read_extras_value);
    CHECK(shape_kind_open_to_underlying(read_extras_value.shape) == 9u, "a number no variant declares");
    CHECK(read_extras_value.least == INT64_MAX, "the greatest 64-bit integer");
    round_trip("an order paid with a voucher", &paid, WRITE_ORDER, read_order, &read_order_value);
    CHECK(read_order_value.voucher_code.len == 3u && memcmp(read_order_value.voucher_code.data, VOUCHER, 3u) == 0,
          "the voucher's octets");
    CHECK(read_order_value.lines.len == 2u && strcmp(read_order_value.lines.items[1].sku.data, "\"q\" \xC3\xA9") == 0,
          "the second line's sku");
    CHECK(!read_order_value.priority.present, "no priority");
}

// The reason `reader` refuses the canonical text of `value` with `from` replaced
// once by `to` for, which must be `expected`.
static void refused(const void *value, writer_fn writer, reader_fn reader, void *scratch, const char *from,
                    const char *to, const char *expected) {
    static char text[MAX_TEXT];
    static char changed[2u * MAX_TEXT];
    written(writer, value, text);
    const char *at = strstr(text, from);
    CHECK(at != NULL, "`%s` in %s", from, text);
    if (at == NULL) {
        return;
    }
    (void)snprintf(changed, sizeof(changed), "%.*s%s%s", (int)(at - text), text, to, at + strlen(from));
    sce_jv_t judged;
    sce_saved_error_t err;
    err.message[0] = '\0';
    if (!sce_json_parse(changed, strlen(changed), &judged, &err)) {
        CHECK(false, "%s is JSON: %s", changed, err.message);
        return;
    }
    CHECK(!reader(judged, scratch, &err), "%s is read", changed);
    CHECK(strcmp(err.message, expected) == 0, "refused for `%s`, not `%s`", err.message, expected);
}

// docs/SCE_TTCN3_LANE.md §10.2: a malformed spelling of each form is refused,
// naming the place it stands at: the record's type, each field by its id and
// `[]` for an element of a list.
static void a_record_refuses_a_malformed_spelling_of_each_form_by_name(void) {
    const OrderTakenPayload_t paid = order(VOUCHER, sizeof(VOUCHER));
    const FormExtrasPayload_t extra = extras(true);
    const JobStatusPayload_t first_status = status(true);
    char long_name[64];
    (void)snprintf(long_name, sizeof(long_name), "\"full_name\":\"%s\"", "xxxxxxxxxxxxxxxxxxxxxxxxx");

    static const struct {
        int record;
        const char *from;
        const char *to;
        const char *expected;
    } CASES[] = {
        {0, "\"order_id\":\"4294967295\"", "\"order_id\":\"007\"",
         "order_taken.order_id: '007' is not the text of an integer's decimal digits"},
        {0, "\"quantity\":\"2\"", "\"quantity\":\"65536\"",
         "order_taken.lines[].quantity: 65536 is past the range of its type"},
        {0, "\"vip\":true", "\"vip\":\"true\"", "order_taken.customer.vip: a text is not a truth value"},
        {0, "\"color\":\"green\"", "\"color\":\"purple\"",
         "order_taken.lines[].color: 'purple' names no variant of its enum"},
        {0, "\"note\":\"ring twice\"", "\"note\":null", "order_taken.note: null is not a text"},
        {0, "{\"customer\":", "{\"rush\":\"1\",\"customer\":", "order_taken: 'rush' is no field of its record"},
        {0, "\"order_id\":\"4294967295\",", "",
         "order_taken.order_id: the field is absent, and its type is not optional"},
        {0, "\"lines\":[",
         "\"lines\":[{\"color\":\"red\",\"quantity\":\"1\",\"sku\":\"s\"},"
         "{\"color\":\"red\",\"quantity\":\"1\",\"sku\":\"s\"},"
         "{\"color\":\"red\",\"quantity\":\"1\",\"sku\":\"s\"},",
         "order_taken.lines: a list of 5 elements is outside its bounds, 0 to 4"},
        {0, "\"voucher_code\":\"00ABFF\"", "\"voucher_code\":\"00abff\"",
         "order_taken.voucher_code: '00abff' is not octets in upper-case hex, two digits each"},
        {0, "\"voucher_code\":\"00ABFF\"", "\"voucher_code\":\"0011223344\"",
         "order_taken.voucher_code: a byte string of 5 octets is past the 4 its type holds"},
        {1, "\"tag\":\"255\"", "\"tag\":\"256\"", "job_status.tag: 256 is past the range of its type"},
        {1, "\"codes\":[]", "\"codes\":[\"x\"]",
         "job_status.codes[]: 'x' is not the text of an integer's decimal digits"},
        {2, "\"shape\":\"circle\"", "\"shape\":\"1\"",
         "form_extras.shape: 1 is the number of a declared variant, which is written by its name"},
        {2, "\"shape\":\"circle\"", "\"shape\":\"hexagon\"",
         "form_extras.shape: 'hexagon' names no variant of its enum, and is no number"},
        {2, "\"slots\":[\"1\",null,", "\"slots\":[\"1\",\"x\",",
         "form_extras.slots[]: 'x' is not the text of an integer's decimal digits"},
        {2, "\"flags\":[true]", "\"flags\":[]",
         "form_extras.flags: a list of 0 elements is outside its bounds, 1 to 2"},
        {2, "\"grid\":[[\"1\",\"2\"]", "\"grid\":[[\"1\",\"2\",\"3\"]",
         "form_extras.grid[]: a list of 3 elements is outside its bounds, 0 to 2"},
        {2, "\"least\":\"-9223372036854775808\"", "\"least\":\"-9223372036854775809\"",
         "form_extras.least: -9223372036854775809 is past the range of its type"},
        {2, "\"spare\":[\"0\",\"255\"]", "\"spare\":null", "form_extras.spare: null is not a list, which is an array"},
    };

    for (size_t i = 0u; i < sizeof(CASES) / sizeof(CASES[0]); ++i) {
        switch (CASES[i].record) {
        case 0:
            refused(&paid, WRITE_ORDER, read_order, &read_order_value, CASES[i].from, CASES[i].to, CASES[i].expected);
            break;
        case 1:
            refused(&first_status, WRITE_STATUS, read_status, &read_status_value, CASES[i].from, CASES[i].to,
                    CASES[i].expected);
            break;
        default:
            refused(&extra, WRITE_EXTRAS, read_extras, &read_extras_value, CASES[i].from, CASES[i].to,
                    CASES[i].expected);
            break;
        }
    }
    refused(&paid, WRITE_ORDER, read_order, &read_order_value, "\"full_name\":\"Ada\"", long_name,
            "order_taken.customer.full_name: a text of 25 bytes is past the 24 its type holds");
}

// ── The replay ──────────────────────────────────────────────────────────

// The one trace every test case of the suite below is given.
static char trace_text[4u * MAX_TEXT];

static bool the_trace(void *user_data, const char *test_case, const char **text, size_t *len, sce_saved_error_t *err) {
    (void)user_data;
    (void)test_case;
    (void)err;
    *text = trace_text;
    *len = strlen(trace_text);
    return true;
}

// What the replay reports of its one test case.
typedef struct replayed_s {
    size_t count;
    sce_test_verdict_t verdict;
    sce_test_instant_t ended_at;
    char first_line[SCE_TEST_MAX_LOG_DETAIL];
    char log[4u * MAX_TEXT];
} replayed_t;

static void keep_replayed(void *user_data, const char *test_case, const sce_test_case_result_t *result) {
    replayed_t *replayed = (replayed_t *)user_data;
    replayed->count++;
    replayed->verdict = result->verdict;
    replayed->ended_at = result->ended_at;
    replayed->first_line[0] = '\0';
    if (result->log_count > 0u) {
        (void)snprintf(replayed->first_line, sizeof(replayed->first_line), "%s", result->log[0].detail);
    }
    size_t len = 0u;
    CHECK(sce_test_case_result_canonical_log(result, "forms_suite", test_case, replayed->log, sizeof(replayed->log),
                                             &len) == SCE_TEST_TRACE_WRITTEN,
          "the log of %s is written", test_case);
}

static replays_every_unionless_form_machines_t replay_machines;
static sce_test_case_run_t replay_run;
static sce_test_traces_t replay_traces;
static replayed_t replayed;

// docs/SCE_TTCN3_LANE.md §10.5: a trace of one record of each type the system
// port takes, two at the start and one later, replays to the listener, which
// takes one a round and passes; the log holds each value as the trace does.
static void a_record_a_trace_of_every_form_replays_to_the_values_it_holds(void) {
    static char order_text[MAX_TEXT];
    static char status_text[MAX_TEXT];
    static char extras_text[MAX_TEXT];
    const OrderTakenPayload_t paid = order(VOUCHER, sizeof(VOUCHER));
    const JobStatusPayload_t first_status = status(true);
    const FormExtrasPayload_t first_extras = extras(true);
    written(WRITE_ORDER, &paid, order_text);
    written(WRITE_STATUS, &first_status, status_text);
    written(WRITE_EXTRAS, &first_extras, extras_text);
    (void)snprintf(trace_text, sizeof(trace_text),
                   "{\"inputs\":[{\"at\":\"0\",\"port\":\"sut\",\"type\":\"order_taken\",\"value\":%s},"
                   "{\"at\":\"0\",\"port\":\"sut\",\"type\":\"job_status\",\"value\":%s},"
                   "{\"at\":\"250\",\"port\":\"sut\",\"type\":\"form_extras\",\"value\":%s}],"
                   "\"schema\":\"sce-test-trace.v1\",\"suite\":\"forms_suite\",\"test_case\":\"%s\"}",
                   order_text, status_text, extras_text, REPLAYS_EVERY_UNIONLESS_FORM_NAME);
    sce_test_execute_t execute;
    memset(&execute, 0, sizeof(execute));
    memset(&replay_machines, 0, sizeof(replay_machines));
    execute.test_case = replays_every_unionless_form_test_case(&replay_machines);
    sce_test_suite_t suite;
    memset(&suite, 0, sizeof(suite));
    suite.name = "forms_suite";
    sce_test_module_parameters_init(&suite.module_parameters);
    suite.executes = &execute;
    suite.execute_count = 1u;
    sce_test_features_t world;
    world.names = NULL;
    world.count = 0u;
    sce_test_trace_source_t source;
    source.read = the_trace;
    source.user_data = NULL;
    memset(&replayed, 0, sizeof(replayed));
    CHECK(sce_test_run_suite_with_traces(&suite, &world, source, NULL, &replay_run, &replay_traces, keep_replayed,
                                         &replayed, NULL),
          "the suite runs");
    CHECK(replayed.count == 1u, "%lu test cases ran", (unsigned long)replayed.count);
    CHECK(replayed.verdict == SCE_TEST_VERDICT_PASS, "verdict %s: %s", sce_test_verdict_name(replayed.verdict),
          replayed.first_line);
    CHECK(replayed.ended_at == 250, "ended at %lld", (long long)replayed.ended_at);
    const char *const types[] = {"order_taken", "job_status", "form_extras"};
    const char *const values[] = {order_text, status_text, extras_text};
    for (size_t i = 0u; i < 3u; ++i) {
        static char received[2u * MAX_TEXT];
        (void)snprintf(received, sizeof(received),
                       "\"kind\":\"message-received\",\"port\":\"sut\",\"type\":\"%s\",\"value\":%s", types[i],
                       values[i]);
        CHECK(strstr(replayed.log, received) != NULL, "`%s` in %s", received, replayed.log);
    }
}

int main(void) {
    a_record_reads_back_a_value_of_each_form_from_its_canonical_text();
    a_record_refuses_a_malformed_spelling_of_each_form_by_name();
    a_record_a_trace_of_every_form_replays_to_the_values_it_holds();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)printf("a record reads back the form it writes: all checks passed\n");
    return 0;
}
