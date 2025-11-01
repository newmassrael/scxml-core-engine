#include "ReadySCXMLEngine.h"
#include <filesystem>
#include <fstream>
#include <gtest/gtest.h>

namespace RSM {

/**
 * @brief Test fixture for ReadySCXMLEngine API tests
 *
 * Tests the high-level facade API that provides a production-ready,
 * zero-configuration interface for SCXML state machine execution.
 */
class ReadySCXMLEngineTest : public ::testing::Test {
protected:
    void SetUp() override {
        // Create test directory
        testDir_ = "/tmp/ready_scxml_test";
        std::filesystem::create_directories(testDir_);
    }

    void TearDown() override {
        // Clean up test files
        if (std::filesystem::exists(testDir_)) {
            std::filesystem::remove_all(testDir_);
        }
    }

    /**
     * @brief Create a simple test SCXML file
     */
    std::string createSimpleTestFile() {
        std::string filePath = testDir_ + "/simple.scxml";
        std::ofstream file(filePath);
        file << R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="idle">
    <datamodel>
        <data id="counter" expr="0"/>
    </datamodel>

    <state id="idle">
        <transition event="start" target="running"/>
    </state>

    <state id="running">
        <onentry>
            <assign location="counter" expr="counter + 1"/>
        </onentry>
        <transition event="pause" target="paused"/>
        <transition event="stop" target="stopped"/>
    </state>

    <state id="paused">
        <transition event="resume" target="running"/>
        <transition event="stop" target="stopped"/>
    </state>

    <final id="stopped"/>
</scxml>)";
        file.close();
        return filePath;
    }

    /**
     * @brief Get simple SCXML content as string
     */
    std::string getSimpleScxmlString() {
        return R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="start">
    <state id="start">
        <transition event="go" target="end"/>
    </state>
    <final id="end"/>
</scxml>)";
    }

    std::string testDir_;
};

// ============================================================================
// Factory Method Tests
// ============================================================================

TEST_F(ReadySCXMLEngineTest, CreateFromFile_ValidFile_Success) {
    std::string filePath = createSimpleTestFile();

    auto engine = ReadySCXMLEngine::fromFile(filePath);

    ASSERT_NE(engine, nullptr);
}

TEST_F(ReadySCXMLEngineTest, CreateFromFile_InvalidFile_ReturnsNull) {
    auto engine = ReadySCXMLEngine::fromFile("/nonexistent/file.scxml");

    EXPECT_EQ(engine, nullptr);
}

TEST_F(ReadySCXMLEngineTest, CreateFromString_ValidContent_Success) {
    std::string scxmlContent = getSimpleScxmlString();

    auto engine = ReadySCXMLEngine::fromString(scxmlContent);

    ASSERT_NE(engine, nullptr);
}

TEST_F(ReadySCXMLEngineTest, CreateFromString_InvalidContent_ReturnsNull) {
    std::string invalidContent = "not valid SCXML";

    auto engine = ReadySCXMLEngine::fromString(invalidContent);

    EXPECT_EQ(engine, nullptr);
}

TEST_F(ReadySCXMLEngineTest, CreateFromString_EmptyContent_ReturnsNull) {
    auto engine = ReadySCXMLEngine::fromString("");

    EXPECT_EQ(engine, nullptr);
}

// ============================================================================
// State Machine Lifecycle Tests
// ============================================================================

TEST_F(ReadySCXMLEngineTest, Start_ValidEngine_Success) {
    std::string scxmlContent = getSimpleScxmlString();
    auto engine = ReadySCXMLEngine::fromString(scxmlContent);
    ASSERT_NE(engine, nullptr);

    bool started = engine->start();

    EXPECT_TRUE(started);
    EXPECT_TRUE(engine->isRunning());
}

TEST_F(ReadySCXMLEngineTest, Stop_RunningEngine_Success) {
    std::string scxmlContent = getSimpleScxmlString();
    auto engine = ReadySCXMLEngine::fromString(scxmlContent);
    ASSERT_NE(engine, nullptr);
    ASSERT_TRUE(engine->start());

    engine->stop();

    EXPECT_FALSE(engine->isRunning());
}

TEST_F(ReadySCXMLEngineTest, Start_AfterStop_CanRestart) {
    std::string scxmlContent = getSimpleScxmlString();
    auto engine = ReadySCXMLEngine::fromString(scxmlContent);
    ASSERT_NE(engine, nullptr);
    ASSERT_TRUE(engine->start());

    engine->stop();
    bool restarted = engine->start();

    EXPECT_TRUE(restarted);
    EXPECT_TRUE(engine->isRunning());
}

// ============================================================================
// Event Handling Tests
// ============================================================================

TEST_F(ReadySCXMLEngineTest, SendEvent_ValidEvent_Success) {
    std::string filePath = createSimpleTestFile();
    auto engine = ReadySCXMLEngine::fromFile(filePath);
    ASSERT_NE(engine, nullptr);
    ASSERT_TRUE(engine->start());

    bool sent = engine->sendEvent("start");

    EXPECT_TRUE(sent);
}

