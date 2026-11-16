// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/SCE_TTCN3_LANE.md §12, §13.1 S2: every trace of a suite of the corpus
// replays, through the suite's runner (`sce_test_runner_run`, the program of §4
// run in this process), to its expected log, byte for byte. The suite is the
// header sce-codegen writes from its document (backends/c/tests/CMakeLists.txt);
// runtime_suite's own traces are replayed by
// test_a_runner_program_writes_the_log_of_each_trace.c, beside the rest of what
// that suite is held to.
//
// The expected logs are the files every backend is held to, the canonical logs a
// reviewed run of the Rust runner wrote. Each case is the twin of the Rust case
// of the same name in
// backends/rust/tests/tests/the_corpus_replays_to_its_expected_logs.rs.
// orders_suite and nested_suite are not replayed here: their mapped record holds
// a union, which C11's test profile matches no value of
// (codegen_matrix::TEST_PROFILE_UNION_BACKENDS).

#include <dirent.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>

#include <sce/directory.h>
#include <sce/test/runner.h>

#include "boundary_suite.h"
#include "job_suite.h"

// After the runtime's headers, one of which names a function-pointer parameter
// `read`, which `-Wshadow` would read as hiding the `read` this declares.
#include <unistd.h>

#ifndef SCE_PROJECT_ROOT
#error "SCE_PROJECT_ROOT names the repository's root, which the traces and the expected logs are read from"
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

// The most bytes a path or a text read here takes: a scratch directory's path,
// its directory of logs, and a file's path in a directory of logs or of expected
// logs, which has room for the name of any entry of it, whose `d_name` holds 256
// bytes. A directory of logs or of expected logs holds at most NAMES_MAX files,
// each named in fewer than NAME_SIZE bytes.
#define PATH_SIZE 1024u
#define LOGS_SIZE (PATH_SIZE + 16u)
#define ENTRY_PATH_SIZE (LOGS_SIZE + 320u)
#define TEXT_SIZE 8192u
#define NAMES_MAX 16u
#define NAME_SIZE 128u

// The text of the file at `path` into the TEXT_SIZE bytes at `out`, terminated;
// empty where there is none.
static size_t file_text(const char *path, char *out) {
    out[0] = '\0';
    FILE *file = fopen(path, "rb");
    if (file == NULL) {
        return 0u;
    }
    const size_t len = fread(out, 1u, TEXT_SIZE - 1u, file);
    (void)fclose(file);
    out[len] = '\0';
    return len;
}

// The text of the stream `file`, written so far, into the TEXT_SIZE bytes at
// `out`, terminated.
static void stream_text(FILE *file, char *out) {
    (void)fflush(file);
    rewind(file);
    const size_t len = fread(out, 1u, TEXT_SIZE - 1u, file);
    out[len] = '\0';
}

// The directory `kind` (`traces` or `expected_logs`) of the runs of `suite`.
static void runs_dir(const char *suite, const char *kind, char *out) {
    (void)snprintf(out, PATH_SIZE, "%s/sce-build/tests/fixtures/test_profile/runs/%s/%s", SCE_PROJECT_ROOT, suite,
                   kind);
}

// Whether anything is at `path`.
static bool exists(const char *path) {
    struct stat held;
    return stat(path, &held) == 0;
}

// The names of the files of a directory, each without the suffix it is read by,
// sorted.
typedef struct names_s {
    size_t count;
    char name[NAMES_MAX][NAME_SIZE];
} names_t;

static int name_order(const void *a, const void *b) {
    return strcmp((const char *)a, (const char *)b);
}

// The name of every file of `dir`, without `suffix`, into `*out`, sorted; a name
// that does not end in `suffix` fails the test and is kept whole, so the sets of
// names differ.
static void file_names(const char *dir, const char *suffix, names_t *out) {
    out->count = 0u;
    DIR *opened = opendir(dir);
    if (opened == NULL) {
        CHECK(false, "the directory %s is read", dir);
        return;
    }
    const size_t suffix_len = strlen(suffix);
    for (struct dirent *entry = readdir(opened); entry != NULL; entry = readdir(opened)) {
        const char *name = entry->d_name;
        if (strcmp(name, ".") == 0 || strcmp(name, "..") == 0) {
            continue;
        }
        const size_t len = strlen(name);
        if (out->count >= NAMES_MAX || len >= NAME_SIZE) {
            CHECK(false, "%s holds at most %u files, each named in fewer than %u bytes: %s", dir, NAMES_MAX, NAME_SIZE,
                  name);
            continue;
        }
        const bool suffixed = len > suffix_len && strcmp(name + len - suffix_len, suffix) == 0;
        CHECK(suffixed, "%s/%s ends in %s", dir, name, suffix);
        const size_t kept = suffixed ? len - suffix_len : len;
        memcpy(out->name[out->count], name, kept);
        out->name[out->count][kept] = '\0';
        ++out->count;
    }
    (void)closedir(opened);
    qsort(out->name, out->count, sizeof(out->name[0]), name_order);
}

// The scratch directory of the logs of `suite`, made where it is missing, in
// `out`, its earlier logs removed.
static void scratch(const char *suite, char *out) {
    const char *tmp = getenv("TMPDIR");
    char path[ENTRY_PATH_SIZE];
    (void)snprintf(out, PATH_SIZE, "%s/sce-c-corpus-runner-%ld-%s", tmp != NULL && tmp[0] != '\0' ? tmp : "/tmp",
                   (long)getpid(), suite);
    DIR *opened = opendir(out);
    if (opened != NULL) {
        for (struct dirent *entry = readdir(opened); entry != NULL; entry = readdir(opened)) {
            if (strcmp(entry->d_name, ".") != 0 && strcmp(entry->d_name, "..") != 0) {
                (void)snprintf(path, sizeof(path), "%s/%s", out, entry->d_name);
                (void)remove(path);
            }
        }
        (void)closedir(opened);
    }
    CHECK(sce_directory_make(out) == 0, "%s is made", out);
}

