// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/SCE_TTCN3_LANE.md §12, §13.1 S2: every trace of a suite of the corpus
// replays, through the suite's runner (SCE::Test::Runner::main, the program of
// §4 run in the process), to its expected log, byte for byte. The suites are
// the headers sce-codegen writes for static_datamodel_generated_cpp_test
// (tests/CMakeLists.txt); runtime_suite's own traces are replayed by
// ATestSuiteRunsEndToEndTest.cpp, beside the rest of what that suite is held to.
//
// The expected logs are the files every backend is held to, the canonical logs
// a reviewed run of the Rust runner wrote. Each test is the twin of the Rust
// case of the same name in
// backends/rust/tests/tests/the_corpus_replays_to_its_expected_logs.rs.

#include <gtest/gtest.h>

#include <filesystem>
#include <sstream>
#include <string>
#include <vector>

#include "boundary_suite.h"
#include "integration/RuntimeSuiteExpectedLogs.h"
#include "job_suite.h"
#include "nested_suite.h"
#include "orders_suite.h"
#include "test/Runner.h"
#include "test/Suite.h"

namespace {

using RuntimeSuiteExpectedLogs::bytesOf;
using RuntimeSuiteExpectedLogs::fileNames;
using RuntimeSuiteExpectedLogs::withoutSuffix;

/// The directory of the runs of @p suite: its `traces/` and `expected_logs/`.
std::filesystem::path runsDir(const std::string &suite) {
    return std::filesystem::path(SCE_PROJECT_ROOT) / "sce-build/tests/fixtures/test_profile/runs" / suite;
}

/// A suite of the corpus and how its runner is run over its traces.
struct Replayed {
    /// The suite's unit name, which names the directory of its runs.
    std::string name;
    /// The suite's table, which the runner runs.
    SCE::Test::Suite suite;
    /// The reader of a module parameter's value from the command line.
    SCE::Test::Runner::ParameterReader parameter;
    /// The world that selects every test case, as `--world` takes it.
    std::string world;
    /// What the runner prints: each test case its world selects, in the
    /// suite's order, with its verdict.
    std::string verdicts;
};

/// Runs the suite's runner over every trace of its directory, in the world
/// that selects every test case, and holds what it prints to the verdicts it
/// expects, every log it writes to its expected log, and the set of logs to
/// the set of expected logs.
void replays(const Replayed &replayed) {
    const std::filesystem::path runs = runsDir(replayed.name);
    const std::filesystem::path logs =
        std::filesystem::temp_directory_path() / ("sce-cpp-corpus-runner-" + replayed.name);
    std::filesystem::remove_all(logs);
    std::ostringstream printed;
    std::ostringstream refused;
    const std::vector<std::string> args{
        "--trace-dir", (runs / "traces").string(), "--log-dir", logs.string(), "--world", replayed.world};
    ASSERT_EQ(SCE::Test::Runner::main(replayed.suite, replayed.parameter, args, printed, refused), 0) << refused.str();
    EXPECT_EQ(printed.str(), replayed.verdicts) << refused.str();
    const std::filesystem::path expectedDir = runs / "expected_logs";
    std::vector<std::string> expected;
    for (const std::string &name : fileNames(expectedDir)) {
        const std::string testCase = withoutSuffix(name, ".json");
        EXPECT_FALSE(testCase.empty()) << "the expected log " << name << " is a .json file";
        expected.push_back(testCase.empty() ? name : testCase);
    }
    std::vector<std::string> written;
    for (const std::string &name : fileNames(logs)) {
        const std::string testCase = withoutSuffix(name, ".log.json");
        written.push_back(testCase.empty() ? name : testCase);
        if (testCase.empty()) {
            ADD_FAILURE() << "the runner wrote " << name << ", which is no log of a case";
            continue;
        }
        const std::filesystem::path expectedPath = expectedDir / (testCase + ".json");
        if (!std::filesystem::exists(expectedPath)) {
            ADD_FAILURE() << "the case " << testCase << " has no expected log " << expectedPath;
            continue;
        }
        std::string want = bytesOf(expectedPath);
        const bool endsInNewline = !want.empty() && want.back() == '\n';
        EXPECT_TRUE(endsInNewline) << "the expected log of " << testCase << " ends in one newline";
        if (endsInNewline) {
            want.pop_back();
        }
        EXPECT_EQ(bytesOf(logs / name), want)
            << "the log of " << testCase << " of " << replayed.name << " is not its expected log";
    }
    EXPECT_EQ(written, expected) << "the runner wrote a log of each case that has an expected log, and of no other";
    std::filesystem::remove_all(logs);
}

/// docs/SCE_TTCN3_LANE.md §12.2: signed and 64-bit integers in a message, the
/// verdicts `fail` and `none`, the two ways to `error` a document leads a run
/// to, a test case that can never end and a dynamic error, and a branch taken
/// before the states it leaves log their exits (docs/SCE_TEST_PROFILE.md §4.4).
TEST(TheCorpus, TheBoundarySuiteReplaysToItsExpectedLogs) {
    replays(Replayed{"boundary_suite", SCE::Generated::BoundarySuite::suite(),
                     &SCE::Generated::BoundarySuite::moduleParameter, "",
                     "exchanges_signed_numbers pass\nanswers_another_job fail\nsets_no_verdict none\n"
                     "never_ends error\nsends_to_a_removed_ptc error\ntakes_before_its_exits pass\n"});
}

/// docs/SCE_TEST_PROFILE.md §11.8: a status every matcher of `every_matcher`
/// accepts, at 1 ms, which the tester's branch reads as a composed message;
/// the helper is told and passes, and the tester settles for the duration its
/// start gives and passes at 501 ms.
TEST(TheCorpus, TheJobSuiteReplaysToItsExpectedLogs) {
    replays(Replayed{"job_suite", SCE::Generated::JobSuite::suite(), &SCE::Generated::JobSuite::moduleParameter,
                     "job-timeout,job-retry", "completes_in_time pass\n"});
}

/// docs/SCE_TEST_PROFILE.md §9.5: an order of two lines, a customer and a card,
/// which `order_placed_match` accepts over its union, its list of records and
/// its template references, at 1 ms.
TEST(TheCorpus, TheOrdersSuiteReplaysToItsExpectedLogs) {
    replays(Replayed{"orders_suite", SCE::Generated::OrdersSuite::suite(),
                     &SCE::Generated::OrdersSuite::moduleParameter, "", "orders_case pass\n"});
}

/// docs/SCE_TEST_PROFILE.md §4.5 (P5), §4.7 (P7): an order at 100 ms, which the
/// atomic state's own targetless branch takes before its ancestor's, and one at
/// 500 ms, which the ancestor's internal branch takes, restarting the running
/// timer; the inner timer then runs out twice and the tester passes at 1250 ms.
TEST(TheCorpus, TheNestedSuiteReplaysToItsExpectedLogs) {
    replays(Replayed{"nested_suite", SCE::Generated::NestedSuite::suite(),
                     &SCE::Generated::NestedSuite::moduleParameter, "", "nested_case pass\n"});
}

}  // namespace