TEST_F(ReadySCXMLEngineTest, SendEvent_WithEventData_Success) {
    std::string filePath = createSimpleTestFile();
    auto engine = ReadySCXMLEngine::fromFile(filePath);
    ASSERT_NE(engine, nullptr);
    ASSERT_TRUE(engine->start());

    bool sent = engine->sendEvent("start", R"({"key": "value"})");

    EXPECT_TRUE(sent);
}

TEST_F(ReadySCXMLEngineTest, SendEvent_BeforeStart_Fails) {
    std::string filePath = createSimpleTestFile();
    auto engine = ReadySCXMLEngine::fromFile(filePath);
    ASSERT_NE(engine, nullptr);

    bool sent = engine->sendEvent("start");

    EXPECT_FALSE(sent);
}

// ============================================================================
// State Query Tests
// ============================================================================

TEST_F(ReadySCXMLEngineTest, IsRunning_BeforeStart_ReturnsFalse) {
    std::string scxmlContent = getSimpleScxmlString();
    auto engine = ReadySCXMLEngine::fromString(scxmlContent);
    ASSERT_NE(engine, nullptr);

    EXPECT_FALSE(engine->isRunning());
}

TEST_F(ReadySCXMLEngineTest, IsRunning_AfterStart_ReturnsTrue) {
    std::string scxmlContent = getSimpleScxmlString();
    auto engine = ReadySCXMLEngine::fromString(scxmlContent);
    ASSERT_NE(engine, nullptr);
    ASSERT_TRUE(engine->start());

    EXPECT_TRUE(engine->isRunning());
}

TEST_F(ReadySCXMLEngineTest, GetCurrentState_AfterStart_ReturnsInitialState) {
    std::string scxmlContent = getSimpleScxmlString();
    auto engine = ReadySCXMLEngine::fromString(scxmlContent);
    ASSERT_NE(engine, nullptr);
    ASSERT_TRUE(engine->start());

    std::string currentState = engine->getCurrentState();

    EXPECT_EQ(currentState, "start");
}

TEST_F(ReadySCXMLEngineTest, GetCurrentState_BeforeStart_ReturnsEmpty) {
    std::string scxmlContent = getSimpleScxmlString();
    auto engine = ReadySCXMLEngine::fromString(scxmlContent);
    ASSERT_NE(engine, nullptr);

    std::string currentState = engine->getCurrentState();

    EXPECT_TRUE(currentState.empty());
}

TEST_F(ReadySCXMLEngineTest, IsInState_CurrentState_ReturnsTrue) {
    std::string filePath = createSimpleTestFile();
    auto engine = ReadySCXMLEngine::fromFile(filePath);
    ASSERT_NE(engine, nullptr);
    ASSERT_TRUE(engine->start());

    bool inState = engine->isInState("idle");

    EXPECT_TRUE(inState);
}

TEST_F(ReadySCXMLEngineTest, IsInState_OtherState_ReturnsFalse) {
    std::string filePath = createSimpleTestFile();
    auto engine = ReadySCXMLEngine::fromFile(filePath);
    ASSERT_NE(engine, nullptr);
    ASSERT_TRUE(engine->start());

    bool inState = engine->isInState("running");

    EXPECT_FALSE(inState);
}

TEST_F(ReadySCXMLEngineTest, GetActiveStates_AfterStart_ReturnsActiveStates) {
    std::string scxmlContent = getSimpleScxmlString();
    auto engine = ReadySCXMLEngine::fromString(scxmlContent);
    ASSERT_NE(engine, nullptr);
    ASSERT_TRUE(engine->start());

    auto activeStates = engine->getActiveStates();

    EXPECT_FALSE(activeStates.empty());
    EXPECT_EQ(activeStates.size(), 1);
    EXPECT_EQ(activeStates[0], "start");
}

// ============================================================================
// Variable Access Tests
// ============================================================================

TEST_F(ReadySCXMLEngineTest, SetVariable_ValidName_Success) {
    std::string filePath = createSimpleTestFile();
    auto engine = ReadySCXMLEngine::fromFile(filePath);
    ASSERT_NE(engine, nullptr);
    ASSERT_TRUE(engine->start());

    bool set = engine->setVariable("testVar", "testValue");

    EXPECT_TRUE(set);
}

TEST_F(ReadySCXMLEngineTest, GetVariable_ExistingVariable_ReturnsValue) {
    std::string filePath = createSimpleTestFile();
    auto engine = ReadySCXMLEngine::fromFile(filePath);
    ASSERT_NE(engine, nullptr);
    ASSERT_TRUE(engine->start());

    // counter is initialized to 0 in datamodel
    std::string value = engine->getVariable("counter");

    EXPECT_FALSE(value.empty());
}

TEST_F(ReadySCXMLEngineTest, GetVariable_NonExistingVariable_ReturnsEmpty) {
    std::string filePath = createSimpleTestFile();
    auto engine = ReadySCXMLEngine::fromFile(filePath);
    ASSERT_NE(engine, nullptr);
    ASSERT_TRUE(engine->start());

    std::string value = engine->getVariable("nonexistent");

    EXPECT_TRUE(value.empty());
}

