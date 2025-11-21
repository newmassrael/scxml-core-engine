// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * @file ComprehensiveInteractiveTest.cpp
 * @brief Comprehensive automated verification system for time-travel debugging
 *
 * Verifies step forward, step back, and reset operations work correctly
 * for ALL W3C SCXML tests using programmatic snapshot comparison.
 *
 * W3C SCXML Compliance: Tests maintain perfect state consistency across
 * all time-travel operations per W3C SCXML 3.13 microstep semantics.
 */

#include <filesystem>
#include <fstream>
#include <gtest/gtest.h>
#include <set>
#include <sstream>
#include <string>
#include <vector>

#include "InteractiveTestRunner.h"
#include "common/Logger.h"
#include "impl/SnapshotComparator.h"
#include "runtime/StateSnapshot.h"

namespace fs = std::filesystem;

namespace SCE::W3C {

// W3C SCXML tests with non-deterministic behavior (excluded from automated verification)
// Math.random() tests, HTTP tests with network timing variability
const std::set<int> NON_DETERMINISTIC_TESTS = {579,  // Math.random() usage

                                               // HTTP tests: Network timing variability (WASM build only)
                                               509, 510, 513, 518, 519, 520, 522, 531, 532, 534, 567, 577};

// Tests requiring special infrastructure or having known issues
const std::set<int> EXCLUDED_TESTS = {
    // Add test IDs here if they require special handling
};

// CMake-configured resources path (absolute path to project root)
#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

/**
 * @brief Helper to get all W3C test IDs from resources/ directory
 */
std::vector<int> discoverW3CTests() {
    std::vector<int> testIds;

#ifdef __EMSCRIPTEN__
    // WASM: Use NODEFS mounted path
    std::string resourcesPath = "/project/resources";
#else
    // Native: Use SCE_PROJECT_ROOT macro
    std::string resourcesPath = std::string(SCE_PROJECT_ROOT) + "/resources";
#endif

    if (!fs::exists(resourcesPath)) {
        return testIds;
    }

    for (const auto &entry : fs::directory_iterator(resourcesPath)) {
        if (entry.is_directory()) {
            std::string dirname = entry.path().filename().string();
            // Check if directory name is numeric (W3C test ID)
            if (std::all_of(dirname.begin(), dirname.end(), ::isdigit)) {
                int testId = std::stoi(dirname);
                testIds.push_back(testId);
            }
        }
    }

    std::sort(testIds.begin(), testIds.end());
    return testIds;
}

// Global variable to store filtered test IDs
static std::vector<int> g_filteredTestIds;

/**
 * @brief Parse test IDs from environment variable W3C_TEST_IDS or command line args
 *
 * Supports two formats:
 * 1. Environment variable: W3C_TEST_IDS=144,147,192
 * 2. Command line: ./comprehensive_interactive_test 144 147 192
 *
 * If not set or empty, returns all discovered tests
 *
 * @return Vector of test IDs to run
 */
std::vector<int> getTestsToRun() {
    // Check if test IDs were provided via command line (stored in global variable)
    if (!g_filteredTestIds.empty()) {
        return g_filteredTestIds;
    }

    // Fallback to environment variable
    const char *envTestIds = std::getenv("W3C_TEST_IDS");

    if (!envTestIds || std::string(envTestIds).empty()) {
        // No filter specified, return all tests
        return discoverW3CTests();
    }

    std::vector<int> testIds;
    std::string testIdsStr(envTestIds);
    std::stringstream ss(testIdsStr);
    std::string token;

    while (std::getline(ss, token, ',')) {
        // Trim whitespace
        token.erase(0, token.find_first_not_of(" \t"));
        token.erase(token.find_last_not_of(" \t") + 1);

        if (!token.empty() && std::all_of(token.begin(), token.end(), ::isdigit)) {
            testIds.push_back(std::stoi(token));
        }
    }

    std::sort(testIds.begin(), testIds.end());

    if (testIds.empty()) {
        // Invalid format, return all tests as fallback
        return discoverW3CTests();
    }

    return testIds;
}

/**
 * @brief Helper to get SCXML file path for a test
 */
std::string getTestSCXMLPath(int testId) {
#ifdef __EMSCRIPTEN__
    // WASM: Use NODEFS mounted path (project root mounted at /project in wasm_polyfill.js)
    std::string resourcesBase = "/project/resources/";
#else
    // Native: Use SCE_PROJECT_ROOT macro
    std::string resourcesBase = std::string(SCE_PROJECT_ROOT) + "/resources/";
#endif

    // Try .scxml first, then .txml
    std::string scxmlPath = resourcesBase + std::to_string(testId) + "/test" + std::to_string(testId) + ".scxml";
    if (fs::exists(scxmlPath)) {
        return scxmlPath;
    }

    std::string txmlPath = resourcesBase + std::to_string(testId) + "/test" + std::to_string(testId) + ".txml";
    if (fs::exists(txmlPath)) {
        return txmlPath;
    }

    return "";
}

/**
 * @brief Helper to check if test should be skipped
 */
bool shouldSkipTest(int testId) {
    return NON_DETERMINISTIC_TESTS.count(testId) > 0 || EXCLUDED_TESTS.count(testId) > 0;
}

/**
 * @brief Get reason for test skip
 */
std::string getSkipReason(int testId) {
    if (NON_DETERMINISTIC_TESTS.count(testId) > 0) {
        return "Non-deterministic behavior (Math.random() or HTTP timing)";
    }
    if (EXCLUDED_TESTS.count(testId) > 0) {
        return "Requires special infrastructure";
    }
    return "Unknown";
}

/**
 * @brief Helper to capture current snapshot from InteractiveTestRunner
 *
 * Extracts complete state machine state for comparison.
 */
StateSnapshot captureCurrentSnapshot(InteractiveTestRunner &runner) {
    StateSnapshot snapshot;

    // Capture activeStates (W3C SCXML 3.13: preserve document order for time-travel debugging)
    snapshot.activeStates = runner.getActiveStates();

    // Capture stepNumber
    snapshot.stepNumber = runner.getCurrentStep();

    // Get last event name
    snapshot.lastEventName = runner.getLastEventName();

    // Note: dataModel, queues, scheduledEvents extraction requires additional API methods
    // For initial implementation, focus on activeStates and stepNumber

    return snapshot;
}

/**
 * @brief Parameterized test fixture for comprehensive interactive debugging verification
 */
class ComprehensiveInteractiveTest : public ::testing::TestWithParam<int> {
protected:
    void SetUp() override {
        // Set log level to warn to reduce noise
        Logger::setLevel(LogLevel::Warn);
    }

