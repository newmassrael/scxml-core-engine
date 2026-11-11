// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The runner program of the generated suite `runtime_suite`
// (docs/SCE_TTCN3_LANE.md §4), run twice over: in this process, through
// `sce_test_runner_run` on a command line given here with its two streams kept,
// and as the downstream's lane runs it, a process, given the directory of the
// suite's committed traces and a directory for its logs, whose standard output,
// standard error, exit status and log files are read. The program is
// integration/runtime_suite_runner.c, whose path the build gives as
// SCE_RUNTIME_SUITE_RUNNER. The twin of the C++ `TheSuitesRunnerWritesTheLogOfEachTrace`
// (tests/integration/ATestSuiteRunsEndToEndTest.cpp) and
// ARunnerProgramWritesTheLogOfEachTraceTest.cpp, and through them of the Rust
// `the_suites_runner_program_writes_the_log_of_each_trace`
// (backends/rust/tests/tests/a_test_suite_runs_through_its_runner.rs).
//
// A hosted POSIX program: it starts a process and waits for it, names its
// scratch directories by its process id, and makes them through the runtime's
// `sce_directory_make`, which the POSIX tier it links implements.

#define _POSIX_C_SOURCE 200809L

#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>

#include <sce/directory.h>
#include <sce/test/runner.h>

#include "runtime_suite.h"

// After the runtime's headers, one of which names a function-pointer parameter
// `read`, which `-Wshadow` would read as hiding the `read` this declares.
#include <unistd.h>

#ifndef SCE_PROJECT_ROOT
#error "SCE_PROJECT_ROOT names the repository's root, which the traces and the shared log are read from"
#endif

#ifndef SCE_RUNTIME_SUITE_RUNNER
#error "the build gives the path of the runner program as SCE_RUNTIME_SUITE_RUNNER"
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

// The most bytes a path, a command line or a text read here takes: a scratch
// directory's path, its directory of logs, and a log's path in that, each with
// room for the one it is made of and what it adds.
#define PATH_SIZE 1024u
#define LOGS_SIZE (PATH_SIZE + 16u)
#define LOG_PATH_SIZE (LOGS_SIZE + 64u)
#define COMMAND_SIZE 4096u
#define TEXT_SIZE 8192u

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

// The test log every runtime writes of `answers_the_job`, without the one newline
// its file ends in, which no canonical text holds.
static void shared_log(char *out) {
    char path[PATH_SIZE];
    (void)snprintf(path, sizeof(path),
                   "%s/sce-build/tests/fixtures/test_profile/runtime/expected_logs/answers_the_job.json",
                   SCE_PROJECT_ROOT);
    const size_t len = file_text(path, out);
    CHECK(len > 0u && out[len - 1u] == '\n', "the shared log ends in one newline");
    if (len > 0u && out[len - 1u] == '\n') {
        out[len - 1u] = '\0';
    }
}

// The directory of the suite's committed traces.
static void traces_dir(char *out) {
    (void)snprintf(out, PATH_SIZE, "%s/sce-build/tests/fixtures/test_profile/runtime/traces", SCE_PROJECT_ROOT);
}

// Whether anything is at `path`.
static bool exists(const char *path) {
    struct stat held;
    return stat(path, &held) == 0;
}

// The directory of this run of the test that `name` keeps apart from every
// other, in `out`.
static void scratch_path(const char *name, char *out) {
    const char *tmp = getenv("TMPDIR");
    (void)snprintf(out, PATH_SIZE, "%s/sce-c-runner-program-%ld-%s", tmp != NULL && tmp[0] != '\0' ? tmp : "/tmp",
                   (long)getpid(), name);
}

// What a test leaves in its scratch directory `dir`, removed: the log of the
// suite's one unguarded case, the directory of the logs, the streams of a run,
// and the directory itself.
static void scratch_removed(const char *dir) {
    static const char *const LEFT[] = {"logs/answers_the_job.log.json", "logs", "out", "err"};
    char path[PATH_SIZE];
    for (size_t i = 0u; i < sizeof(LEFT) / sizeof(LEFT[0]); ++i) {
        (void)snprintf(path, sizeof(path), "%s/%s", dir, LEFT[i]);
        (void)remove(path);
    }
    (void)remove(dir);
}

// The scratch directory `name` names, made empty, in `out`.
static void scratch(const char *name, char *out) {
    scratch_path(name, out);
    scratch_removed(out);
    CHECK(sce_directory_make(out) == 0, "%s is made", out);
}