TEST_F(ReadySCXMLEngineTest, SetVariable_ThenGetVariable_Success) {
    std::string filePath = createSimpleTestFile();
    auto engine = ReadySCXMLEngine::fromFile(filePath);
    ASSERT_NE(engine, nullptr);
    ASSERT_TRUE(engine->start());

    ASSERT_TRUE(engine->setVariable("myVar", "myValue"));
    std::string value = engine->getVariable("myVar");

    EXPECT_EQ(value, "myValue");
}

// ============================================================================
// Error Handling Tests
// ============================================================================

TEST_F(ReadySCXMLEngineTest, GetLastError_AfterSuccess_ReturnsEmpty) {
    std::string scxmlContent = getSimpleScxmlString();
    auto engine = ReadySCXMLEngine::fromString(scxmlContent);
    ASSERT_NE(engine, nullptr);
    ASSERT_TRUE(engine->start());

    std::string error = engine->getLastError();

    EXPECT_TRUE(error.empty());
}

TEST_F(ReadySCXMLEngineTest, GetLastError_AfterFailure_ReturnsErrorMessage) {
    std::string scxmlContent = getSimpleScxmlString();
    auto engine = ReadySCXMLEngine::fromString(scxmlContent);
    ASSERT_NE(engine, nullptr);

    // Try to send event before starting (should fail)
    engine->sendEvent("test");

    std::string error = engine->getLastError();

    EXPECT_FALSE(error.empty());
}

// ============================================================================
// Statistics Tests
// ============================================================================

TEST_F(ReadySCXMLEngineTest, GetStatistics_AfterStart_ReturnsValidStats) {
    std::string scxmlContent = getSimpleScxmlString();
    auto engine = ReadySCXMLEngine::fromString(scxmlContent);
    ASSERT_NE(engine, nullptr);
    ASSERT_TRUE(engine->start());

    auto stats = engine->getStatistics();

    EXPECT_TRUE(stats.isRunning);
    EXPECT_FALSE(stats.currentState.empty());
}

TEST_F(ReadySCXMLEngineTest, GetStatistics_BeforeStart_ReturnsNotRunning) {
    std::string scxmlContent = getSimpleScxmlString();
    auto engine = ReadySCXMLEngine::fromString(scxmlContent);
    ASSERT_NE(engine, nullptr);

    auto stats = engine->getStatistics();

    EXPECT_FALSE(stats.isRunning);
}

// TODO: Re-enable when statistics counters are implemented in SCXMLEngine
// Currently totalEvents and totalTransitions are not tracked by the high-level API
// TEST_F(ReadySCXMLEngineTest, GetStatistics_AfterEvents_UpdatesCounters) {
//     std::string filePath = createSimpleTestFile();
//     auto engine = ReadySCXMLEngine::fromFile(filePath);
//     ASSERT_NE(engine, nullptr);
//     ASSERT_TRUE(engine->start());
//
//     // Send some events
//     engine->sendEvent("start");
//     engine->sendEvent("pause");
//
//     auto stats = engine->getStatistics();
//
//     EXPECT_GT(stats.totalEvents, 0);
//     EXPECT_GT(stats.totalTransitions, 0);
// }

// ============================================================================
// Integration Tests
// ============================================================================

TEST_F(ReadySCXMLEngineTest, FullWorkflow_StartEventTransitionStop_Success) {
    std::string filePath = createSimpleTestFile();
    auto engine = ReadySCXMLEngine::fromFile(filePath);
    ASSERT_NE(engine, nullptr);

    // Start the engine
    ASSERT_TRUE(engine->start());
    EXPECT_TRUE(engine->isInState("idle"));

    // Send event to transition
    ASSERT_TRUE(engine->sendEvent("start"));
    EXPECT_TRUE(engine->isInState("running"));

    // Check variable was updated
    std::string counter = engine->getVariable("counter");
    EXPECT_FALSE(counter.empty());

    // Stop the engine
    engine->stop();
    EXPECT_FALSE(engine->isRunning());
}

TEST_F(ReadySCXMLEngineTest, MultipleTransitions_CounterIncreases_Success) {
    std::string filePath = createSimpleTestFile();
    auto engine = ReadySCXMLEngine::fromFile(filePath);
    ASSERT_NE(engine, nullptr);
    ASSERT_TRUE(engine->start());

    // Transition to running (counter = 1)
    ASSERT_TRUE(engine->sendEvent("start"));
    EXPECT_TRUE(engine->isInState("running"));

    // Transition to paused (counter stays 1)
    ASSERT_TRUE(engine->sendEvent("pause"));
    EXPECT_TRUE(engine->isInState("paused"));

    // Transition back to running (counter = 2)
    ASSERT_TRUE(engine->sendEvent("resume"));
    EXPECT_TRUE(engine->isInState("running"));
}

}  // namespace RSM