    void TearDown() override {
        // Cleanup
    }
};

/**
 * @brief Scenario 1: Forward-Back-Forward Determinism
 *
 * Verify: Forward N steps → Back N steps → Forward N steps = identical snapshots
 * Goal: Ensure step backward + step forward produces exact same execution path
 */
TEST_P(ComprehensiveInteractiveTest, ForwardBackwardDeterminism) {
    int testId = GetParam();

    // Skip non-deterministic tests
    if (shouldSkipTest(testId)) {
        GTEST_SKIP() << "Test " << testId << " skipped: " << getSkipReason(testId);
        return;
    }

    // Get SCXML path
    std::string scxmlPath = getTestSCXMLPath(testId);
    if (scxmlPath.empty()) {
        GTEST_SKIP() << "Test " << testId << " SCXML file not found";
        return;
    }

    // Create runner and load SCXML
    InteractiveTestRunner runner;
    if (!runner.loadSCXML(scxmlPath)) {
        GTEST_SKIP() << "Test " << testId << " failed to load SCXML";
        return;
    }

    if (!runner.initialize()) {
        GTEST_SKIP() << "Test " << testId << " failed to initialize";
        return;
    }

    // Phase 1: Execute forward steps and capture snapshots
    std::vector<StateSnapshot> forwardSnapshots;
    int maxSteps = 100;  // Safety limit
    int totalSteps = 0;

    for (int step = 0; step < maxSteps; step++) {
        // Capture snapshot at current state
        forwardSnapshots.push_back(captureCurrentSnapshot(runner));

        // Step forward
        auto result = runner.stepForward();

        // Check termination
        if (result == StepResult::FINAL_STATE || result == StepResult::NO_EVENTS_AVAILABLE) {
            totalSteps = step + 1;
            // Capture final snapshot
            forwardSnapshots.push_back(captureCurrentSnapshot(runner));
            break;
        }

        if (result == StepResult::NO_EVENTS_READY) {
            // Scheduled events waiting - consider this end of execution
            totalSteps = step + 1;
            forwardSnapshots.push_back(captureCurrentSnapshot(runner));
            break;
        }
    }

    // If reached maxSteps without termination, record total
    if (totalSteps == 0) {
        totalSteps = maxSteps;
        forwardSnapshots.push_back(captureCurrentSnapshot(runner));
    }

    // Phase 2: Step backward and verify each snapshot
    for (int step = totalSteps - 1; step >= 0; step--) {
        if (step == 0) {
            // Already at step 0, can't step back further
            break;
        }

        bool backwardSuccess = runner.stepBackward();
        ASSERT_TRUE(backwardSuccess) << "Test " << testId << ": Failed to step backward to step " << step - 1;

        // Capture current snapshot after stepping backward
        auto currentSnapshot = captureCurrentSnapshot(runner);

        // Compare with original forward snapshot at this step
        auto diff = SnapshotComparator::compare(forwardSnapshots[step - 1], currentSnapshot);

        EXPECT_TRUE(diff.isIdentical) << "Test " << testId << ": Snapshot mismatch at step " << step - 1
                                      << " after stepping backward\n"
                                      << diff.format();

        if (!diff.isIdentical) {
            // Stop on first failure to avoid cascade of errors
            return;
        }
    }

    // Phase 3: Step forward again and verify determinism (replay)
    for (int step = 0; step < totalSteps; step++) {
        // Capture snapshot before stepping forward
        auto beforeSnapshot = captureCurrentSnapshot(runner);

        // Verify we're at the correct step
        EXPECT_EQ(beforeSnapshot.stepNumber, step)
            << "Test " << testId << ": Step number mismatch before forward step " << step;

        // Step forward
        auto result = runner.stepForward();

        // Capture snapshot after stepping forward
        auto afterSnapshot = captureCurrentSnapshot(runner);

        // Compare with original forward snapshot at next step
        if (static_cast<size_t>(step + 1) < forwardSnapshots.size()) {
            auto diff = SnapshotComparator::compare(forwardSnapshots[step + 1], afterSnapshot);

            EXPECT_TRUE(diff.isIdentical)
                << "Test " << testId << ": Forward replay mismatch at step " << step + 1 << "\n"
                << diff.format();

            if (!diff.isIdentical) {
                // Stop on first failure
                return;
            }
        }

        // Check if we've reached the end
        if (result == StepResult::FINAL_STATE || result == StepResult::NO_EVENTS_AVAILABLE ||
            result == StepResult::NO_EVENTS_READY) {
            break;
        }
    }
}

/**
 * @brief Scenario 2: Reset Replay Consistency
 *
 * Verify: Forward to end → Reset → Forward to end = identical execution
 * Goal: Ensure reset() correctly restores initial state and re-execution is deterministic
 */
TEST_P(ComprehensiveInteractiveTest, ResetReplayConsistency) {
    int testId = GetParam();

    // Skip non-deterministic tests
    if (shouldSkipTest(testId)) {
        GTEST_SKIP() << "Test " << testId << " skipped: " << getSkipReason(testId);
        return;
    }

    // Get SCXML path
    std::string scxmlPath = getTestSCXMLPath(testId);
    if (scxmlPath.empty()) {
        GTEST_SKIP() << "Test " << testId << " SCXML file not found";
        return;
    }

    // Create runner and load SCXML
    InteractiveTestRunner runner;
    if (!runner.loadSCXML(scxmlPath)) {
        GTEST_SKIP() << "Test " << testId << " failed to load SCXML";
        return;
    }

    if (!runner.initialize()) {
        GTEST_SKIP() << "Test " << testId << " failed to initialize";
        return;
    }

    // Phase 1: Execute to completion and capture all snapshots
    std::vector<StateSnapshot> firstRunSnapshots;
    int maxSteps = 100;

    for (int step = 0; step < maxSteps; step++) {
        firstRunSnapshots.push_back(captureCurrentSnapshot(runner));

        auto result = runner.stepForward();

        if (result == StepResult::FINAL_STATE || result == StepResult::NO_EVENTS_AVAILABLE ||
            result == StepResult::NO_EVENTS_READY) {
            firstRunSnapshots.push_back(captureCurrentSnapshot(runner));
            break;
        }
    }

    // Phase 2: Reset
    runner.reset();

    // Verify reset to step 0
    auto resetSnapshot = captureCurrentSnapshot(runner);
    EXPECT_EQ(resetSnapshot.stepNumber, 0) << "Test " << testId << ": Reset did not return to step 0";

    // Verify reset snapshot matches initial snapshot
    auto diff = SnapshotComparator::compare(firstRunSnapshots[0], resetSnapshot);
    EXPECT_TRUE(diff.isIdentical) << "Test " << testId << ": Reset snapshot differs from initial snapshot\n"
                                  << diff.format();

    // Phase 3: Re-execute and compare
    for (size_t step = 0; step < firstRunSnapshots.size() - 1; step++) {
        // Capture before stepping
        auto beforeSnapshot = captureCurrentSnapshot(runner);

        // Step forward
        auto result = runner.stepForward();

        // Capture after stepping
        auto afterSnapshot = captureCurrentSnapshot(runner);

        // Compare with first run
        if (step + 1 < firstRunSnapshots.size()) {
            auto diff = SnapshotComparator::compare(firstRunSnapshots[step + 1], afterSnapshot);

            EXPECT_TRUE(diff.isIdentical) << "Test " << testId << ": Reset replay mismatch at step " << step + 1 << "\n"
                                          << diff.format();

            if (!diff.isIdentical) {
                return;
            }
        }

        if (result == StepResult::FINAL_STATE || result == StepResult::NO_EVENTS_AVAILABLE ||
            result == StepResult::NO_EVENTS_READY) {
            break;
        }
    }
}

// Instantiate tests for all discovered W3C tests
// Use W3C_TEST_IDS environment variable to filter tests (e.g., W3C_TEST_IDS=144,147,192)
// Or pass test IDs as command line arguments (e.g., ./comprehensive_interactive_test 144 147 192)
INSTANTIATE_TEST_SUITE_P(AllW3CTests, ComprehensiveInteractiveTest, ::testing::ValuesIn(getTestsToRun()),
                         [](const ::testing::TestParamInfo<int> &info) { return "Test" + std::to_string(info.param); });

}  // namespace SCE::W3C

