// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $5000 cumulative
//   Enterprise: Contact for pricing
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include "interfaces/ITestExecutor.h"
#include "interfaces/ITestReporter.h"
#include <vector>

namespace SCE::Common {

/**
 * @brief Test summary calculation helper (Single Source of Truth)
 *
 * W3C SCXML test infrastructure: Centralized summary statistics calculation
 * shared across Interpreter engine, AOT engine, and CLI test runners.
 *
 * Zero Duplication: Eliminates 5 duplicate implementations of skip counting logic
 * (W3CTestRunner::calculateSummary, W3CTestCLI "up to" mode, specific tests mode,
 * AOT mode, engine-specific stats).
 *
 * Features:
 * - Skipped test handling (excluded from pass rate calculation)
 * - Pass/fail/error counting with test ID tracking
 * - Execution time accumulation
 * - Pass rate calculation (passed / (passed + failed + error) * 100)
 *
 * Used by: W3CTestRunner, W3CTestCLI, AOT test infrastructure
 * Benefits: Zero code duplication, guaranteed consistency, simplified maintenance
 */
class TestSummaryHelper {
public:
    /**
     * @brief Update summary statistics from a single test report
     *
     * W3C SCXML test infrastructure: Handles skipped tests appropriately
     * - Skipped tests increment skippedTests counter
     * - Skipped tests NOT counted in passedTests (avoid false positives)
     * - Only non-skipped tests contribute to pass/fail/error counts
     *
     * @param summary TestRunSummary to update (modified in-place)
     * @param report TestReport containing test execution results
     */
    static void updateSummary(W3C::TestRunSummary &summary, const W3C::TestReport &report) {
        if (report.validationResult.skipped) {
            summary.skippedTests++;
            // Skipped tests are not counted as passed
        } else {
            // Only count non-skipped tests in passed/failed/error
            switch (report.validationResult.finalResult) {
            case W3C::TestResult::PASS:
                summary.passedTests++;
                break;
            case W3C::TestResult::FAIL:
                summary.failedTests++;
                summary.failedTestIds.push_back(report.testId);
                break;
            case W3C::TestResult::ERROR:
            case W3C::TestResult::TIMEOUT:
                summary.errorTests++;
                summary.errorTestIds.push_back(report.testId);
                break;
            }
        }

        summary.totalExecutionTime += report.executionContext.executionTime;
    }

    /**
     * @brief Calculate complete summary from test reports
     *
     * W3C SCXML test infrastructure: Aggregates all test results and calculates pass rate
     * Pass rate = passedTests / (passedTests + failedTests + errorTests) * 100
     * Skipped tests excluded from pass rate calculation
     *
     * @param reports Vector of test reports from test run
     * @return TestRunSummary with calculated statistics and pass rate
     */
    static W3C::TestRunSummary calculateSummary(const std::vector<W3C::TestReport> &reports) {
        W3C::TestRunSummary summary;
        summary.totalTests = reports.size();

        for (const auto &report : reports) {
            updateSummary(summary, report);
        }

        // Calculate pass rate (exclude skipped tests from denominator)
        size_t nonSkippedTests = summary.passedTests + summary.failedTests + summary.errorTests;
        summary.passRate =
            nonSkippedTests > 0 ? (static_cast<double>(summary.passedTests) / nonSkippedTests * 100.0) : 0.0;

        return summary;
    }

    /**
     * @brief Process exit status a run with this summary must report
     *
     * A conformance run's exit status is what CI acts on, so the rule lives
     * here as a pure function rather than inline at the one caller that
     * happens to exit: it is then testable without running a suite, which is
     * what its last regression needed. The CLI computed
     * `errorTests == 0 && passRate > 0` and so ignored `failedTests`
     * entirely — W3C 233 and 234 sat in their fail state while the process
     * exited 0, because four hundred other tests passed and nothing errored,
     * and the CI lane had no signal to turn red on.
     *
     * A run that executed nothing is also a failure, which is what the old
     * `passRate > 0` term was reaching for. Skipped tests are not failures:
     * they are counted in `totalTests` but in none of the three outcome
     * buckets, so a fully-skipped run still reports success here — the
     * decision to skip is the suite's, made before this is consulted.
     *
     * @return 0 when every executed test passed and at least one ran; 1 otherwise
     */
    static int exitStatus(const W3C::TestRunSummary &summary) {
        const bool ranSomething = summary.totalTests > 0;
        const bool allClean = summary.failedTests == 0 && summary.errorTests == 0;
        return (ranSomething && allClean) ? 0 : 1;
    }
};

}  // namespace SCE::Common
