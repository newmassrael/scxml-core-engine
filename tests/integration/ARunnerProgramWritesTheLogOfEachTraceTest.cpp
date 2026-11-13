// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The runner program of the generated suite `runtime_suite`
// (docs/SCE_TTCN3_LANE.md §4), run as the downstream's lane runs it: a
// process, given the directory of the suite's committed traces and a
// directory for its logs, whose standard output, standard error, exit status
// and log files are read. The program is tests/integration/runtime_suite_runner.cpp,
// whose path the build gives as SCE_RUNTIME_SUITE_RUNNER. The twin of the Rust
// `the_suites_runner_program_writes_the_log_of_each_trace`
// (backends/rust/tests/tests/a_test_suite_runs_through_its_runner.rs).

#include <gtest/gtest.h>

#include <sys/wait.h>
#include <unistd.h>

#include <cstdlib>
#include <filesystem>
#include <sstream>
#include <string>
#include <vector>

#include "integration/RuntimeSuiteExpectedLogs.h"

#ifndef SCE_RUNTIME_SUITE_RUNNER
#error "the build gives the path of the runner program as SCE_RUNTIME_SUITE_RUNNER"
#endif

namespace {

using RuntimeSuiteExpectedLogs::bytesOf;

/// @p text in single quotes, as the shell reads one word.
std::string quoted(const std::string &text) {
    std::string out = "'";
    for (const char c : text) {
        if (c == '\'') {
            out += "'\\''";
        } else {
            out += c;
        }
    }
    return out + "'";
}

/// What a run of the program left: its exit status, its standard output and
/// its standard error.
struct ProgramRun {
    int status = -1;
    std::string out;
    std::string err;
};

/// The directory of this run of the test that @p name keeps apart from every
/// other.
std::filesystem::path scratchPath(const std::string &name) {
    return std::filesystem::temp_directory_path() /
           ("sce-cpp-runner-program-" + std::to_string(::getpid()) + "-" + name);
}

/// The directory scratchPath() names, made empty.
std::filesystem::path scratch(const std::string &name) {
    const std::filesystem::path dir = scratchPath(name);
    std::filesystem::remove_all(dir);
    std::filesystem::create_directories(dir);
    return dir;
}

/// The runner program, given the directories of the traces and of the logs,
/// then @p args.
ProgramRun runProgram(const std::filesystem::path &logs, const std::filesystem::path &streams,
                      const std::vector<std::string> &args) {
    std::string command = quoted(SCE_RUNTIME_SUITE_RUNNER) + " --trace-dir " +
                          quoted(RuntimeSuiteExpectedLogs::tracesDir().string()) + " --log-dir " +
                          quoted(logs.string());
    for (const std::string &arg : args) {
        command += " " + quoted(arg);
    }
    const std::filesystem::path out = streams / "out";
    const std::filesystem::path err = streams / "err";
    command += " >" + quoted(out.string()) + " 2>" + quoted(err.string());
    const int waited = std::system(command.c_str());
    ProgramRun run;
    run.status = (waited != -1 && WIFEXITED(waited)) ? WEXITSTATUS(waited) : -1;
    run.out = bytesOf(out);
    run.err = bytesOf(err);
    return run;
}

/// docs/SCE_TTCN3_LANE.md §4: the suite's runner program runs each test case
/// its world selects against its trace and writes `<case>.log.json`, the
/// canonical log with nothing after it, and prints each case's verdict. In the
/// world of every feature it replays every trace of the directory, and each
/// log it writes is its case's expected log (§12, §13.1 S2). A module
/// parameter's value on its command line replaces its default, and one the
/// suite does not declare is refused with exit status 2 before any case runs.
TEST(ARunnerProgram, WritesTheLogOfEachTrace) {
    const std::filesystem::path streams = scratch("streams");

    const std::filesystem::path logs = scratch("every") / "logs";
    const ProgramRun first = runProgram(logs, streams, {"--world", RuntimeSuiteExpectedLogs::kEveryFeature});
    ASSERT_EQ(first.status, 0) << first.err;
    EXPECT_EQ(first.out, RuntimeSuiteExpectedLogs::kEveryVerdict)
        << "the standard output is the contract's lines alone";
    // The standard error may hold the engine's diagnostics, whatever they say:
    // none of its lines is one of the standard output's.
    std::istringstream diagnostics(first.err);
    for (std::string line; std::getline(diagnostics, line);) {
        if (!line.empty()) {
            EXPECT_EQ(first.out.find(line + "\n"), std::string::npos) << line;
        }
    }
    RuntimeSuiteExpectedLogs::expectEveryLogIsItsExpectedLog(logs);

    const std::filesystem::path overridden = scratch("overridden") / "logs";
    const ProgramRun second = runProgram(overridden, streams, {"--world", "", "--module-parameter", "job=9"});
    ASSERT_EQ(second.status, 0) << second.err;
    const std::string log = bytesOf(overridden / "answers_the_job.log.json");
    EXPECT_NE(log.find("\"kind\":\"message-sent\",\"port\":\"sut\",\"type\":\"job_request\",\"value\":{\"job\":\"9\"}"),
              std::string::npos)
        << "the request names the job of the value given: " << log;

    const std::filesystem::path undeclared = scratch("undeclared") / "logs";
    const ProgramRun third = runProgram(undeclared, streams, {"--module-parameter", "limit=1"});
    EXPECT_EQ(third.status, 2);
    EXPECT_NE(third.err.find("the suite declares no module parameter 'limit'"), std::string::npos) << third.err;
    EXPECT_FALSE(std::filesystem::exists(undeclared)) << "no case ran, so no log was written";

    for (const std::string name : {"streams", "every", "overridden", "undeclared"}) {
        std::filesystem::remove_all(scratchPath(name));
    }
}

}  // namespace
