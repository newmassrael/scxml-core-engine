#include "runtime/StateMachine.h"
#include "common/Logger.h"
#include "runtime/StateMachineFactory.h"
#include <fstream>
#include <gtest/gtest.h>

using namespace SCE;

class StateMachineTest : public ::testing::Test {
protected:
    void SetUp() override {
        // Setup test environment
        // Logger::setLevel(Logger::Level::DEBUG); // Logger API verification needed
    }

    void TearDown() override {
        // Cleanup
    }

    // Helper to create a simple SCXML document
    std::string createSimpleSCXML() {
        return R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="idle">
    <state id="idle">
        <transition event="start" target="running"/>
    </state>
    
    <state id="running">
        <transition event="stop" target="idle"/>
        <transition event="finish" target="done"/>
    </state>
    
    <final id="done"/>
</scxml>)";
    }

    // Helper to create SCXML with JavaScript guards/actions
    std::string createSCXMLWithJS() {
        return R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="waiting">
    <datamodel>
        <data id="counter" expr="0"/>
    </datamodel>
    
    <state id="waiting">
        <onentry>
            <script>counter = 0;</script>
        </onentry>
        <transition event="increment" cond="counter &lt; 5" target="counting">
            <script>counter = counter + 1;</script>
        </transition>
        <transition event="increment" cond="counter >= 5" target="finished"/>
    </state>
    
    <state id="counting">
        <transition event="increment" cond="counter &lt; 5" target="counting">
            <script>counter = counter + 1;</script>
        </transition>
        <transition event="increment" cond="counter >= 5" target="finished"/>
        <transition event="reset" target="waiting"/>
    </state>
    
    <final id="finished"/>
</scxml>)";
    }

    // Helper to create SCXML for C++ binding test
    std::string createSCXMLWithCppBinding() {
        return R"SCXML(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="init">
    <state id="init">
        <transition event="check_temp" cond="hardware.isTemperatureHigh()" target="cooling"/>
        <transition event="check_temp" target="normal"/>
    </state>

    <state id="normal">
        <onentry>
            <script>hardware.setStatus("Normal operation");</script>
        </onentry>
        <transition event="check_temp" cond="hardware.isTemperatureHigh()" target="cooling"/>
    </state>

    <state id="cooling">
        <onentry>
            <script>hardware.startCooling();</script>
        </onentry>
        <transition event="check_temp" cond="!hardware.isTemperatureHigh()" target="normal"/>
    </state>
</scxml>)SCXML";
    }
};

// Mock hardware class for C++ binding tests
class MockHardware {
public:
    bool isTemperatureHigh() const {
        return temperature_ > 30.0;
    }

    void setTemperature(double temp) {
        temperature_ = temp;
    }

    void startCooling() {
        cooling_ = true;
        status_ = "Cooling active";
    }

    void setStatus(const std::string &status) {
        status_ = status;
    }

    double getTemperature() const {
        return temperature_;
    }

    bool isCooling() const {
        return cooling_;
    }

    std::string getStatus() const {
        return status_;
    }

private:
    double temperature_ = 25.0;
    bool cooling_ = false;
    std::string status_ = "Unknown";
};

// Basic functionality tests
TEST_F(StateMachineTest, Constructor) {
    // Default constructor succeeds safely
    StateMachine sm;
    EXPECT_FALSE(sm.isRunning());
    EXPECT_TRUE(sm.getCurrentState().empty());
    EXPECT_TRUE(sm.getActiveStates().empty());

    // Verify statistics show initial state
    auto stats = sm.getStatistics();
    EXPECT_EQ(stats.totalTransitions, 0);
    EXPECT_EQ(stats.totalEvents, 0);
    EXPECT_FALSE(stats.isRunning);
}

TEST_F(StateMachineTest, FactoryPattern_CreateProduction) {
    // Verify factory creates production StateMachine instance
    auto result = StateMachineFactory::createProduction();

    // Factory should succeed
    ASSERT_TRUE(result.has_value()) << "Factory failed: " << result.error;
    EXPECT_FALSE(result.value->isRunning());

    // Result.value is already shared_ptr (StateMachine requires shared_ptr for enable_shared_from_this)
    auto sm = std::move(result.value);

    // Verify created instance is functional
    std::string scxml = createSimpleSCXML();
    EXPECT_TRUE(sm->loadSCXMLFromString(scxml));
    EXPECT_TRUE(sm->start());
    EXPECT_EQ(sm->getCurrentState(), "idle");
}

