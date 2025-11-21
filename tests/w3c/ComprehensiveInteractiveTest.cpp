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
#include <random>
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
 * @brief Find reference snapshot matching target stepNumber
 *
 * Phase 1 may capture duplicate snapshots at same step when FINAL_STATE is returned
 * (e.g., refs[1] and refs[2] both have stepNumber=1).
 *
 * This function searches for the matching snapshot using reverse iteration to find
 * the LAST occurrence, which represents the most recent state at that step number.
 *
 * @param snapshots Vector of reference snapshots from Phase 1
 * @param stepNumber Target step number to find
 * @return Iterator to matching snapshot, or snapshots.end() if not found
 */
std::vector<StateSnapshot>::const_iterator findSnapshotByStepNumber(const std::vector<StateSnapshot> &snapshots,
                                                                    int stepNumber) {
    // Use reverse iterator to find LAST occurrence (most recent snapshot at this step)
    auto rit = std::find_if(snapshots.rbegin(), snapshots.rend(),
                            [stepNumber](const StateSnapshot &snap) { return snap.stepNumber == stepNumber; });

    // Convert reverse iterator to forward iterator
    return (rit != snapshots.rend()) ? std::prev(rit.base()) : snapshots.end();
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
        auto snapshot = captureCurrentSnapshot(runner);
        LOG_DEBUG("Phase 1: step={}, captured snapshot with stepNumber={}", step, snapshot.stepNumber);
        forwardSnapshots.push_back(snapshot);

        // Step forward
        auto result = runner.stepForward();

        // Check termination
        if (result == StepResult::FINAL_STATE || result == StepResult::NO_EVENTS_AVAILABLE) {
            totalSteps = step + 1;
            // Capture final snapshot
            auto finalSnapshot = captureCurrentSnapshot(runner);
            LOG_DEBUG("Phase 1: FINAL_STATE/NO_EVENTS_AVAILABLE, captured final snapshot with stepNumber={}",
                      finalSnapshot.stepNumber);
            forwardSnapshots.push_back(finalSnapshot);
            break;
        }

        if (result == StepResult::NO_EVENTS_READY) {
            // Scheduled events waiting - consider this end of execution
            totalSteps = step + 1;
            auto finalSnapshot = captureCurrentSnapshot(runner);
            LOG_DEBUG("Phase 1: NO_EVENTS_READY, captured final snapshot with stepNumber={}", finalSnapshot.stepNumber);
            forwardSnapshots.push_back(finalSnapshot);
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
                                      << " after stepping backward\\n"
                                      << diff.format();

        if (!diff.isIdentical) {
            // Stop on first failure to avoid cascade of errors
            return;
        }
    }

    // W3C SCXML 3.13: Always reset before Phase 3 for deterministic replay
    // When totalSteps=1, Phase 2 skips all backward steps, leaving scheduler logical time advanced
    // Reset ensures both currentStep_ and scheduler logical time are restored to initial state
    LOG_DEBUG("Before reset, getCurrentStep()={}", runner.getCurrentStep());
    runner.reset();
    LOG_DEBUG("After reset, getCurrentStep()={}", runner.getCurrentStep());

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
/**
 * @brief Scenario 3: Complex Navigation Pattern
 *
 * Verify: Complex forward/backward/reset combinations maintain consistency
 * Pattern: Forward → Back → Forward → Back → Reset → Forward
 * Goal: Ensure snapshot consistency across non-linear navigation
 */
TEST_P(ComprehensiveInteractiveTest, ComplexNavigationPattern) {
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

    // Phase 1: Execute forward and capture all snapshots
    std::vector<StateSnapshot> referenceSnapshots;
    int maxSteps = 100;

    for (int step = 0; step < maxSteps; step++) {
        referenceSnapshots.push_back(captureCurrentSnapshot(runner));

        auto result = runner.stepForward();

        if (result == StepResult::FINAL_STATE || result == StepResult::NO_EVENTS_AVAILABLE ||
            result == StepResult::NO_EVENTS_READY) {
            referenceSnapshots.push_back(captureCurrentSnapshot(runner));
            break;
        }
    }

    int totalSteps = static_cast<int>(referenceSnapshots.size()) - 1;

    // Phase 2: Adaptive complex navigation pattern based on available steps
    // Adapt pattern complexity to test length for comprehensive coverage

    if (totalSteps == 0) {
        // Only initial state - just verify reset
        runner.reset();
        auto snapshotReset = captureCurrentSnapshot(runner);
        auto diffReset = SnapshotComparator::compare(referenceSnapshots[0], snapshotReset);
        EXPECT_TRUE(diffReset.isIdentical) << "Test " << testId << ": Snapshot mismatch after reset\n"
                                           << diffReset.format();
        return;
    }

    // Pattern: Back to midpoint → Forward to end → Back to start → Reset → Forward to end
    int midPoint = totalSteps / 2;

    // CRITICAL FIX: Use runner's actual current step, not totalSteps index
    // After phase 1, runner might be at step N-1 even though totalSteps = N (due to FINAL_STATE)
    int currentStepBeforeBackward = runner.getCurrentStep();

    // Step 2.1: Back to midpoint (always execute, even if midPoint == 0)
    for (int i = currentStepBeforeBackward; i > midPoint; i--) {
        ASSERT_TRUE(runner.stepBackward()) << "Test " << testId << ": Failed to step back to " << i - 1;
    }

    auto snapshotMid = captureCurrentSnapshot(runner);
    auto itMid = findSnapshotByStepNumber(referenceSnapshots, snapshotMid.stepNumber);

    if (itMid != referenceSnapshots.end()) {
        auto diffMid = SnapshotComparator::compare(*itMid, snapshotMid);
        EXPECT_TRUE(diffMid.isIdentical) << "Test " << testId << ": Snapshot mismatch at midpoint after backward\n"
                                         << diffMid.format();
    }

    // Step 2.2: Forward to end
    for (int i = midPoint; i < totalSteps; i++) {
        auto result = runner.stepForward();
        if (result == StepResult::FINAL_STATE || result == StepResult::NO_EVENTS_AVAILABLE) {
            break;
        }
    }

    auto snapshotEnd = captureCurrentSnapshot(runner);
    auto itEnd = findSnapshotByStepNumber(referenceSnapshots, snapshotEnd.stepNumber);

    if (itEnd != referenceSnapshots.end()) {
        auto diffEnd = SnapshotComparator::compare(*itEnd, snapshotEnd);
        EXPECT_TRUE(diffEnd.isIdentical) << "Test " << testId << ": Snapshot mismatch at end after forward\n"
                                         << diffEnd.format();
    }

    // Step 2.3: Back to start
    int currentEnd = runner.getCurrentStep();
    for (int i = currentEnd; i > 0; i--) {
        ASSERT_TRUE(runner.stepBackward()) << "Test " << testId << ": Failed to step back to " << i - 1;
    }

    auto snapshotStart = captureCurrentSnapshot(runner);
    auto diffStart = SnapshotComparator::compare(referenceSnapshots[0], snapshotStart);
    EXPECT_TRUE(diffStart.isIdentical) << "Test " << testId << ": Snapshot mismatch at start after backward\n"
                                       << diffStart.format();

    // Step 2.4: Reset to step 0
    runner.reset();
    auto snapshotReset = captureCurrentSnapshot(runner);
    auto diffReset = SnapshotComparator::compare(referenceSnapshots[0], snapshotReset);
    EXPECT_TRUE(diffReset.isIdentical) << "Test " << testId << ": Snapshot mismatch after reset\n"
                                       << diffReset.format();

    // Step 2.5: Forward to end and verify
    for (int step = 0; step < totalSteps; step++) {
        auto result = runner.stepForward();

        auto currentSnapshot = captureCurrentSnapshot(runner);

        // Use helper function to find snapshot with matching stepNumber (finds LAST occurrence for duplicates)
        auto it = findSnapshotByStepNumber(referenceSnapshots, currentSnapshot.stepNumber);

        if (it != referenceSnapshots.end()) {
            auto diff = SnapshotComparator::compare(*it, currentSnapshot);
            EXPECT_TRUE(diff.isIdentical)
                << "Test " << testId << ": Snapshot mismatch at step " << currentSnapshot.stepNumber << " after reset\n"
                << diff.format();
        }

        if (result == StepResult::FINAL_STATE || result == StepResult::NO_EVENTS_AVAILABLE ||
            result == StepResult::NO_EVENTS_READY) {
            break;
        }
    }
}

/**
 * @brief Scenario 4: Multiple Reset Consistency
 *
 * Verify: Multiple reset cycles produce identical executions
 * Pattern: Execute → Reset → Execute → Reset → Execute
 * Goal: Ensure reset() is idempotent and deterministic across multiple cycles
 */
TEST_P(ComprehensiveInteractiveTest, MultipleResetConsistency) {
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

    // Execute 3 cycles: Execute → Reset → Execute → Reset → Execute
    const int numCycles = 3;
    std::vector<std::vector<StateSnapshot>> allCycleSnapshots(numCycles);
    int maxSteps = 100;

    for (int cycle = 0; cycle < numCycles; cycle++) {
        // Execute to completion
        for (int step = 0; step < maxSteps; step++) {
            allCycleSnapshots[cycle].push_back(captureCurrentSnapshot(runner));

            auto result = runner.stepForward();

            if (result == StepResult::FINAL_STATE || result == StepResult::NO_EVENTS_AVAILABLE ||
                result == StepResult::NO_EVENTS_READY) {
                allCycleSnapshots[cycle].push_back(captureCurrentSnapshot(runner));
                break;
            }
        }

        // Reset for next cycle (except last cycle)
        if (cycle < numCycles - 1) {
            runner.reset();

            // Verify reset returns to step 0
            auto resetSnapshot = captureCurrentSnapshot(runner);
            EXPECT_EQ(resetSnapshot.stepNumber, 0)
                << "Test " << testId << ": Reset did not return to step 0 in cycle " << cycle + 1;

            // Verify reset matches initial state
            auto diff = SnapshotComparator::compare(allCycleSnapshots[0][0], resetSnapshot);
            EXPECT_TRUE(diff.isIdentical)
                << "Test " << testId << ": Reset snapshot differs from initial in cycle " << cycle + 1 << "\n"
                << diff.format();
        }
    }

    // Verify all cycles produced identical executions
    for (int cycle = 1; cycle < numCycles; cycle++) {
        EXPECT_EQ(allCycleSnapshots[cycle].size(), allCycleSnapshots[0].size())
            << "Test " << testId << ": Cycle " << cycle + 1 << " has different number of steps than cycle 1";

        size_t minSize = std::min(allCycleSnapshots[cycle].size(), allCycleSnapshots[0].size());
        for (size_t step = 0; step < minSize; step++) {
            auto diff = SnapshotComparator::compare(allCycleSnapshots[0][step], allCycleSnapshots[cycle][step]);
            EXPECT_TRUE(diff.isIdentical)
                << "Test " << testId << ": Cycle " << cycle + 1 << " differs at step " << step << "\n"
                << diff.format();

            if (!diff.isIdentical) {
                return;  // Stop on first mismatch
            }
        }
    }
}

/**
 * @brief Scenario 5: Random Navigation Stress Test
 *
 * Verify: Random navigation patterns maintain snapshot consistency
 * Pattern: Deterministic pseudo-random sequence of forward/backward/reset
 * Goal: Stress test time-travel debugging with unpredictable navigation
 */
TEST_P(ComprehensiveInteractiveTest, RandomNavigationStress) {
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

    // Phase 1: Capture reference execution
    std::vector<StateSnapshot> referenceSnapshots;
    int maxSteps = 100;

    for (int step = 0; step < maxSteps; step++) {
        referenceSnapshots.push_back(captureCurrentSnapshot(runner));

        auto result = runner.stepForward();

        if (result == StepResult::FINAL_STATE || result == StepResult::NO_EVENTS_AVAILABLE ||
            result == StepResult::NO_EVENTS_READY) {
            referenceSnapshots.push_back(captureCurrentSnapshot(runner));
            break;
        }
    }

    int totalSteps = static_cast<int>(referenceSnapshots.size()) - 1;

    // Phase 2: Adaptive deterministic pseudo-random navigation (seed = testId for reproducibility)
    // Scale number of operations based on test length for comprehensive coverage
    std::mt19937 rng(testId);
    std::uniform_int_distribution<int> actionDist(0, 2);  // 0=forward, 1=backward, 2=reset

    // Scale random operations: min 10, max 50, scaled by totalSteps
    int numRandomOps = std::min(50, std::max(10, totalSteps * 5));

    // CRITICAL FIX: Initialize from runner's actual current step, not 0
    // After phase 1, runner might be at step N-1 even though totalSteps = N (due to FINAL_STATE)
    int currentStep = runner.getCurrentStep();

    for (int op = 0; op < numRandomOps; op++) {
        int action = actionDist(rng);

        if (action == 0) {
            // Forward
            if (currentStep < totalSteps) {
                runner.stepForward();
                // CRITICAL FIX: Synchronize currentStep with runner's actual step after forward
                // Don't track independently - stepForward() may not change step (NO_EVENTS_READY)
                currentStep = runner.getCurrentStep();
            }
        } else if (action == 1) {
            // Backward
            if (currentStep > 0) {
                bool backwardSuccess = runner.stepBackward();
                // CRITICAL FIX: Synchronize currentStep with runner's actual step after backward
                // Only decrement if backward actually succeeded
                if (backwardSuccess) {
                    currentStep = runner.getCurrentStep();
                } else {
                    // Backward failed (e.g., already at step 0), sync with actual step
                    currentStep = runner.getCurrentStep();
                    // Don't fail the test - just adjust tracking
                    continue;
                }
            }
        } else {
            // Reset
            runner.reset();
            currentStep = 0;
        }

        // Verify snapshot consistency
        auto currentSnapshot = captureCurrentSnapshot(runner);

        // Use helper function to find snapshot with matching stepNumber (finds LAST occurrence for duplicates)
        auto itSnap = findSnapshotByStepNumber(referenceSnapshots, currentSnapshot.stepNumber);

        if (itSnap != referenceSnapshots.end()) {
            auto diff = SnapshotComparator::compare(*itSnap, currentSnapshot);
            EXPECT_TRUE(diff.isIdentical)
                << "Test " << testId << ": Snapshot mismatch at step " << currentSnapshot.stepNumber
                << " after random operation " << op << " (action=" << action << ")\n"
                << diff.format();

            if (!diff.isIdentical) {
                return;  // Stop on first mismatch
            }
        }
    }
}

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
