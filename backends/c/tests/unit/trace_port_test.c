// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The trace port of a C11 machine and the writer of what it tells
// (sce/trace_port.h, sce/trace_json.h; docs/adr/0115, D4 and D8), held to the
// cases of the Rust runtime's trace_port module, so the two lanes write one
// format in one layout:
//
//   * each kind of step writes the fields the run trace format asks of it, in
//     the order the format lists them, and no other;
//   * a run is written with its version and its lane, one step to a line;
//   * a name is written as a JSON string;
//   * a recorder keeps nothing while no step is open, and a step opened anew
//     keeps nothing of the one before;
//   * a step told more names than a list holds is lost, and no writer writes
//     it or a run that holds it;
//   * a run written into a buffer too short for it is measured, and written
//     whole into one of the length it measured.
//
// Header-only code; nothing here allocates.

#include <sce/trace_json.h>
#include <sce/trace_port.h>

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

enum { TEXT_CAP = 1024 };

// A step of `kind` saying every field, as the Rust runtime's test builds one:
// what the kind does not say must not be written.
static const sce_trace_step_t *told_step(sce_trace_recorder_t *recorder, sce_trace_kind_t kind, const char *outcome) {
    sce_trace_begin(recorder, kind, "go.on");
    sce_trace_fired(recorder, "s0", 2u);
    sce_trace_exited(recorder, "s01");
    sce_trace_exited(recorder, "s0");
    sce_trace_entered(recorder, "s1");
    sce_trace_configuration(recorder, "s1");
    sce_trace_outcome(recorder, outcome);
    return sce_trace_close(recorder);
}

// `step` as the writer writes it, terminated, in `out`.
static const char *written_step(const sce_trace_step_t *step, char *out) {
    sce_jw_t w;
    sce_jw_init(&w, out, TEXT_CAP);
    const bool whole = sce_trace_json_step(&w, step);
    CHECK(whole, "the step is written");
    CHECK(sce_jw_finish(&w), "the step fits");
    return out;
}

static void each_kind_writes_the_fields_the_format_asks_of_it(void) {
    sce_trace_recorder_t recorder = {0};
    char text[TEXT_CAP];
    const char *expected = "{\"kind\": \"start\", \"entered\": [\"s1\"], \"configuration\": [\"s1\"]}";
    written_step(told_step(&recorder, SCE_TRACE_START, "pass"), text);
    CHECK(strcmp(text, expected) == 0, "start: %s", text);

    expected = "{\"kind\": \"eventless\", \"fired\": [{\"source\": \"s0\", \"index\": 2}], \"exited\": [\"s01\", "
               "\"s0\"], \"entered\": [\"s1\"], \"configuration\": [\"s1\"]}";
    written_step(told_step(&recorder, SCE_TRACE_EVENTLESS, "pass"), text);
    CHECK(strcmp(text, expected) == 0, "eventless: %s", text);

    expected = "{\"kind\": \"internal\", \"event\": \"go.on\", \"fired\": [{\"source\": \"s0\", \"index\": 2}], "
               "\"exited\": [\"s01\", \"s0\"], \"entered\": [\"s1\"], \"configuration\": [\"s1\"]}";
    written_step(told_step(&recorder, SCE_TRACE_INTERNAL, "pass"), text);
    CHECK(strcmp(text, expected) == 0, "internal: %s", text);

    expected = "{\"kind\": \"external\", \"event\": \"go.on\", \"fired\": [{\"source\": \"s0\", \"index\": 2}], "
               "\"exited\": [\"s01\", \"s0\"], \"entered\": [\"s1\"], \"configuration\": [\"s1\"]}";
    written_step(told_step(&recorder, SCE_TRACE_EXTERNAL, "pass"), text);
    CHECK(strcmp(text, expected) == 0, "external: %s", text);

    expected = "{\"kind\": \"stop\", \"exited\": [\"s01\", \"s0\"], \"outcome\": \"pass\"}";
    written_step(told_step(&recorder, SCE_TRACE_STOP, "pass"), text);
    CHECK(strcmp(text, expected) == 0, "stop: %s", text);

    expected = "{\"kind\": \"stop\", \"exited\": [\"s01\", \"s0\"], \"outcome\": null}";
    written_step(told_step(&recorder, SCE_TRACE_STOP, NULL), text);
    CHECK(strcmp(text, expected) == 0, "a stop that ended in no final: %s", text);
}

static void a_run_is_written_with_its_version_and_lane_one_step_to_a_line(void) {
    sce_trace_recorder_t recorder = {0};
    char text[TEXT_CAP];
    sce_trace_json_run_t run;

    sce_trace_json_run_begin(&run, text, sizeof(text));
    CHECK(sce_trace_json_run_end(&run), "a run of no step is written");
    CHECK(strcmp(text, "{\n  \"version\": 1,\n  \"lane\": \"c11\",\n  \"steps\": []\n}\n") == 0, "no step: %s", text);

    sce_trace_json_run_begin(&run, text, sizeof(text));
    sce_trace_json_run_step(&run, told_step(&recorder, SCE_TRACE_START, "pass"));
    sce_trace_json_run_step(&run, told_step(&recorder, SCE_TRACE_STOP, "pass"));
    CHECK(sce_trace_json_run_end(&run), "a run of two steps is written");
    CHECK(run.steps == 2u, "two steps, not %zu", run.steps);
    CHECK(strcmp(text, "{\n  \"version\": 1,\n  \"lane\": \"c11\",\n  \"steps\": [\n    "
                       "{\"kind\": \"start\", \"entered\": [\"s1\"], \"configuration\": [\"s1\"]},\n    "
                       "{\"kind\": \"stop\", \"exited\": [\"s01\", \"s0\"], \"outcome\": \"pass\"}\n  ]\n}\n") == 0,
          "two steps: %s", text);
}

