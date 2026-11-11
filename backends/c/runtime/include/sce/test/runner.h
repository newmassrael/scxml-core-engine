/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/**
 * @file runner.h
 * @brief The runner program of a generated suite (docs/SCE_TTCN3_LANE.md §4,
 *        §10.5): it runs every test case a world selects against its recorded
 *        input trace and writes each one's canonical test log
 *
 * The unit of a suite generates the program's `main` (`<suite>_main`), which
 * hands `sce_test_runner_main` the suite's tables and the reader of its module
 * parameters (`<suite>_module_parameter`); a build makes it a program with a
 * one-line `main` that calls it. The command line is the same on every backend:
 *
 *   <runner> --trace-dir <dir> --log-dir <dir> [--world <feature>,...] [--module-parameter <id>=<value>]...
 *
 *   - `--trace-dir`: the directory of the traces, `<test case>.json` each
 *     (`schemas/sce-test-trace.v1.schema.json`);
 *   - `--log-dir`: the directory the logs are written to, `<test case>.log.json`
 *     each, the canonical test log's text with nothing after it
 *     (`schemas/sce-test-log.v1.schema.json`), made where it is missing;
 *   - `--world`: the features the world enables, by name, separated by commas;
 *     none where it is not given (ADR 0057 §2);
 *   - `--module-parameter`: a module parameter's value in place of its default,
 *     written as `<sce:module-parameter value>` writes a literal of its type
 *     (docs/SCE_TEST_PROFILE.md §11.4, §11.8); repeated, one per parameter.
 *
 * Each test case run prints `<test case> <verdict>` on its own line, and nothing
 * else is printed on the standard output: every diagnostic goes to the standard
 * error, where a generated machine writes its own already. A test case whose
 * trace is missing or refused ends with `error` and its log is written, the
 * reason on the standard error (P13). The exit status is 0 when every selected
 * test case ran and its log was written, whatever its verdict: the verdict is
 * the log's. It is 2 when the command line is not one of the above, a module
 * parameter's value is refused, or a log cannot be written. The Rust sibling is
 * `sce_rust_runtime::test::runner`, whose command line, output and refusals
 * these are, word for word; the C11 port of `sce/include/test/Runner.h`.
 *
 * What is C's own: the runner holds everything it reads and writes in one
 * struct the program places (`sce_test_runner_t`), each table bounded at compile
 * time and each bound refused by name: a trace's text
 * (`SCE_TEST_INPUT_TRACE_TEXT_SIZE`), a log's (`SCE_TEST_RUNNER_LOG_SIZE`), a
 * path (`SCE_TEST_RUNNER_PATH_SIZE`), a module parameter's value
 * (`SCE_TEST_RUNNER_PARAMETER_SIZE`) and the features of a world
 * (`SCE_TEST_RUNNER_MAX_FEATURES`). A log is written as its test case ends,
 * since a result lives only for the call that reports it, so the log directory is
 * made once the command line and its module parameters are read and before any
 * case runs. The directory is made through `sce_directory_make` (sce/directory.h),
 * which the program links from its platform tier; the files are ISO C's.
 * Header-only and allocation-free.
 */

#ifndef SCE_TEST_RUNNER_H
#define SCE_TEST_RUNNER_H

#include <errno.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "sce/directory.h"
#include "sce/saved_json.h"
#include "sce/saved_values.h"
#include "sce/test/from_canonical.h"
#include "sce/test/input_trace.h"
#include "sce/test/module_parameters.h"
#include "sce/test/suite.h"
#include "sce/test/verdict.h"
#include "sce/types.h"

