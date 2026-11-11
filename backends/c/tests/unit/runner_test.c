// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The runner program of a generated suite in the C11 runtime
// (docs/SCE_TTCN3_LANE.md §4; sce/test/runner.h): the command line every
// backend's runner takes, and the readers of a module parameter's value given
// on it. Each case is the twin of the C++ case of the same subject in
// tests/test_runtime/RunnerTest.cpp, and through it of the Rust runtime's in
// backends/rust/runtime/src/test/runner.rs, with the same lines and the same
// refusals. The program run whole, over a generated suite, is
// integration/test_a_runner_program_writes_the_log_of_each_trace.c.
//
// Header-only code; nothing here allocates, and nothing here runs a suite, so
// the program's directory, which only a run makes, is not linked.

#include <sce/saved_json.h>
#include <sce/saved_values.h>
#include <sce/test/runner.h>

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

static sce_test_runner_options_t the_options;

// The options the `count` texts `line` give, read into `the_options`.
static bool parsed(const char *const *line, size_t count, sce_saved_error_t *err) {
    err->message[0] = '\0';
    return sce_test_runner_options_parse(&the_options, (int)count, line, err);
}

// The command line every backend's runner takes.
static void the_runner_reads_the_command_line_as_every_runner_reads_it(void) {
    sce_saved_error_t err;
    static const char *const FULL[] = {
        "--trace-dir",        "traces", "--log-dir",          "logs",     "--world", "ptcs,guard",
        "--module-parameter", "job=9",  "--module-parameter", "label=a=b"};
    CHECK(parsed(FULL, sizeof(FULL) / sizeof(FULL[0]), &err), "%s", err.message);
    CHECK(strcmp(the_options.trace_dir, "traces") == 0, "trace dir %s", the_options.trace_dir);
    CHECK(strcmp(the_options.log_dir, "logs") == 0, "log dir %s", the_options.log_dir);
    CHECK(the_options.world_count == 2u && strcmp(the_options.world[0], "ptcs") == 0 &&
              strcmp(the_options.world[1], "guard") == 0,
          "the world is ptcs and guard");
    CHECK(the_options.parameter_count == 2u && strcmp(the_options.parameter_ids[0], "job") == 0 &&
              strcmp(the_options.parameter_literals[0], "9") == 0 &&
              strcmp(the_options.parameter_ids[1], "label") == 0 &&
              strcmp(the_options.parameter_literals[1], "a=b") == 0,
          "each module parameter, its id and its literal, in the order given");
    static const char *const BARE[] = {"--log-dir", "l", "--trace-dir", "t"};
    CHECK(parsed(BARE, 4u, &err) && the_options.world_count == 0u, "%s", err.message);
    static const char *const EMPTY_WORLD[] = {"--trace-dir", "t", "--log-dir", "l", "--world", ""};
    CHECK(parsed(EMPTY_WORLD, 6u, &err) && the_options.world_count == 0u, "an empty world names no feature");

    static const char *const NO_TRACE_DIR[] = {"--log-dir", "l"};
    static const char *const NO_LOG_DIR[] = {"--trace-dir", "t"};
    static const char *const TWICE[] = {"--trace-dir", "t", "--log-dir", "l", "--trace-dir", "u"};
    static const char *const UNKNOWN[] = {"--trace-dir", "t", "--log-dir", "l", "--seed", "1"};
    static const char *const NO_VALUE[] = {"--trace-dir"};
    static const char *const NO_EQUALS[] = {"--trace-dir", "t", "--log-dir", "l", "--module-parameter", "job"};
    static const char *const SAME_ID[] = {"--trace-dir",        "t",     "--log-dir",          "l",
                                          "--module-parameter", "job=1", "--module-parameter", "job=2"};

    static const struct {
        const char *const *line;
        size_t count;
        const char *why;
    } REFUSED[] = {
        {NO_TRACE_DIR, 2u, "--trace-dir is not given"},  {NO_LOG_DIR, 2u, "--log-dir is not given"},
        {TWICE, 6u, "--trace-dir is given twice"},       {UNKNOWN, 6u, "--seed is no option"},
        {NO_VALUE, 1u, "--trace-dir is given no value"}, {NO_EQUALS, 6u, "is not <id>=<value>"},
        {SAME_ID, 8u, "'job' is given twice"},
    };

    for (size_t i = 0u; i < sizeof(REFUSED) / sizeof(REFUSED[0]); ++i) {
        CHECK(!parsed(REFUSED[i].line, REFUSED[i].count, &err) && strstr(err.message, REFUSED[i].why) != NULL, "%s: %s",
              REFUSED[i].why, err.message);
    }
}

