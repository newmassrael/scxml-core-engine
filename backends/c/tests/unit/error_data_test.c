// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The `_event.data` of an `error.*` event a C11 machine raises for itself
// (docs/adr/0106 #111): the record SCE_MESH.md section 10.7.1 pins, in the
// bytes every engine writes. The twins are the Rust
// `event::tests::a_communication_code_names_its_event_and_its_detail_is_escaped`,
// the Python `test_error_data.py` and the C++ `SCE::Common::errorData`.
//
//   * a code's value range names its event, `execution` below 64 and
//     `communication` from it;
//   * the record names the code by the vocabulary's declared name, and the
//     detail is escaped as every engine escapes a JSON string;
//   * a writer over no buffer measures the text a second call writes, so the
//     script engine's binding sizes its string with the first;
//   * no code of the vocabulary is the value an event that carries none holds.
//
// Header-only code; nothing here allocates.

#include <sce/error_data.h>

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

// The record of `reason` and `detail` must be `want`, and a writer over no
// buffer must measure exactly its length.
static void records(ErrorReason_t reason, const char *detail, const char *want) {
    char text[256];
    sce_jw_t w;
    sce_jw_init(&w, NULL, 0u);
    sce_error_data_write(&w, reason, detail);
    const size_t measured = w.len;
    sce_jw_init(&w, text, sizeof(text));
    sce_error_data_write(&w, reason, detail);
    CHECK(sce_jw_finish(&w), "the record of '%s' fits %lu bytes", detail != NULL ? detail : "(null)",
          (unsigned long)sizeof(text));
    CHECK(strcmp(text, want) == 0, "the record is '%s', not '%s'", text, want);
    CHECK(measured == strlen(want), "the writer over no buffer measured %lu bytes, not %lu", (unsigned long)measured,
          (unsigned long)strlen(want));
}

static void a_code_names_the_event_its_value_range_does(void) {
    CHECK(strcmp(sce_error_name(ERROR_REASON_EXPRESSION_FAILED), "execution") == 0,
          "expressionFailed is raised as error.execution");
    CHECK(strcmp(sce_error_name(ERROR_REASON_PAYLOAD_REFUSED), "execution") == 0,
          "payloadRefused is raised as error.execution");
    CHECK(strcmp(sce_error_name(ERROR_REASON_CAPACITY_EXCEEDED), "execution") == 0,
          "capacityExceeded is raised as error.execution");
    CHECK(strcmp(sce_error_name(ERROR_REASON_TARGET_UNREACHABLE), "communication") == 0,
          "targetUnreachable is raised as error.communication");
    CHECK(strcmp(sce_error_name(ERROR_REASON_CAUSAL_VIEW_MISMATCH), "communication") == 0,
          "causalViewMismatch is raised as error.communication");
}

static void the_record_names_the_code_and_escapes_the_detail(void) {
    records(ERROR_REASON_ASSIGNMENT_FAILED, "<assign> to 'x' failed",
            "{\"errorName\":\"execution\",\"reason\":\"assignmentFailed\",\"detail\":\"<assign> to 'x' failed\"}");
    records(ERROR_REASON_TARGET_UNREACHABLE, "say \"hi\"\n",
            "{\"errorName\":\"communication\",\"reason\":\"targetUnreachable\",\"detail\":\"say \\\"hi\\\"\\n\"}");
    records(ERROR_REASON_SEND_DELAY_INVALID, NULL,
            "{\"errorName\":\"execution\",\"reason\":\"sendDelayInvalid\",\"detail\":\"\"}");
}

static void no_code_is_the_value_of_an_event_that_carries_none(void) {
    ErrorReason_t reason = ERROR_REASON_EXPRESSION_FAILED;
    CHECK(!error_reason_from_underlying((uint8_t)SCE_ERROR_REASON_NONE, &reason),
          "the value an event that carries no code holds is no code of the vocabulary");
    CHECK(error_reason_from_underlying(error_reason_to_underlying(ERROR_REASON_PAYLOAD_REFUSED), &reason) &&
              reason == ERROR_REASON_PAYLOAD_REFUSED,
          "the byte an event carries reads back as the code it was raised for");
}

int main(void) {
    a_code_names_the_event_its_value_range_does();
    the_record_names_the_code_and_escapes_the_detail();
    no_code_is_the_value_of_an_event_that_carries_none();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)puts("error data: ok");
    return 0;
}
