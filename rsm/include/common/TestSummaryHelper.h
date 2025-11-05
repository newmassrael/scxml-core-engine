// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-RSM-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of RSM (Reactive State Machine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $100 cumulative
//   Enterprise: $500 cumulative
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/reactive-state-machine/blob/main/LICENSE

#pragma once

#include "interfaces/ITestExecutor.h"
#include "interfaces/ITestReporter.h"
#include <vector>

namespace RSM::Common {

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
};

}  // namespace RSM::Common