// The JSON text `writer` writes into `buf`, which holds it.
static const char *saved(void (*writer)(sce_jw_t *, const void *), const void *value, char *buf, size_t cap) {
    sce_jw_t w;
    sce_jw_init(&w, buf, cap);
    writer(&w, value);
    CHECK(sce_jw_finish(&w), "the saved value fits");
    return buf;
}

static void saved_uint32(sce_jw_t *w, const void *value) {
    sce_saved_write_uint32(w, *(const uint32_t *)value);
}

static void saved_uint64(sce_jw_t *w, const void *value) {
    sce_saved_write_uint64(w, *(const uint64_t *)value);
}

static void saved_bytes(sce_jw_t *w, const void *value) {
    static const uint8_t OCTETS[] = {0x0au, 0xffu};
    (void)value;
    sce_saved_write_bytes(w, OCTETS, sizeof(OCTETS));
}

// A module parameter's literal is read as the saved value of its type, and one
// that is no value of it is refused, naming the parameter.
static void the_runner_reads_a_module_parameter_as_the_saved_value_of_its_type(void) {
    char out[64];
    char want[64];
    sce_saved_error_t err;
    err.message[0] = '\0';
    const uint32_t nine = 9u;
    CHECK(sce_test_runner_unsigned_parameter("job", "9", UINT32_MAX, false, out, sizeof(out), &err) &&
              strcmp(out, saved(saved_uint32, &nine, want, sizeof(want))) == 0,
          "job is %s: %s", out, err.message);
    const uint64_t widest = UINT64_MAX;
    CHECK(
        sce_test_runner_unsigned_parameter("wide", "18446744073709551615", UINT64_MAX, true, out, sizeof(out), &err) &&
            strcmp(out, saved(saved_uint64, &widest, want, sizeof(want))) == 0,
        "wide is %s: %s", out, err.message);
    CHECK(sce_test_runner_signed_parameter("offset", "-5", INT8_MIN, INT8_MAX, false, out, sizeof(out), &err) &&
              strcmp(out, "-5") == 0,
          "offset is %s: %s", out, err.message);
    CHECK(!sce_test_runner_unsigned_parameter("job", "256", UINT8_MAX, false, out, sizeof(out), &err) &&
              strncmp(err.message, "job: ", 5u) == 0,
          "%s", err.message);
    CHECK(!sce_test_runner_unsigned_parameter("job", "09", UINT32_MAX, false, out, sizeof(out), &err) &&
              err.message[0] != '\0',
          "a leading zero is refused");
    CHECK(sce_test_runner_bool_parameter("armed", "true", out, sizeof(out), &err) && strcmp(out, "true") == 0, "%s",
          err.message);
    err.message[0] = '\0';
    CHECK(!sce_test_runner_bool_parameter("armed", "yes", out, sizeof(out), &err) && err.message[0] != '\0',
          "yes is no bool");
    CHECK(sce_test_runner_text_parameter("label", "nightly", out, sizeof(out), &err) && strcmp(out, "\"nightly\"") == 0,
          "%s", out);
    CHECK(sce_test_runner_enum_parameter("color", "green", out, sizeof(out), &err) && strcmp(out, "\"green\"") == 0,
          "%s", out);
    CHECK(sce_test_runner_bytes_parameter("key", "0aFF", out, sizeof(out), &err) &&
              strcmp(out, saved(saved_bytes, NULL, want, sizeof(want))) == 0,
          "key is %s: %s", out, err.message);
    static const char *const NOT_HEX[] = {"0a0", "0g", "+a"};
    for (size_t i = 0u; i < sizeof(NOT_HEX) / sizeof(NOT_HEX[0]); ++i) {
        err.message[0] = '\0';
        CHECK(!sce_test_runner_bytes_parameter("key", NOT_HEX[i], out, sizeof(out), &err) && err.message[0] != '\0',
              "%s is no octets", NOT_HEX[i]);
    }
    CHECK(!sce_test_runner_undeclared_parameter("limit", &err) &&
              strcmp(err.message, "the suite declares no module parameter 'limit'") == 0,
          "%s", err.message);
    char small[2];
    CHECK(!sce_test_runner_text_parameter("label", "nightly", small, sizeof(small), &err) &&
              strncmp(err.message, "label: ", 7u) == 0,
          "a value past its buffer is refused, naming its parameter: %s", err.message);
}

int main(void) {
    the_runner_reads_the_command_line_as_every_runner_reads_it();
    the_runner_reads_a_module_parameter_as_the_saved_value_of_its_type();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)puts("runner: ok");
    return 0;
}
