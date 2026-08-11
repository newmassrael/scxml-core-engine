// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

}  // namespace
