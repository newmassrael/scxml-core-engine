// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "ITestMetadataParser.h"
#include "core/LogMacros.h"
#include <chrono>
#include <memory>
#include <string>

namespace SCE::W3C {

/**
 * @brief Test execution result
 */
enum class TestResult {
    PASS,     // Test completed successfully, reached expected target
    FAIL,     // Test completed but reached wrong target
    ERROR,    // Test execution failed (parse error, runtime exception, etc.)
    TIMEOUT,  // Test execution timed out
    SKIPPED   // Test was not run (a manual fixture with no verifier, or a BasicHTTP one under ThreadSanitizer)
};

/**
 * @brief The result's name, as the console, the XML report and its readers spell it
 */
inline const char *testResultToString(TestResult result) {
    switch (result) {
    case TestResult::PASS:
        return "PASS";
    case TestResult::FAIL:
        return "FAIL";
    case TestResult::ERROR:
        return "ERROR";
    case TestResult::TIMEOUT:
        return "TIMEOUT";
    case TestResult::SKIPPED:
        return "SKIPPED";
    }
    return "UNKNOWN";
}

/**
 * @brief Whether a result fails the run that reported it
 *
 * FAIL, ERROR and TIMEOUT do. PASS does not, and neither does SKIPPED: a
 * test that was not run neither passed nor failed, so every tally counts it
 * apart from both (TestSummaryHelper::updateSummary), and the JUnit report
 * marks it `<skipped/>`.
 */
inline bool failsTheRun(TestResult result) {
    switch (result) {
    case TestResult::PASS:
    case TestResult::SKIPPED:
        return false;
    case TestResult::FAIL:
    case TestResult::ERROR:
    case TestResult::TIMEOUT:
        return true;
    }
    return true;
}

/**
 * @brief Test execution context - contains all execution details
 */
struct TestExecutionContext {
    std::string scxmlContent;
    TestMetadata metadata;
    std::string finalState;      // The state machine ended in
    std::string expectedTarget;  // Expected target (pass/fail)
    std::chrono::milliseconds executionTime{0};
    std::string errorMessage;  // If result == ERROR
};

/**
 * @brief Interface for executing SCXML tests
 *
 * Single Responsibility: Only executes individual tests
 * - Runs SCXML through SCE engine
 * - Captures execution results
 * - Handles timeouts and errors
 */
class ITestExecutor {
public:
    virtual ~ITestExecutor() = default;

    /**
     * @brief Execute a single SCXML test
     * @param scxml The SCXML content to execute
     * @param metadata Test metadata for context
     * @return Test execution result with full context
     */
    virtual TestExecutionContext executeTest(const std::string &scxml, const TestMetadata &metadata) = 0;

    /**
     * @brief Execute a single SCXML test with source file path for relative resolution
     * @param scxml The SCXML content to execute
     * @param metadata Test metadata for context
     * @param sourceFilePath Path to the original TXML/SCXML file for relative path resolution
     * @return Test execution result with full context
     */
    virtual TestExecutionContext executeTest(const std::string &scxml, const TestMetadata &metadata,
                                             const std::string &sourceFilePath) {
        // Default implementation calls the original method (for backward compatibility)
        SCE_LOG_DEBUG("ITestExecutor: executeTest called with sourceFilePath: '{}'", sourceFilePath);
        return executeTest(scxml, metadata);
    }

    /**
     * @brief Set execution timeout
     * @param timeoutMs Timeout in milliseconds (default: 5000ms)
     */
    virtual void setTimeout(std::chrono::milliseconds timeoutMs) = 0;
};

}  // namespace SCE::W3C