// `text` in single quotes, as the shell reads one word, after the `*len` bytes of
// the COMMAND_SIZE at `command`.
static void quoted(char *command, size_t *len, const char *text) {
    const size_t room = COMMAND_SIZE;
    if (*len + 1u < room) {
        command[(*len)++] = '\'';
    }
    for (const char *c = text; *c != '\0'; ++c) {
        const char *spelled = *c == '\'' ? "'\\''" : NULL;
        if (spelled != NULL) {
            for (const char *s = spelled; *s != '\0' && *len + 1u < room; ++s) {
                command[(*len)++] = *s;
            }
        } else if (*len + 1u < room) {
            command[(*len)++] = *c;
        }
    }
    if (*len + 1u < room) {
        command[(*len)++] = '\'';
    }
    command[*len] = '\0';
}

// `text` as it is, after the `*len` bytes at `command`.
static void plain(char *command, size_t *len, const char *text) {
    for (const char *c = text; *c != '\0' && *len + 1u < COMMAND_SIZE; ++c) {
        command[(*len)++] = *c;
    }
    command[*len] = '\0';
}

// What a run of the program left: its exit status, its standard output and its
// standard error.
typedef struct program_run_s {
    int status;
    char out[TEXT_SIZE];
    char err[TEXT_SIZE];
} program_run_t;

// The runner program, given the directories of the traces and of the logs, then
// the `count` texts `args`, its streams written to files of `streams`.
static void run_program(const char *logs, const char *streams, const char *const *args, size_t count,
                        program_run_t *run) {
    static char command[COMMAND_SIZE];
    char traces[PATH_SIZE];
    char out_path[PATH_SIZE];
    char err_path[PATH_SIZE];
    size_t len = 0u;
    traces_dir(traces);
    (void)snprintf(out_path, sizeof(out_path), "%s/out", streams);
    (void)snprintf(err_path, sizeof(err_path), "%s/err", streams);
    command[0] = '\0';
    quoted(command, &len, SCE_RUNTIME_SUITE_RUNNER);
    plain(command, &len, " --trace-dir ");
    quoted(command, &len, traces);
    plain(command, &len, " --log-dir ");
    quoted(command, &len, logs);
    for (size_t i = 0u; i < count; ++i) {
        plain(command, &len, " ");
        quoted(command, &len, args[i]);
    }
    plain(command, &len, " >");
    quoted(command, &len, out_path);
    plain(command, &len, " 2>");
    quoted(command, &len, err_path);
    const int waited = system(command);
    run->status = (waited != -1 && WIFEXITED(waited)) ? WEXITSTATUS(waited) : -1;
    (void)file_text(out_path, run->out);
    (void)file_text(err_path, run->err);
}

// Whether no line of `err` is one of the lines of `out`: the standard error may
// hold the engine's diagnostics, whatever they say, and none is the standard
// output's.
static bool no_line_of_err_in_out(const char *err, const char *out) {
    char line[PATH_SIZE];
    const char *start = err;
    while (*start != '\0') {
        const char *end = strchr(start, '\n');
        const size_t n = end != NULL ? (size_t)(end - start) : strlen(start);
        if (n > 0u && n + 2u < sizeof(line)) {
            memcpy(line, start, n);
            line[n] = '\n';
            line[n + 1u] = '\0';
            if (strstr(out, line) != NULL) {
                return false;
            }
        }
        if (end == NULL) {
            break;
        }
        start = end + 1;
    }
    return true;
}

static char want[TEXT_SIZE];
static char log_text[TEXT_SIZE];

// The message the run of `answers_the_job` sends when the job given is 9.
static const char JOB_NINE[] =
    "\"kind\":\"message-sent\",\"port\":\"sut\",\"type\":\"job_request\",\"value\":{\"job\":\"9\"}";

// docs/SCE_TTCN3_LANE.md §4: the suite's runner program runs each test case its
// world selects against its trace and writes `<case>.log.json`, the canonical
// log with nothing after it, and prints each case's verdict; a module parameter's
// value on its command line replaces its default, and one the suite does not
// declare is refused with exit status 2 before any case runs.
static void a_runner_program_writes_the_log_of_each_trace(void) {
    static program_run_t run;
    char streams[PATH_SIZE];
    char dir[PATH_SIZE];
    char logs[LOGS_SIZE];
    char path[LOG_PATH_SIZE];
    shared_log(want);
    scratch("streams", streams);

    scratch("default", dir);
    (void)snprintf(logs, sizeof(logs), "%s/logs", dir);
    run_program(logs, streams, NULL, 0u, &run);
    CHECK(run.status == 0, "exit status %d: %s", run.status, run.err);
    CHECK(strcmp(run.out, "answers_the_job pass\n") == 0, "the standard output is the contract's lines alone: %s",
          run.out);
    CHECK(no_line_of_err_in_out(run.err, run.out), "no diagnostic is on the standard output");
    (void)snprintf(path, sizeof(path), "%s/answers_the_job.log.json", logs);
    (void)file_text(path, log_text);
    CHECK(strcmp(log_text, want) == 0, "the log is the shared one:\n  wrote %s\n  want  %s", log_text, want);
    scratch_removed(dir);

    static const char *const OVERRIDDEN[] = {"--world", "", "--module-parameter", "job=9"};
    scratch("overridden", dir);
    (void)snprintf(logs, sizeof(logs), "%s/logs", dir);
    run_program(logs, streams, OVERRIDDEN, 4u, &run);
    CHECK(run.status == 0, "exit status %d: %s", run.status, run.err);
    (void)snprintf(path, sizeof(path), "%s/answers_the_job.log.json", logs);
    (void)file_text(path, log_text);
    CHECK(strstr(log_text, JOB_NINE) != NULL, "the request names the job of the value given: %s", log_text);
    scratch_removed(dir);

    static const char *const UNDECLARED[] = {"--module-parameter", "limit=1"};
    scratch("undeclared", dir);
    (void)snprintf(logs, sizeof(logs), "%s/logs", dir);
    run_program(logs, streams, UNDECLARED, 2u, &run);
    CHECK(run.status == 2, "exit status %d", run.status);
    CHECK(strstr(run.err, "the suite declares no module parameter 'limit'") != NULL, "%s", run.err);
    CHECK(!exists(logs), "no case ran, so no log was written");
    scratch_removed(dir);
    scratch_removed(streams);
}

