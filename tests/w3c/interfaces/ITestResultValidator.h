// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "ITestExecutor.h"
#include <string>

namespace SCE::W3C {

/**
 * @brief W3C test validation result
 */
struct ValidationResult {
    bool isValid{false};
    // TestResult::SKIPPED when the test was not run: the one place a skip is
    // recorded, read by every tally (failsTheRun, TestSummaryHelper)
    TestResult finalResult{TestResult::ERROR};
    std::string reason;  // Explanation of validation logic

    // Default constructor
    ValidationResult() = default;

    ValidationResult(bool valid, TestResult result, const std::string &explanation)
        : isValid(valid), finalResult(result), reason(explanation) {}

    /// The result of a test that was not run, and why it was not
    static ValidationResult skip(const std::string &reason) {
        return ValidationResult(true, TestResult::SKIPPED, reason);
    }
};

/**
 * @brief Interface for validating test execution results
 *
 * Single Responsibility: Only validates test outcomes
 * - Interprets final states against expected targets
 * - Handles W3C-specific validation rules
 * - Provides clear validation reasoning
 */
class ITestResultValidator {
public:
    virtual ~ITestResultValidator() = default;

    /**
     * @brief Validate test execution result against expected outcome
     * @param context Complete test execution context
     * @return Validation result with explanation
     */
    virtual ValidationResult validateResult(const TestExecutionContext &context) = 0;

    /**
     * @brief Check if a test should be skipped (e.g., manual tests)
     * @param metadata Test metadata
     * @return true if test should be skipped
     */
    virtual bool shouldSkipTest(const TestMetadata &metadata) = 0;
};

}  // namespace SCE::W3C