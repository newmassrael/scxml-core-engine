// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A receive branch whose content reads a message that holds every part a C11
// unit holds in a type of its own (docs/SCE_TEST_PROFILE.md §12.3): a list of
// byte strings, a list of lists, a list of values that may be absent, a value
// that may be absent of a list, a union, a record within the record and a list
// of records that hold an enum. The machine makes each from the unit's value
// by the functions the generator writes beside its statements
// (`composed_lowering::fields_from_unit`), compiled here under the strict
// flags, and the component reads each back: the trace hands the system port
// one value, the component logs what it read and passes where the union and
// the enum read back as they were handed. The documents are
// sce-build/tests/fixtures/test_profile/received/ (backends/c/tests/CMakeLists.txt).
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

#include "reads_every_part.h"

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

// The most bytes the canonical text of the trace and of the log take.
#define MAX_TEXT 4096u

// The value of received_parts the system port is handed, in its canonical
// text: two byte strings of two octets and one, a list of two lists, a list of
// an absent value and a present one, a list that may be absent present, the
// payment by card, a customer and two lines.
static const char PARTS[] = "{\"blobs\":[\"00AB\",\"FF\"],"
                            "\"customer\":{\"full_name\":\"Ada\",\"vip\":true},"
                            "\"grid\":[[\"1\",\"2\"],[\"3\"]],"
                            "\"lines\":[{\"color\":\"blue\",\"quantity\":\"2\",\"sku\":\"A-1\"},"
                            "{\"color\":\"red\",\"quantity\":\"5\",\"sku\":\"B\"}],"
                            "\"payment\":{\"card\":{\"amount\":\"7\",\"last4\":\"4242\"}},"
                            "\"slots\":[null,\"9\"],"
                            "\"spare\":[\"4\",\"5\"]}";

// The one trace the test case is given.
static char trace_text[MAX_TEXT];

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
    char log[2u * MAX_TEXT];
} replayed_t;

static void keep_replayed(void *user_data, const char *test_case, const sce_test_case_result_t *result) {
    replayed_t *replayed = (replayed_t *)user_data;
    replayed->count++;
    replayed->verdict = result->verdict;
    size_t len = 0u;
    CHECK(sce_test_case_result_canonical_log(result, "parts_suite", test_case, replayed->log, sizeof(replayed->log),
                                             &len) == SCE_TEST_TRACE_WRITTEN,
          "the log of %s is written", test_case);
}

static reads_every_part_machines_t parts_machines;
static sce_test_case_run_t parts_run;
static sce_test_traces_t parts_traces;
static replayed_t parts_replayed;

// docs/SCE_TEST_PROFILE.md §12.3: the message is taken into the machine's
// types whole, and each part reads back as the trace handed it: the lengths of
// the byte strings, the numbers of the list of lists, the present value of the
// list of values that may be absent, the length of the list that may be
// absent, the card's amount, the customer's flag and the lines' quantities in
// the log, the union's case and the first line's enum in the verdict.
static void a_received_message_is_made_in_the_machines_types_and_read_back(void) {
    (void)snprintf(trace_text, sizeof(trace_text),
                   "{\"inputs\":[{\"at\":\"0\",\"port\":\"sut\",\"type\":\"received_parts\",\"value\":%s}],"
                   "\"schema\":\"sce-test-trace.v1\",\"suite\":\"parts_suite\",\"test_case\":\"%s\"}",
                   PARTS, READS_EVERY_PART_NAME);
    sce_test_execute_t execute;
    memset(&execute, 0, sizeof(execute));
    memset(&parts_machines, 0, sizeof(parts_machines));
    execute.test_case = reads_every_part_test_case(&parts_machines);
    sce_test_suite_t suite;
    memset(&suite, 0, sizeof(suite));
    suite.name = "parts_suite";
    sce_test_module_parameters_init(&suite.module_parameters);
    suite.executes = &execute;
    suite.execute_count = 1u;
    sce_test_features_t world;
    world.names = NULL;
    world.count = 0u;
    sce_test_trace_source_t source;
    source.read = the_trace;
    source.user_data = NULL;
    memset(&parts_replayed, 0, sizeof(parts_replayed));
    CHECK(sce_test_run_suite_with_traces(&suite, &world, source, NULL, &parts_run, &parts_traces, keep_replayed,
                                         &parts_replayed, NULL),
          "the suite runs");
    CHECK(parts_replayed.count == 1u, "%lu test cases ran", (unsigned long)parts_replayed.count);
    CHECK(parts_replayed.verdict == SCE_TEST_VERDICT_PASS, "verdict %s: %s",
          sce_test_verdict_name(parts_replayed.verdict), parts_replayed.log);
    static char received[2u * MAX_TEXT];
    (void)snprintf(received, sizeof(received),
                   "\"kind\":\"message-received\",\"port\":\"sut\",\"type\":\"received_parts\",\"value\":%s", PARTS);
    CHECK(strstr(parts_replayed.log, received) != NULL, "the value is read and written back as it was: `%s` in %s",
          received, parts_replayed.log);
    static const char *const READ[] = {
        "\"blob_bytes: 3\"",  "\"cell_sum: 6\"", "\"cell: 3\"",   "\"slot_count: 2\"",   "\"slot_value: 9\"",
        "\"spare_count: 2\"", "\"amount: 7\"",   "\"vip: true\"", "\"quantity_sum: 7\"",
    };
    for (size_t i = 0u; i < sizeof(READ) / sizeof(READ[0]); ++i) {
        CHECK(strstr(parts_replayed.log, READ[i]) != NULL, "`%s` in %s", READ[i], parts_replayed.log);
    }
}

int main(void) {
    a_received_message_is_made_in_the_machines_types_and_read_back();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)printf("a received message is made in the machine's types: all checks passed\n");
    return 0;
}