#ifdef __cplusplus
extern "C" {
#endif

/** The program's usage line. */
#define SCE_TEST_RUNNER_USAGE                                                                                          \
    "usage: <runner> --trace-dir <dir> --log-dir <dir> [--world <feature>,...] [--module-parameter <id>=<value>]..."

/* How many features one `--world` names. */
#ifndef SCE_TEST_RUNNER_MAX_FEATURES
#define SCE_TEST_RUNNER_MAX_FEATURES 16u
#endif

/* How many bytes the value of `--world` takes. */
#ifndef SCE_TEST_RUNNER_WORLD_SIZE
#define SCE_TEST_RUNNER_WORLD_SIZE 256u
#endif

/* How many bytes the saved value of one module parameter takes, as the JSON text
   the runtime holds it in (sce/test/module_parameters.h), its terminator
   included. */
#ifndef SCE_TEST_RUNNER_PARAMETER_SIZE
#define SCE_TEST_RUNNER_PARAMETER_SIZE 256u
#endif

/* How many bytes the path of one trace or one log takes, its terminator
   included. */
#ifndef SCE_TEST_RUNNER_PATH_SIZE
#define SCE_TEST_RUNNER_PATH_SIZE 1024u
#endif

/* How many bytes the canonical text of one test case's log takes, its
   terminator included: a log holds at most SCE_TEST_MAX_TRACE_RECORDS records
   and SCE_TEST_TRACE_TEXT_SIZE bytes of their texts. */
#ifndef SCE_TEST_RUNNER_LOG_SIZE
#define SCE_TEST_RUNNER_LOG_SIZE 16384u
#endif

/**
 * The reader of a suite's module parameters, generated as
 * `<suite>_module_parameter`: the saved value of the module parameter `id` of its
 * declared type, read from `literal`, the value the command line gives it, as the
 * compact JSON text the runtime holds a value in, into the `cap` bytes at `out`;
 * false, naming why in `*err`, where it is not one.
 */
typedef bool (*sce_test_parameter_reader_fn)(const char *id, const char *literal, char *out, size_t cap,
                                             sce_saved_error_t *err);

/** What the command line asks of the runner. Its texts are the command line's,
 *  lent, but for the world and the ids of the module parameters, which it holds. */
typedef struct sce_test_runner_options_s {
    /** The directory of the traces, `<test case>.json` each. */
    const char *trace_dir;
    /** The directory the logs are written to, `<test case>.log.json` each. */
    const char *log_dir;
    /** The features the world enables, each a text of `world_text`. */
    char world_text[SCE_TEST_RUNNER_WORLD_SIZE];
    const char *world[SCE_TEST_RUNNER_MAX_FEATURES];
    size_t world_count;
    /** Each `--module-parameter`, its id and the literal it gives, in the order
        given. */
    char parameter_ids[SCE_TEST_MAX_MODULE_PARAMETERS][SCE_MAX_ID_LEN];
    const char *parameter_literals[SCE_TEST_MAX_MODULE_PARAMETERS];
    size_t parameter_count;
} sce_test_runner_options_t;

/**
 * The options the `argc` texts `args` give, the program's name not among them,
 * in `*options`; refused, naming why in `*err`, where they are not the program's
 * command line.
 */
static inline bool sce_test_runner_options_parse(sce_test_runner_options_t *options, int argc, const char *const *args,
                                                 sce_saved_error_t *err) {
    const size_t count = argc > 0 ? (size_t)argc : 0u;
    const char *world = NULL;
    memset(options, 0, sizeof(*options));
    /* Every option takes one value: the line is read a pair at a time. */
    for (size_t at = 0u; at < count; at += 2u) {
        const char *option = args[at];
        if (at + 1u == count) {
            return sce_saved_refuse(err, "%s is given no value", option);
        }
        const char *value = args[at + 1u];
        if (strcmp(option, "--trace-dir") == 0 && options->trace_dir == NULL) {
            options->trace_dir = value;
        } else if (strcmp(option, "--log-dir") == 0 && options->log_dir == NULL) {
            options->log_dir = value;
        } else if (strcmp(option, "--world") == 0 && world == NULL) {
            world = value;
        } else if (strcmp(option, "--module-parameter") == 0) {
            const char *equals = strchr(value, '=');
            if (equals == NULL) {
                return sce_saved_refuse(err, "--module-parameter %s is not <id>=<value>", value);
            }
            const size_t id_len = (size_t)(equals - value);
            if (id_len >= SCE_MAX_ID_LEN) {
                return sce_saved_refuse(err,
                                        "--module-parameter %s names an id past the %u bytes the runner holds "
                                        "(SCE_MAX_ID_LEN)",
                                        value, (unsigned)(SCE_MAX_ID_LEN - 1));
            }
            char id[SCE_MAX_ID_LEN];
            memcpy(id, value, id_len);
            id[id_len] = '\0';
            for (size_t i = 0u; i < options->parameter_count; ++i) {
                if (strcmp(options->parameter_ids[i], id) == 0) {
                    return sce_saved_refuse(err, "the module parameter '%s' is given twice", id);
                }
            }
            if (options->parameter_count >= SCE_TEST_MAX_MODULE_PARAMETERS) {
                return sce_saved_refuse(err,
                                        "--module-parameter is given more than the %u times the runner holds "
                                        "(SCE_TEST_MAX_MODULE_PARAMETERS)",
                                        (unsigned)SCE_TEST_MAX_MODULE_PARAMETERS);
            }
            memcpy(options->parameter_ids[options->parameter_count], id, id_len + 1u);
            options->parameter_literals[options->parameter_count] = equals + 1;
            options->parameter_count++;
        } else if (strcmp(option, "--trace-dir") == 0 || strcmp(option, "--log-dir") == 0 ||
                   strcmp(option, "--world") == 0) {
            return sce_saved_refuse(err, "%s is given twice", option);
        } else {
            return sce_saved_refuse(err, "%s is no option of the runner", option);
        }
    }
    if (options->trace_dir == NULL) {
        return sce_saved_refuse(err, "--trace-dir is not given");
    }
    if (options->log_dir == NULL) {
        return sce_saved_refuse(err, "--log-dir is not given");
    }
    if (world != NULL) {
        const size_t len = strlen(world);
        if (len >= sizeof(options->world_text)) {
            return sce_saved_refuse(err, "--world is past the %u bytes the runner holds (SCE_TEST_RUNNER_WORLD_SIZE)",
                                    (unsigned)(SCE_TEST_RUNNER_WORLD_SIZE - 1u));
        }
        memcpy(options->world_text, world, len + 1u);
        /* Each feature between two commas; an empty one names none. */
        char *feature = options->world_text;
        for (size_t i = 0u; i <= len; ++i) {
            if (options->world_text[i] != ',' && options->world_text[i] != '\0') {
                continue;
            }
            options->world_text[i] = '\0';
            if (feature[0] != '\0') {
                if (options->world_count >= SCE_TEST_RUNNER_MAX_FEATURES) {
                    return sce_saved_refuse(err,
                                            "--world names more than the %u features the runner holds "
                                            "(SCE_TEST_RUNNER_MAX_FEATURES)",
                                            (unsigned)SCE_TEST_RUNNER_MAX_FEATURES);
                }
                options->world[options->world_count++] = feature;
            }
            feature = &options->world_text[i + 1u];
        }
    }
    return true;
}

/* The saved value `*w` wrote of the module parameter `id`, which must have fit. */
static inline bool sce_test_runner_written_(sce_jw_t *w, const char *id, sce_saved_error_t *err) {
    if (!sce_jw_finish(w)) {
        return sce_saved_refuse(err, "%s: the value's text of %lu bytes is past the %lu its buffer holds", id,
                                (unsigned long)w->len, (unsigned long)(w->cap > 0u ? w->cap - 1u : 0u));
    }
    return true;
}

/** The refusal of a value for the module parameter `id`, which the suite does
 *  not declare. Always false. */
static inline bool sce_test_runner_undeclared_parameter(const char *id, sce_saved_error_t *err) {
    return sce_saved_refuse(err, "the suite declares no module parameter '%s'", id);
}

/**
 * A value of the unsigned integer module parameter `id`, whose type's greatest
 * value is `most`: the text of its decimal digits, as the canonical form writes
 * one, saved as its type is (sce/saved_values.h): a number, or the text of its
 * digits for a 64-bit type (`wide`), which JSON's numbers cannot all hold.
 */
static inline bool sce_test_runner_unsigned_parameter(const char *id, const char *literal, uint64_t most, bool wide,
                                                      char *out, size_t cap, sce_saved_error_t *err) {
    uint64_t value = 0u;
    if (!sce_test_integer_text_unsigned(literal, strlen(literal), most, id, &value, err)) {
        return false;
    }
    sce_jw_t w;
    sce_jw_init(&w, out, cap);
    if (wide) {
        sce_saved_write_uint64(&w, value);
    } else {
        sce_jw_uint(&w, value);
    }
    return sce_test_runner_written_(&w, id, err);
}

/** A value of the signed integer module parameter `id`, of the range `least` to
 *  `most`: `sce_test_runner_unsigned_parameter` of a signed type. */
static inline bool sce_test_runner_signed_parameter(const char *id, const char *literal, int64_t least, int64_t most,
                                                    bool wide, char *out, size_t cap, sce_saved_error_t *err) {
    int64_t value = 0;
    if (!sce_test_integer_text_signed(literal, strlen(literal), least, most, id, &value, err)) {
        return false;
    }
    sce_jw_t w;
    sce_jw_init(&w, out, cap);
    if (wide) {
        sce_saved_write_int64(&w, value);
    } else {
        sce_jw_int(&w, value);
    }
    return sce_test_runner_written_(&w, id, err);
}

/** A value of the `bool` module parameter `id`: `true` or `false`. */
static inline bool sce_test_runner_bool_parameter(const char *id, const char *literal, char *out, size_t cap,
                                                  sce_saved_error_t *err) {
    sce_jw_t w;
    sce_jw_init(&w, out, cap);
    if (strcmp(literal, "true") == 0) {
        sce_saved_write_bool(&w, true);
    } else if (strcmp(literal, "false") == 0) {
        sce_saved_write_bool(&w, false);
    } else {
        return sce_saved_refuse(err, "%s: '%s' is neither true nor false", id, literal);
    }
    return sce_test_runner_written_(&w, id, err);
}

/** A value of the text module parameter `id`: the text itself. Its bound, like
 *  every value's type, is held where the component reads it, as for a default. */
static inline bool sce_test_runner_text_parameter(const char *id, const char *literal, char *out, size_t cap,
                                                  sce_saved_error_t *err) {
    sce_jw_t w;
    sce_jw_init(&w, out, cap);
    sce_saved_write_string(&w, literal);
    return sce_test_runner_written_(&w, id, err);
}

/** A value of the enum module parameter `id`: the name its document declares for
 *  the variant, which is how a value of an enum is saved. A name its enum does
 *  not declare is refused where the component reads it, as for a default. */
static inline bool sce_test_runner_enum_parameter(const char *id, const char *literal, char *out, size_t cap,
                                                  sce_saved_error_t *err) {
    sce_jw_t w;
    sce_jw_init(&w, out, cap);
    sce_jw_string(&w, literal);
    return sce_test_runner_written_(&w, id, err);
}

/* The value of one hex digit of either case, or -1. */
static inline int sce_test_runner_hex_digit_(char digit) {
    if (digit >= '0' && digit <= '9') {
        return digit - '0';
    }
    if (digit >= 'a' && digit <= 'f') {
        return digit - 'a' + 10;
    }
    if (digit >= 'A' && digit <= 'F') {
        return digit - 'A' + 10;
    }
    return -1;
}

/** A value of the byte string module parameter `id`: its octets in hex, two
 *  digits each, as a literal of docs/SCE_TEST_PROFILE.md §11.4 writes them,
 *  saved as the Latin-1 text a byte string is saved as. */
static inline bool sce_test_runner_bytes_parameter(const char *id, const char *literal, char *out, size_t cap,
                                                   sce_saved_error_t *err) {
    uint8_t octets[SCE_TEST_RUNNER_PARAMETER_SIZE];
    const size_t len = strlen(literal);
    if (len % 2u != 0u) {
        return sce_saved_refuse(err, "%s: '%s' is not octets in hex", id, literal);
    }
    if (len / 2u > sizeof(octets)) {
        return sce_saved_refuse(err, "%s: '%s' is past the %lu octets the runner holds", id, literal,
                                (unsigned long)sizeof(octets));
    }
    for (size_t at = 0u; at < len; at += 2u) {
        const int high = sce_test_runner_hex_digit_(literal[at]);
        const int low = sce_test_runner_hex_digit_(literal[at + 1u]);
        if (high < 0 || low < 0) {
            return sce_saved_refuse(err, "%s: '%s' is not octets in hex", id, literal);
        }
        octets[at / 2u] = (uint8_t)(high * 16 + low);
    }
    sce_jw_t w;
    sce_jw_init(&w, out, cap);
    sce_saved_write_bytes(&w, octets, len / 2u);
    return sce_test_runner_written_(&w, id, err);
}

/** The traces of one directory, `<test case>.json` each, and the text of the one
 *  last read. */
typedef struct sce_test_trace_directory_s {
    const char *dir;
    char text[SCE_TEST_INPUT_TRACE_TEXT_SIZE];
} sce_test_trace_directory_t;

/**
 * The text of the trace of the test case `test_case` in the directory `dir`,
 * `<dir>/<test case>.json`, into the `cap` bytes at `buf`, its length in `*len`;
 * refused, naming why, where it is not read, or is past what `buf` holds.
 */
static inline bool sce_test_runner_read_trace(const char *dir, const char *test_case, char *buf, size_t cap,
                                              size_t *len, sce_saved_error_t *err) {
    char path[SCE_TEST_RUNNER_PATH_SIZE];
    const int n = snprintf(path, sizeof(path), "%s/%s.json", dir, test_case);
    if (n < 0 || (size_t)n >= sizeof(path)) {
        return sce_saved_refuse(err,
                                "the path of the trace of '%s' is past the %u bytes the runner holds "
                                "(SCE_TEST_RUNNER_PATH_SIZE)",
                                test_case, (unsigned)(SCE_TEST_RUNNER_PATH_SIZE - 1u));
    }
    FILE *file = fopen(path, "rb");
    if (file == NULL) {
        return sce_saved_refuse(err, "%s is not read: %s", path, strerror(errno));
    }
    const size_t taken = fread(buf, 1u, cap, file);
    const bool failed = ferror(file) != 0;
    const int cause = errno;
    (void)fclose(file);
    if (failed) {
        return sce_saved_refuse(err, "%s is not read: %s", path, strerror(cause));
    }
    if (taken >= cap) {
        return sce_saved_refuse(err, "%s is past the %lu bytes a trace's text holds (SCE_TEST_INPUT_TRACE_TEXT_SIZE)",
                                path, (unsigned long)(cap - 1u));
    }
    *len = taken;
    return true;
}

static inline bool sce_test_trace_directory_read_(void *user_data, const char *test_case, const char **text,
                                                  size_t *len, sce_saved_error_t *err) {
    sce_test_trace_directory_t *directory = (sce_test_trace_directory_t *)user_data;
    if (!sce_test_runner_read_trace(directory->dir, test_case, directory->text, sizeof(directory->text), len, err)) {
        return false;
    }
    *text = directory->text;
    return true;
}

/** The source of the traces of the directory `dir`, read into `*directory`
 *  (`sce_test_runner_read_trace`), for `sce_test_run_suite_with_traces`. */
static inline sce_test_trace_source_t sce_test_trace_directory(sce_test_trace_directory_t *directory, const char *dir) {
    sce_test_trace_source_t source;
    directory->dir = dir;
    source.read = sce_test_trace_directory_read_;
    source.user_data = directory;
    return source;
}

/**
 * Everything the runner reads and writes while it runs one suite: the command
 * line, the module parameters' values, the traces, the run, and the text of the
 * log being written. Placed by the program, static storage on a host whose stack
 * is small, and set up by `sce_test_runner_run`.
 */
typedef struct sce_test_runner_s {
    sce_test_runner_options_t options;
    char parameter_texts[SCE_TEST_MAX_MODULE_PARAMETERS][SCE_TEST_RUNNER_PARAMETER_SIZE];
    sce_test_module_parameters_t overrides;
    sce_test_trace_directory_t traces;
    sce_test_traces_t played;
    sce_test_case_run_t run;
    char log[SCE_TEST_RUNNER_LOG_SIZE];
    /** The suite's name, which each log names. */
    const char *suite;
    FILE *out;
    FILE *err;
    /** A log was not written: no further case is reported, and the exit status
        is 2. */
    bool failed;
} sce_test_runner_t;

/* The log of `test_case`, which `result` holds, written to `<log dir>/<test
   case>.log.json`, and its verdict printed; the standard error names each error
   its run logged. Nothing once a log was not written. */
static inline void sce_test_runner_report_(void *user_data, const char *test_case,
                                           const sce_test_case_result_t *result) {
    sce_test_runner_t *runner = (sce_test_runner_t *)user_data;
    if (runner->failed) {
        return;
    }
    for (size_t i = 0u; i < result->log_count; ++i) {
        const sce_test_log_record_t *line = &result->log[i];
        if (line->kind == SCE_TEST_LOG_ERROR) {
            (void)fprintf(runner->err, "%s: %s: %s\n", test_case, line->component, line->detail);
        }
    }
    if (result->log_lost > 0u) {
        (void)fprintf(runner->err,
                      "%s: %lu line(s) of its log are past what the run holds (SCE_TEST_MAX_LOG_RECORDS, "
                      "SCE_TEST_MAX_LOG_DETAIL)\n",
                      test_case, (unsigned long)result->log_lost);
    }
    char path[SCE_TEST_RUNNER_PATH_SIZE];
    const int n = snprintf(path, sizeof(path), "%s/%s.log.json", runner->options.log_dir, test_case);
    if (n < 0 || (size_t)n >= sizeof(path)) {
        (void)fprintf(runner->err, "the path of the log of '%s' is past the %u bytes the runner holds\n", test_case,
                      (unsigned)(SCE_TEST_RUNNER_PATH_SIZE - 1u));
        runner->failed = true;
        return;
    }
    size_t len = 0u;
    switch (
        sce_test_case_result_canonical_log(result, runner->suite, test_case, runner->log, sizeof(runner->log), &len)) {
    case SCE_TEST_TRACE_WRITTEN:
        break;
    case SCE_TEST_TRACE_TOO_SMALL:
        (void)fprintf(runner->err, "%s is not written: its log of %lu bytes is past SCE_TEST_RUNNER_LOG_SIZE (%u)\n",
                      path, (unsigned long)len, (unsigned)SCE_TEST_RUNNER_LOG_SIZE);
        runner->failed = true;
        return;
    case SCE_TEST_TRACE_INCOMPLETE:
        (void)fprintf(runner->err, "%s is not written: the run lost a record of its log past the run's bounds\n", path);
        runner->failed = true;
        return;
    case SCE_TEST_TRACE_NOT_CANONICAL:
        (void)fprintf(runner->err,
                      "%s is not written: a value of its log is not in the JSON Canonicalization "
                      "Scheme\n",
                      path);
        runner->failed = true;
        return;
    }
    FILE *file = fopen(path, "wb");
    bool written = file != NULL && fwrite(runner->log, 1u, len, file) == len;
    if (file != NULL && fclose(file) != 0) {
        written = false;
    }
    if (!written) {
        (void)fprintf(runner->err, "%s is not written\n", path);
        runner->failed = true;
        return;
    }
    (void)fprintf(runner->out, "%s %s\n", test_case, sce_test_verdict_name(result->verdict));
}

/* The command line is not the program's: why, then the usage line. */
static inline int sce_test_runner_usage_(FILE *err, const char *why) {
    (void)fprintf(err, "%s\n%s\n", why, SCE_TEST_RUNNER_USAGE);
    return 2;
}

/**
 * The runner program of `*suite`, its module parameters read by `parameter`, on
 * the `argc` texts `args` of its command line, the program's name not among them
 * (docs/SCE_TTCN3_LANE.md §4), in `*runner`, which prints each test case's
 * verdict on `out` and each reason on `err`: see the header's description.
 * Returns the exit status.
 */
static inline int sce_test_runner_run(sce_test_runner_t *runner, const sce_test_suite_t *suite,
                                      sce_test_parameter_reader_fn parameter, int argc, const char *const *args,
                                      FILE *out, FILE *err) {
    sce_saved_error_t why;
    why.message[0] = '\0';
    memset(runner, 0, sizeof(*runner));
    if (!sce_test_runner_options_parse(&runner->options, argc, args, &why)) {
        return sce_test_runner_usage_(err, why.message);
    }
    sce_test_module_parameters_init(&runner->overrides);
    for (size_t i = 0u; i < runner->options.parameter_count; ++i) {
        const char *id = runner->options.parameter_ids[i];
        const char *literal = runner->options.parameter_literals[i];
        if (!parameter(id, literal, runner->parameter_texts[i], sizeof(runner->parameter_texts[i]), &why)) {
            (void)fprintf(err, "--module-parameter %s=%s: %s\n%s\n", id, literal, why.message, SCE_TEST_RUNNER_USAGE);
            return 2;
        }
        (void)sce_test_module_parameters_set(&runner->overrides, id, runner->parameter_texts[i]);
    }
    /* Every override is one the suite declares before the log directory is made,
       so a refused command line leaves nothing behind. */
    sce_test_module_parameters_t checked;
    sce_test_override_refusal_t refusal;
    refusal.reason[0] = '\0';
    if (!sce_test_suite_parameters(suite, &runner->overrides, &checked, &refusal)) {
        return sce_test_runner_usage_(err, refusal.reason);
    }
    const int made = sce_directory_make(runner->options.log_dir);
    if (made != 0) {
        (void)fprintf(err, "%s is not made: %s\n", runner->options.log_dir, strerror(made));
        return 2;
    }
    runner->suite = suite->name;
    runner->out = out;
    runner->err = err;
    sce_test_features_t world;
    world.names = runner->options.world;
    world.count = runner->options.world_count;
    if (!sce_test_run_suite_with_traces(
            suite, &world, sce_test_trace_directory(&runner->traces, runner->options.trace_dir), &runner->overrides,
            &runner->run, &runner->played, sce_test_runner_report_, runner, &refusal)) {
        return sce_test_runner_usage_(err, refusal.reason);
    }
    (void)fflush(out);
    return runner->failed ? 2 : 0;
}

/**
 * The runner program of `*suite`, its module parameters read by `parameter`, on
 * the command line of a program's `main`: `sce_test_runner_run`, which prints on
 * the standard output and the standard error, in `*runner`, which the program
 * places.
 */
static inline int sce_test_runner_main(sce_test_runner_t *runner, const sce_test_suite_t *suite,
                                       sce_test_parameter_reader_fn parameter, int argc, char **argv) {
    const int status = sce_test_runner_run(runner, suite, parameter, argc > 0 ? argc - 1 : 0,
                                           (const char *const *)(argc > 0 ? argv + 1 : argv), stdout, stderr);
    (void)fflush(stdout);
    (void)fflush(stderr);
    return status;
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_TEST_RUNNER_H */