TEST_F(StateMachineTest, LoadSimpleSCXML) {
    auto sm = std::make_shared<StateMachine>();
    std::string scxml = createSimpleSCXML();

    EXPECT_TRUE(sm->loadSCXMLFromString(scxml));

    // Verify loaded SCXML is functional
    EXPECT_TRUE(sm->start());
    EXPECT_TRUE(sm->isRunning());
    EXPECT_EQ(sm->getCurrentState(), "idle");
}

TEST_F(StateMachineTest, StartStateMachine) {
    auto sm = std::make_shared<StateMachine>();
    std::string scxml = createSimpleSCXML();

    ASSERT_TRUE(sm->loadSCXMLFromString(scxml));
    EXPECT_TRUE(sm->start());
    EXPECT_TRUE(sm->isRunning());
    EXPECT_EQ(sm->getCurrentState(), "idle");
    EXPECT_TRUE(sm->isStateActive("idle"));
}

TEST_F(StateMachineTest, BasicTransition) {
    auto sm = std::make_shared<StateMachine>();
    std::string scxml = createSimpleSCXML();

    ASSERT_TRUE(sm->loadSCXMLFromString(scxml));
    ASSERT_TRUE(sm->start());

    // Test transition from idle to running
    auto result = sm->processEvent("start");
    EXPECT_TRUE(result.success);
    EXPECT_EQ(result.fromState, "idle");
    EXPECT_EQ(result.toState, "running");
    EXPECT_EQ(result.eventName, "start");
    EXPECT_EQ(sm->getCurrentState(), "running");
}

TEST_F(StateMachineTest, InvalidEvent) {
    auto sm = std::make_shared<StateMachine>();
    std::string scxml = createSimpleSCXML();

    ASSERT_TRUE(sm->loadSCXMLFromString(scxml));
    ASSERT_TRUE(sm->start());

    // Test invalid event
    auto result = sm->processEvent("invalid_event");
    EXPECT_FALSE(result.success);
    EXPECT_FALSE(result.errorMessage.empty());
    EXPECT_EQ(sm->getCurrentState(), "idle");  // Should stay in same state
}

TEST_F(StateMachineTest, MultipleTransitions) {
    auto sm = std::make_shared<StateMachine>();
    std::string scxml = createSimpleSCXML();

    ASSERT_TRUE(sm->loadSCXMLFromString(scxml));
    ASSERT_TRUE(sm->start());

    // idle -> running
    auto result1 = sm->processEvent("start");
    EXPECT_TRUE(result1.success);
    EXPECT_EQ(sm->getCurrentState(), "running");

    // running -> idle
    auto result2 = sm->processEvent("stop");
    EXPECT_TRUE(result2.success);
    EXPECT_EQ(sm->getCurrentState(), "idle");

    // idle -> running -> done
    sm->processEvent("start");
    auto result3 = sm->processEvent("finish");
    EXPECT_TRUE(result3.success);
    EXPECT_EQ(sm->getCurrentState(), "done");
}

TEST_F(StateMachineTest, StopStateMachine) {
    auto sm = std::make_shared<StateMachine>();
    std::string scxml = createSimpleSCXML();

    ASSERT_TRUE(sm->loadSCXMLFromString(scxml));
    ASSERT_TRUE(sm->start());

    sm->stop();
    EXPECT_FALSE(sm->isRunning());
    EXPECT_TRUE(sm->getCurrentState().empty());
    EXPECT_TRUE(sm->getActiveStates().empty());
}

TEST_F(StateMachineTest, Statistics) {
    auto sm = std::make_shared<StateMachine>();
    std::string scxml = createSimpleSCXML();

    ASSERT_TRUE(sm->loadSCXMLFromString(scxml));
    ASSERT_TRUE(sm->start());

    auto stats1 = sm->getStatistics();
    EXPECT_EQ(stats1.totalTransitions, 0);
    EXPECT_EQ(stats1.totalEvents, 0);
    EXPECT_TRUE(stats1.isRunning);

    // Make some transitions with verification
    auto result1 = sm->processEvent("start");
    EXPECT_TRUE(result1.success);
    EXPECT_EQ(sm->getCurrentState(), "running");

    auto result2 = sm->processEvent("stop");
    EXPECT_TRUE(result2.success);
    EXPECT_EQ(sm->getCurrentState(), "idle");

    auto result3 = sm->processEvent("invalid");
    EXPECT_FALSE(result3.success);             // This should fail
    EXPECT_EQ(sm->getCurrentState(), "idle");  // Should stay in same state

    auto stats2 = sm->getStatistics();
    EXPECT_EQ(stats2.totalTransitions, 2);
    EXPECT_EQ(stats2.failedTransitions, 1);
    EXPECT_EQ(stats2.totalEvents, 3);
}

