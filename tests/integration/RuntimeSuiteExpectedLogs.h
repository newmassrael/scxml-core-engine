// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The committed traces of the generated suite `runtime_suite` and the expected
// log of each (docs/SCE_TTCN3_LANE.md §12, §13.1 S2), as the C++ runner's tests
// read them: the runner run in the process (ATestSuiteRunsEndToEndTest.cpp) and
// the runner program run as a process
// (ARunnerProgramWritesTheLogOfEachTraceTest.cpp). Both run the suite in the
// world of every feature, so every trace of the directory is replayed, and hold
// each log written to its case's expected log byte for byte. The rule is the
// Rust `assert_every_log_is_its_expected_log`
// (backends/rust/tests/tests/a_test_suite_runs_through_its_runner.rs).

#pragma once

#include <gtest/gtest.h>

#include <algorithm>
#include <filesystem>
#include <fstream>
#include <sstream>
#include <string>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#error "SCE_PROJECT_ROOT names the repository's root, which the traces and the expected logs are read from"
#endif

namespace RuntimeSuiteExpectedLogs {

/// The world that selects every test case of the suite, so that the runner
/// replays every trace of the directory.
inline constexpr const char *kEveryFeature = "ptcs,removal,guard";

/// The lines the runner prints in the world of every feature: each case of
/// the suite, in its order, with the verdict it ends with.
inline constexpr const char *kEveryVerdict =
    "answers_the_job pass\nserves_a_ptc pass\nkeeps_a_ptc inconc\nanswers_before_the_guard error\n";

/// The directory of the suite's committed fixtures of a run.
inline std::filesystem::path runtimeDir() {
    return std::filesystem::path(SCE_PROJECT_ROOT) / "sce-build/tests/fixtures/test_profile/runtime";
}

/// The directory of the suite's committed traces, `<test case>.json` each.
inline std::filesystem::path tracesDir() {
    return runtimeDir() / "traces";
}

/// The directory of the expected log of each committed trace, `<test
/// case>.json` each: the canonical log a reviewed run wrote of it, with one
/// newline after it.
inline std::filesystem::path expectedLogsDir() {
    return runtimeDir() / "expected_logs";
}

/// The bytes of the file at @p path, or empty where there is none.
inline std::string bytesOf(const std::filesystem::path &path) {
    std::ifstream in(path, std::ios::binary);
    std::ostringstream buffer;
    buffer << in.rdbuf();
    return buffer.str();
}

/// The name of every file in @p dir, sorted.
inline std::vector<std::string> fileNames(const std::filesystem::path &dir) {
    std::vector<std::string> names;
    for (const std::filesystem::directory_entry &entry : std::filesystem::directory_iterator(dir)) {
        names.push_back(entry.path().filename().string());
    }
    std::sort(names.begin(), names.end());
    return names;
}

/// @p name without @p suffix, or empty where it does not end in it.
inline std::string withoutSuffix(const std::string &name, const std::string &suffix) {
    if (name.size() <= suffix.size() || name.compare(name.size() - suffix.size(), suffix.size(), suffix) != 0) {
        return {};
    }
    return name.substr(0, name.size() - suffix.size());
}

/// Every log the runner wrote into @p logs, `<case>.log.json`, is the expected
/// log of its case byte for byte, without the one newline the expected log's
/// file ends in, which no canonical text holds; and the runner wrote a log for
/// every expected log and nothing else.
inline void expectEveryLogIsItsExpectedLog(const std::filesystem::path &logs) {
    std::vector<std::string> expected;
    for (const std::string &name : fileNames(expectedLogsDir())) {
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
        const std::filesystem::path expectedPath = expectedLogsDir() / (testCase + ".json");
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
        EXPECT_EQ(bytesOf(logs / name), want) << "the log of " << testCase << " is not its expected log";
    }
    EXPECT_EQ(written, expected) << "the runner wrote a log of each case that has an expected log, and of no other";
}

}  // namespace RuntimeSuiteExpectedLogs