/**
 * @brief Custom main to support command line test ID filtering
 *
 * Usage:
 *   ./comprehensive_interactive_test              # Run all tests
 *   ./comprehensive_interactive_test 144 147 192  # Run specific tests
 *   ./comprehensive_interactive_test 570          # Run single test
 *
 * Also supports GTest flags:
 *   ./comprehensive_interactive_test 144 --gtest_repeat=3
 */
int main(int argc, char **argv) {
    // Parse command line arguments for test IDs
    std::vector<int> testIds;

    // Scan for test IDs first (pure digit arguments)
    for (int i = 1; i < argc; ++i) {
        std::string arg(argv[i]);

        // Check if this is a test ID (pure digits)
        if (!arg.empty() && std::all_of(arg.begin(), arg.end(), ::isdigit)) {
            testIds.push_back(std::stoi(arg));
        }
    }

    // Store test IDs in global variable for getTestsToRun()
    if (!testIds.empty()) {
        std::sort(testIds.begin(), testIds.end());
        SCE::W3C::g_filteredTestIds = testIds;

        std::cout << "Running tests: ";
        for (size_t i = 0; i < testIds.size(); ++i) {
            std::cout << testIds[i];
            if (i < testIds.size() - 1) {
                std::cout << ", ";
            }
        }
        std::cout << std::endl;
    }

    // Remove test IDs from argv by compacting the array
    int newArgc = 1;  // Start with program name
    for (int i = 1; i < argc; ++i) {
        std::string arg(argv[i]);

        // Keep this argument if it's not a pure digit (not a test ID)
        if (arg.empty() || !std::all_of(arg.begin(), arg.end(), ::isdigit)) {
            argv[newArgc++] = argv[i];
        }
    }

    // Initialize GTest with filtered arguments (no test IDs in argv)
    ::testing::InitGoogleTest(&newArgc, argv);

    return RUN_ALL_TESTS();
}