// JavaScript integration tests
TEST_F(StateMachineTest, JavaScriptDatamodel) {
    // Test JavaScript datamodel (W3C SCXML 5.2)
    // - Data variable initialization and modification
    // - Conditional guards (cond attribute)
    // - Script actions (onentry, transition)
    auto sm = std::make_shared<StateMachine>();
    std::string scxml = createSCXMLWithJS();

    ASSERT_TRUE(sm->loadSCXMLFromString(scxml));
    ASSERT_TRUE(sm->start());
    EXPECT_EQ(sm->getCurrentState(), "waiting");

    // First few increments should go to counting
    for (int i = 0; i < 5; i++) {
        auto result = sm->processEvent("increment");
        EXPECT_TRUE(result.success);
        // Should be in counting state after first increment
        if (i == 0) {
            EXPECT_EQ(sm->getCurrentState(), "counting");
        }
    }

    // 6th increment should go to finished (counter will be 5, so counter >= 5 condition triggers)
    auto result = sm->processEvent("increment");
    EXPECT_TRUE(result.success);
    EXPECT_EQ(sm->getCurrentState(), "finished");
}

// C++ binding tests
TEST_F(StateMachineTest, CppObjectBinding) {
    auto sm = std::make_shared<StateMachine>();
    MockHardware hardware;

    std::string scxml = createSCXMLWithCppBinding();

    ASSERT_TRUE(sm->loadSCXMLFromString(scxml));

    // Bind C++ object with method registration
    sm->bindObject("hardware", &hardware, [](auto &binder) {
        binder.def("getTemperature", &MockHardware::getTemperature)
            .def("setTemperature", &MockHardware::setTemperature)
            .def("isTemperatureHigh", &MockHardware::isTemperatureHigh)
            .def("startCooling", &MockHardware::startCooling)
            .def("setStatus", &MockHardware::setStatus)
            .def("getStatus", &MockHardware::getStatus)
            .def("isCooling", &MockHardware::isCooling);
    });

    ASSERT_TRUE(sm->start());
    EXPECT_EQ(sm->getCurrentState(), "init");

    // Test with low temperature
    hardware.setTemperature(25.0);
    auto result1 = sm->processEvent("check_temp");
    EXPECT_TRUE(result1.success);
    EXPECT_EQ(sm->getCurrentState(), "normal");
    EXPECT_EQ(hardware.getStatus(), "Normal operation");

    // Test with high temperature
    hardware.setTemperature(35.0);
    auto result2 = sm->processEvent("check_temp");
    EXPECT_TRUE(result2.success);
    EXPECT_EQ(sm->getCurrentState(), "cooling");
    EXPECT_TRUE(hardware.isCooling());
}

// Integration with existing JSEngine tests
TEST_F(StateMachineTest, ScriptExecutionBasic) {
    // Verify script execution affects state machine behavior (W3C SCXML 5.8)
    auto sm = std::make_shared<StateMachine>();

    // Create SCXML with script that affects transition logic
    std::string scxml = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="start">
    <datamodel>
        <data id="executed" expr="false"/>
    </datamodel>
    <state id="start">
        <onentry>
            <script>executed = true;</script>
        </onentry>
        <transition event="check" cond="executed" target="success"/>
        <transition event="check" target="failed"/>
    </state>
    <state id="success"/>
    <state id="failed"/>
</scxml>)";

    ASSERT_TRUE(sm->loadSCXMLFromString(scxml));
    ASSERT_TRUE(sm->start());

    // If script executed correctly, executed=true and transition should go to success
    auto result = sm->processEvent("check");
    EXPECT_TRUE(result.success);
    EXPECT_EQ(sm->getCurrentState(), "success");
    EXPECT_NE(sm->getCurrentState(), "failed");
}