static void a_name_is_written_as_a_json_string(void) {
    sce_trace_recorder_t recorder = {0};
    char text[TEXT_CAP];
    sce_trace_begin(&recorder, SCE_TRACE_START, NULL);
    sce_trace_entered(&recorder, "a\"b\\c");
    written_step(sce_trace_close(&recorder), text);
    CHECK(strcmp(text, "{\"kind\": \"start\", \"entered\": [\"a\\\"b\\\\c\"], \"configuration\": []}") == 0, "%s",
          text);
}

static void a_recorder_keeps_nothing_while_no_step_is_open(void) {
    sce_trace_recorder_t recorder = {0};
    CHECK(!sce_trace_is_open(&recorder), "a zeroed recorder holds no open step");
    sce_trace_entered(&recorder, "s0");
    sce_trace_fired(&recorder, "s0", 0u);
    CHECK(recorder.step.entered_count == 0u && recorder.step.fired_count == 0u, "nothing is added with no step open");
    CHECK(sce_trace_close(&recorder) == NULL, "no step is closed with none open");

    sce_trace_begin(&recorder, SCE_TRACE_INTERNAL, "e");
    CHECK(sce_trace_is_open(&recorder), "a begun step is open");
    sce_trace_exited(&recorder, "s0");
    sce_trace_entered(&recorder, "s1");
    sce_trace_begin(&recorder, SCE_TRACE_EVENTLESS, NULL);
    CHECK(recorder.step.kind == SCE_TRACE_EVENTLESS && recorder.step.event == NULL, "the step opened anew is its own");
    CHECK(recorder.step.exited_count == 0u && recorder.step.entered_count == 0u,
          "a step opened anew keeps nothing of the one before");
    const sce_trace_step_t *closed = sce_trace_close(&recorder);
    CHECK(closed == &recorder.step, "the closed step is handed back");
    CHECK(!sce_trace_is_open(&recorder), "a closed step is no longer open");
    sce_trace_exited(&recorder, "s1");
    CHECK(recorder.step.exited_count == 0u, "nothing is added once the step is closed");
}

static void a_step_told_more_names_than_a_list_holds_is_lost(void) {
    sce_trace_recorder_t recorder = {0};
    char text[TEXT_CAP];
    sce_jw_t w;

    sce_trace_begin(&recorder, SCE_TRACE_STOP, NULL);
    for (unsigned i = 0u; i < SCE_TRACE_MAX_NAMES; ++i) {
        sce_trace_exited(&recorder, "s");
    }
    CHECK(!recorder.step.lost, "a list holds SCE_TRACE_MAX_NAMES names");
    sce_trace_exited(&recorder, "s");
    CHECK(recorder.step.lost, "a name past the bound loses the step");
    CHECK(recorder.step.exited_count == SCE_TRACE_MAX_NAMES, "the list is not written past its bound");
    const sce_trace_step_t *lost = sce_trace_close(&recorder);
    sce_jw_init(&w, text, sizeof(text));
    CHECK(!sce_trace_json_step(&w, lost), "a lost step is not written");
    CHECK(w.len == 0u, "nothing of a lost step is written, not %zu bytes", w.len);

    sce_trace_json_run_t run;
    sce_trace_json_run_begin(&run, text, sizeof(text));
    sce_trace_json_run_step(&run, lost);
    CHECK(!sce_trace_json_run_end(&run), "a run that holds a lost step is refused");
    CHECK(run.steps == 0u, "the lost step is not counted");

    sce_trace_begin(&recorder, SCE_TRACE_EVENTLESS, NULL);
    for (uint32_t i = 0u; i <= SCE_TRACE_MAX_NAMES; ++i) {
        sce_trace_fired(&recorder, "s", i);
    }
    CHECK(recorder.step.lost, "a transition past the bound loses the step");
    CHECK(recorder.step.fired_count == SCE_TRACE_MAX_NAMES, "the transitions are not written past their bound");
}

static void a_run_too_long_for_its_buffer_is_measured(void) {
    sce_trace_recorder_t recorder = {0};
    char short_text[8];
    char text[TEXT_CAP];
    sce_trace_json_run_t run;

    sce_trace_json_run_begin(&run, short_text, sizeof(short_text));
    sce_trace_json_run_step(&run, told_step(&recorder, SCE_TRACE_START, NULL));
    CHECK(!sce_trace_json_run_end(&run), "a run that does not fit is refused");
    const size_t needed = run.w.len;
    CHECK(short_text[sizeof(short_text) - 1u] == '\0', "the short text is terminated");

    sce_trace_json_run_begin(&run, text, needed + 1u);
    sce_trace_json_run_step(&run, told_step(&recorder, SCE_TRACE_START, NULL));
    CHECK(sce_trace_json_run_end(&run), "the run fits the length it measured");
    CHECK(strlen(text) == needed, "the run is %zu bytes, as measured, not %zu", needed, strlen(text));
}

int main(void) {
    each_kind_writes_the_fields_the_format_asks_of_it();
    a_run_is_written_with_its_version_and_lane_one_step_to_a_line();
    a_name_is_written_as_a_json_string();
    a_recorder_keeps_nothing_while_no_step_is_open();
    a_step_told_more_names_than_a_list_holds_is_lost();
    a_run_too_long_for_its_buffer_is_measured();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)puts("trace port: ok");
    return 0;
}
