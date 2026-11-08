// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/**
 * @file TestSummaryExitStatusTest.cpp
 * @brief The conformance runner's exit status is what CI acts on
 *
 * These cases exist because the rule was wrong and nothing could see it. The
 * CLI computed `errorTests == 0 && passRate > 0`, which never looks at
 * `failedTests`: W3C 233 and 234 reached their fail state while the process
 * exited 0, because the other four hundred tests passed and nothing errored.
 * The workflow's `exit $EXIT_CODE` then exited 0 too, so the lane could not
 * turn red on a conformance regression, and the failure was visible only as
 * an annotation nobody was gating on.
 *
 * The rule now lives in `TestSummaryHelper::exitStatus`, a pure function, so
 * a case here costs no suite run.
 */

#include "common/TestSummaryHelper.h"
#include "w3c/interfaces/ITestReporter.h"
#include <gtest/gtest.h>
#include <string>

using SCE::Common::TestSummaryHelper;
using SCE::W3C::TestRunSummary;

namespace {

TestRunSummary summaryOf(size_t total, size_t passed, size_t failed, size_t errors, size_t skipped = 0) {
    TestRunSummary s;
    s.totalTests = total;
    s.passedTests = passed;
    s.failedTests = failed;
    s.errorTests = errors;
    s.skippedTests = skipped;
    return s;
}

TEST(TestSummaryExitStatus, AllPassedIsSuccess) {
    EXPECT_EQ(0, TestSummaryHelper::exitStatus(summaryOf(404, 404, 0, 0)));
}

TEST(TestSummaryExitStatus, OneFailedAmongManyPassedIsFailure) {
    // The exact shape that escaped: 402 of 404 passing, two failing, nothing
    // in error. The previous rule returned 0 here.
    EXPECT_EQ(1, TestSummaryHelper::exitStatus(summaryOf(404, 402, 2, 0)));
}

TEST(TestSummaryExitStatus, ErrorIsFailure) {
    EXPECT_EQ(1, TestSummaryHelper::exitStatus(summaryOf(404, 403, 0, 1)));
}

TEST(TestSummaryExitStatus, RunningNothingIsFailure) {
    // What `passRate > 0` was reaching for: a run that executed no test has
    // not demonstrated conformance, and reporting success would make a
    // broken invocation indistinguishable from a clean suite. Measured on
    // this repo: three CI runs configured a build whose generator path was
    // stale, executed zero tests, and published a green check.
    EXPECT_EQ(1, TestSummaryHelper::exitStatus(summaryOf(0, 0, 0, 0)));
}

TEST(TestSummaryExitStatus, SkippedTestsAreNotFailures) {
    // A skip is a decision the suite already made; it is counted in
    // totalTests but in none of the outcome buckets.
    EXPECT_EQ(0, TestSummaryHelper::exitStatus(summaryOf(404, 400, 0, 0, 4)));
}

TEST(TestSummaryExitStatus, AFailureAmongSkipsStillFails) {
    EXPECT_EQ(1, TestSummaryHelper::exitStatus(summaryOf(404, 399, 1, 0, 4)));
}

SCE::W3C::TestReport reportOf(const std::string &testId, const SCE::W3C::ValidationResult &result) {
    SCE::W3C::TestReport report;
    report.testId = testId;
    report.engineType = "interpreter";
    report.validationResult = result;
    return report;
}

// The tally, from the reports a run made. A BasicHTTP fixture under
// ThreadSanitizer and a manual fixture with no verifier are not run, and were
// once reported as PASS with a reason that said "Skipped": the summary, the
// XML report and the console all counted them as passes, so a build that ran
// none of the HTTP fixtures reported them passing.
TEST(TestSummaryTally, ASkipIsCountedAsASkipNeverAsAPass) {
    using SCE::W3C::TestResult;
    using SCE::W3C::ValidationResult;
    const auto summary = TestSummaryHelper::calculateSummary({
        reportOf("201", ValidationResult(true, TestResult::PASS, "ran")),
        reportOf("513", ValidationResult::skip("Skipped: HTTP test under ThreadSanitizer")),
        reportOf("1", ValidationResult::skip("Skipped: manual fixture with no verifier")),
    });

    EXPECT_EQ(3u, summary.totalTests);
    EXPECT_EQ(1u, summary.passedTests);
    EXPECT_EQ(2u, summary.skippedTests);
    EXPECT_EQ(0u, summary.failedTests);
    EXPECT_EQ(0u, summary.errorTests);
    EXPECT_TRUE(summary.failedTestIds.empty());
    EXPECT_TRUE(summary.errorTestIds.empty());
    // The pass rate is over the tests that ran
    EXPECT_DOUBLE_EQ(100.0, summary.passRate);
    EXPECT_EQ(0, TestSummaryHelper::exitStatus(summary));
}

TEST(TestSummaryTally, ASkipIsNeverAFailure) {
    using SCE::W3C::TestResult;
    EXPECT_FALSE(SCE::W3C::failsTheRun(TestResult::SKIPPED));
    EXPECT_FALSE(SCE::W3C::failsTheRun(TestResult::PASS));
    EXPECT_TRUE(SCE::W3C::failsTheRun(TestResult::FAIL));
    EXPECT_TRUE(SCE::W3C::failsTheRun(TestResult::ERROR));
    EXPECT_TRUE(SCE::W3C::failsTheRun(TestResult::TIMEOUT));
}

TEST(TestSummaryTally, AFailureBesideASkipIsStillAFailure) {
    using SCE::W3C::TestResult;
    using SCE::W3C::ValidationResult;
    const auto summary = TestSummaryHelper::calculateSummary({
        reportOf("513", ValidationResult::skip("Skipped: HTTP test under ThreadSanitizer")),
        reportOf("233", ValidationResult(false, TestResult::FAIL, "reached fail")),
    });

    EXPECT_EQ(1u, summary.skippedTests);
    EXPECT_EQ(1u, summary.failedTests);
    EXPECT_EQ(0u, summary.passedTests);
    EXPECT_EQ(1, TestSummaryHelper::exitStatus(summary));
}

}  // namespace