static char run_out[TEXT_SIZE];
static char run_err[TEXT_SIZE];
static char want[TEXT_SIZE];
static char log_text[TEXT_SIZE];

// Every log the runner wrote into `logs`, `<case>.log.json`, is the expected log
// of its case in `expected_dir`, without the one newline the expected log's file
// ends in, byte for byte; and the runner wrote a log for every expected log and
// nothing else.
static void every_log_is_its_expected_log(const char *suite, const char *expected_dir, const char *logs) {
    static names_t expected;
    static names_t written;
    char path[ENTRY_PATH_SIZE];
    file_names(expected_dir, ".json", &expected);
    file_names(logs, ".log.json", &written);
    for (size_t i = 0u; i < written.count; ++i) {
        const char *test_case = written.name[i];
        (void)snprintf(path, sizeof(path), "%s/%s.json", expected_dir, test_case);
        if (!exists(path)) {
            CHECK(false, "the case %s has no expected log %s", test_case, path);
            continue;
        }
        const size_t want_len = file_text(path, want);
        CHECK(want_len + 1u < TEXT_SIZE, "the expected log of %s is read whole in %u bytes", test_case, TEXT_SIZE);
        CHECK(want_len > 0u && want[want_len - 1u] == '\n', "the expected log of %s ends in one newline", test_case);
        if (want_len > 0u && want[want_len - 1u] == '\n') {
            want[want_len - 1u] = '\0';
        }
        (void)snprintf(path, sizeof(path), "%s/%s.log.json", logs, test_case);
        const size_t len = file_text(path, log_text);
        CHECK(len + 1u < TEXT_SIZE, "the log of %s is read whole in %u bytes", test_case, TEXT_SIZE);
        CHECK(strcmp(log_text, want) == 0, "the log of %s of %s is not its expected log:\n  wrote %s\n  want  %s",
              test_case, suite, log_text, want);
        (void)remove(path);
    }
    bool same = expected.count == written.count;
    for (size_t i = 0u; same && i < expected.count; ++i) {
        same = strcmp(expected.name[i], written.name[i]) == 0;
    }
    CHECK(same,
          "the runner wrote a log of each case that has an expected log, and of no other: %zu written, %zu expected",
          written.count, expected.count);
}

// The runner of every suite below, one at a time.
static sce_test_runner_t runner;

// Runs the runner of the suite `name`, whose tables `suite` are and whose
// module parameters `parameter` reads, over every trace of its directory in
// the world `world`, which selects every test case, and holds what it prints to
// `verdicts`, every log it writes to its expected log, and the set of logs to
// the set of expected logs.
static void replays(const char *name, const sce_test_suite_t *suite, sce_test_parameter_reader_fn parameter,
                    const char *world, const char *verdicts) {
    char traces[PATH_SIZE];
    char expected_dir[PATH_SIZE];
    char dir[PATH_SIZE];
    char logs[LOGS_SIZE];
    runs_dir(name, "traces", traces);
    runs_dir(name, "expected_logs", expected_dir);
    scratch(name, dir);
    (void)snprintf(logs, sizeof(logs), "%s/logs", dir);
    const char *line[] = {"--trace-dir", traces, "--log-dir", logs, "--world", world};
    FILE *out = tmpfile();
    FILE *err = tmpfile();
    CHECK(out != NULL && err != NULL, "the streams are made");
    if (out == NULL || err == NULL) {
        return;
    }
    const int status =
        sce_test_runner_run(&runner, suite, parameter, (int)(sizeof(line) / sizeof(line[0])), line, out, err);
    stream_text(out, run_out);
    stream_text(err, run_err);
    (void)fclose(out);
    (void)fclose(err);
    CHECK(status == 0, "%s: exit status %d: %s", name, status, run_err);
    CHECK(strcmp(run_out, verdicts) == 0, "%s: the standard output is %s", name, run_out);
    every_log_is_its_expected_log(name, expected_dir, logs);
    (void)remove(logs);
    (void)remove(dir);
}

static boundary_suite_storage_t boundary_storage;
static job_suite_storage_t job_storage;

// docs/SCE_TTCN3_LANE.md §12.2: signed and 64-bit integers in a message, the
// verdicts `fail` and `none`, the two ways to `error` a document leads a run to,
// a test case that can never end and a dynamic error, and a branch taken before
// the states it leaves log their exits (docs/SCE_TEST_PROFILE.md §4.4).
static void the_boundary_suite_replays_to_its_expected_logs(void) {
    replays("boundary_suite", boundary_suite_suite(&boundary_storage), boundary_suite_module_parameter, "",
            "exchanges_signed_numbers pass\nanswers_another_job fail\nsets_no_verdict none\n"
            "never_ends error\nsends_to_a_removed_ptc error\ntakes_before_its_exits pass\n");
}

// docs/SCE_TEST_PROFILE.md §11.8: a status every matcher of `every_matcher`
// accepts, at 1 ms, which the tester's branch reads as a composed message; the
// helper is told and passes, and the tester settles for the duration its start
// gives and passes at 501 ms.
static void the_job_suite_replays_to_its_expected_logs(void) {
    replays("job_suite", job_suite_suite(&job_storage), job_suite_module_parameter, "job-timeout,job-retry",
            "completes_in_time pass\n");
}

int main(void) {
    the_boundary_suite_replays_to_its_expected_logs();
    the_job_suite_replays_to_its_expected_logs();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)printf("a corpus suite replays to its expected logs: all checks passed\n");
    return 0;
}