// Error handling tests
TEST_F(StateMachineTest, InvalidSCXML) {
    StateMachine sm;

    std::string invalidScxml = "<?xml version='1.0'?><invalid>not scxml</invalid>";
    EXPECT_FALSE(sm.loadSCXMLFromString(invalidScxml));
}

TEST_F(StateMachineTest, EmptySCXML) {
    StateMachine sm;

    EXPECT_FALSE(sm.loadSCXMLFromString(""));
}

TEST_F(StateMachineTest, StartWithoutLoading) {
    StateMachine sm;

    EXPECT_FALSE(sm.start());
    EXPECT_FALSE(sm.isRunning());
}

TEST_F(StateMachineTest, ProcessEventWithoutStarting) {
    StateMachine sm;
    std::string scxml = createSimpleSCXML();

    ASSERT_TRUE(sm.loadSCXMLFromString(scxml));
    // Don't start the state machine

    auto result = sm.processEvent("start");
    EXPECT_FALSE(result.success);
    EXPECT_FALSE(result.errorMessage.empty());
}

// Final state and lifecycle tests
TEST_F(StateMachineTest, FinalStateReached) {
    // Verify behavior when final state is reached (W3C SCXML 3.7)
    auto sm = std::make_shared<StateMachine>();
    std::string scxml = createSimpleSCXML();

    ASSERT_TRUE(sm->loadSCXMLFromString(scxml));
    ASSERT_TRUE(sm->start());
    EXPECT_EQ(sm->getCurrentState(), "idle");

    // Transition to final state: idle -> running -> done
    auto result1 = sm->processEvent("start");
    EXPECT_TRUE(result1.success);
    EXPECT_EQ(sm->getCurrentState(), "running");

    auto result2 = sm->processEvent("finish");
    EXPECT_TRUE(result2.success);
    EXPECT_EQ(result2.toState, "done");
    EXPECT_EQ(sm->getCurrentState(), "done");

    // Critical: Verify final state stops execution
    // Note: When top-level final state is reached, state machine stops automatically
    EXPECT_FALSE(sm->isRunning());
}

TEST_F(StateMachineTest, RestartAfterStop) {
    // Verify state machine can restart after stop
    auto sm = std::make_shared<StateMachine>();
    std::string scxml = createSimpleSCXML();

    // First run
    ASSERT_TRUE(sm->loadSCXMLFromString(scxml));
    ASSERT_TRUE(sm->start());
    EXPECT_EQ(sm->getCurrentState(), "idle");

    auto result = sm->processEvent("start");
    EXPECT_TRUE(result.success);
    EXPECT_EQ(sm->getCurrentState(), "running");

    // Stop
    sm->stop();
    EXPECT_FALSE(sm->isRunning());
    EXPECT_TRUE(sm->getCurrentState().empty());

    // Restart - Critical: Can we restart?
    EXPECT_TRUE(sm->start());
    EXPECT_TRUE(sm->isRunning());
    EXPECT_EQ(sm->getCurrentState(), "idle");  // Should reset to initial state
}

TEST_F(StateMachineTest, CompletionCallback) {
    // Verify completion callback is invoked on final state
    auto sm = std::make_shared<StateMachine>();
    std::string scxml = createSimpleSCXML();

    ASSERT_TRUE(sm->loadSCXMLFromString(scxml));

    bool callbackInvoked = false;

    sm->setCompletionCallback([&]() { callbackInvoked = true; });

    ASSERT_TRUE(sm->start());
    sm->processEvent("start");
    sm->processEvent("finish");

    // Callback should be invoked when final state is reached
    EXPECT_TRUE(callbackInvoked);
    EXPECT_EQ(sm->getCurrentState(), "done");
    EXPECT_FALSE(sm->isRunning());
}

TEST_F(StateMachineTest, LoadSCXMLFromFile) {
    // Verify loading SCXML from file
    auto sm = std::make_shared<StateMachine>();

    // Create temporary SCXML file
    std::string tempPath = "/tmp/test_scxml_load.xml";
    std::ofstream ofs(tempPath);
    ofs << createSimpleSCXML();
    ofs.close();

    EXPECT_TRUE(sm->loadSCXML(tempPath));
    EXPECT_TRUE(sm->start());
    EXPECT_EQ(sm->getCurrentState(), "idle");

    // Verify functionality
    auto result = sm->processEvent("start");
    EXPECT_TRUE(result.success);
    EXPECT_EQ(sm->getCurrentState(), "running");

    // Cleanup
    std::remove(tempPath.c_str());
}