static runtime_suite_storage_t in_process_storage;
static sce_test_runner_t in_process_runner;

// The runner run in this process on the command line `args` after the trace and
// log directories, its streams kept in `*run`.
static void run_in_process(const char *logs, const char *const *args, size_t count, program_run_t *run) {
    char traces[PATH_SIZE];
    const char *line[8];
    size_t n = 0u;
    traces_dir(traces);
    line[n++] = "--trace-dir";
    line[n++] = traces;
    line[n++] = "--log-dir";
    line[n++] = logs;
    for (size_t i = 0u; i < count && n < sizeof(line) / sizeof(line[0]); ++i) {
        line[n++] = args[i];
    }
    FILE *out = tmpfile();
    FILE *err = tmpfile();
    CHECK(out != NULL && err != NULL, "the streams are made");
    if (out == NULL || err == NULL) {
        run->status = -1;
        return;
    }
    run->status = sce_test_runner_run(&in_process_runner, runtime_suite_suite(&in_process_storage),
                                      runtime_suite_module_parameter, (int)n, line, out, err);
    stream_text(out, run->out);
    stream_text(err, run->err);
    (void)fclose(out);
    (void)fclose(err);
}

// docs/SCE_TTCN3_LANE.md §4: the suite's runner, run in this process with its
// streams given, does what the program does: the twin of the C++
// `TheSuitesRunnerWritesTheLogOfEachTrace`.
static void the_suites_runner_writes_the_log_of_each_trace(void) {
    static program_run_t run;
    char dir[PATH_SIZE];
    char logs[LOGS_SIZE];
    char path[LOG_PATH_SIZE];
    shared_log(want);

    scratch("in-process-default", dir);
    (void)snprintf(logs, sizeof(logs), "%s/logs", dir);
    run_in_process(logs, NULL, 0u, &run);
    CHECK(run.status == 0, "exit status %d: %s", run.status, run.err);
    CHECK(strcmp(run.out, "answers_the_job pass\n") == 0, "%s", run.out);
    (void)snprintf(path, sizeof(path), "%s/answers_the_job.log.json", logs);
    (void)file_text(path, log_text);
    CHECK(strcmp(log_text, want) == 0, "the log is the shared one:\n  wrote %s\n  want  %s", log_text, want);
    scratch_removed(dir);

    static const char *const OVERRIDDEN[] = {"--world", "", "--module-parameter", "job=9"};
    scratch("in-process-overridden", dir);
    (void)snprintf(logs, sizeof(logs), "%s/logs", dir);
    run_in_process(logs, OVERRIDDEN, 4u, &run);
    CHECK(run.status == 0, "exit status %d: %s", run.status, run.err);
    (void)snprintf(path, sizeof(path), "%s/answers_the_job.log.json", logs);
    (void)file_text(path, log_text);
    CHECK(strstr(log_text, JOB_NINE) != NULL, "the request names the job of the value given: %s", log_text);
    scratch_removed(dir);

    static const char *const UNDECLARED[] = {"--module-parameter", "limit=1"};
    scratch("in-process-undeclared", dir);
    (void)snprintf(logs, sizeof(logs), "%s/logs", dir);
    run_in_process(logs, UNDECLARED, 2u, &run);
    CHECK(run.status == 2, "exit status %d", run.status);
    CHECK(strstr(run.err, "the suite declares no module parameter 'limit'") != NULL, "%s", run.err);
    CHECK(!exists(logs), "no case ran, so no log was written");
    scratch_removed(dir);
}

int main(void) {
    the_suites_runner_writes_the_log_of_each_trace();
    a_runner_program_writes_the_log_of_each_trace();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)printf("a runner program writes the log of each trace: all checks passed\n");
    return 0;
}
