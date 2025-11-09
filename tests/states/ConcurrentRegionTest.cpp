#include "states/ConcurrentRegion.h"
#include "actions/AssignAction.h"
#include "actions/ScriptAction.h"
#include "mocks/MockActionExecutor.h"
#include "model/StateNode.h"
#include "runtime/ExecutionContextImpl.h"
#include "gtest/gtest.h"
#include <memory>
#include <stdexcept>

namespace SCE {

class ConcurrentRegionTest : public ::testing::Test {
protected:
    void SetUp() override {
        mockExecutor = std::make_shared<SCE::Test::MockActionExecutor>("test_session");
        executionContext = std::make_shared<ExecutionContextImpl>(mockExecutor, "test_session");
        rootState = std::make_shared<StateNode>("rootState", Type::COMPOUND);
        region = std::make_unique<ConcurrentRegion>("testRegion", rootState, executionContext);
    }

    void TearDown() override {
        mockExecutor->clearHistory();
    }

    std::shared_ptr<SCE::Test::MockActionExecutor> mockExecutor;
    std::shared_ptr<ExecutionContextImpl> executionContext;
    std::shared_ptr<StateNode> rootState;
    std::unique_ptr<ConcurrentRegion> region;
};

// Test activation with no entry actions
TEST_F(ConcurrentRegionTest, NoEntryActions_ActivationSucceeds) {
    // Create a state with no entry actions
    auto testState = std::make_shared<StateNode>("testState", Type::ATOMIC);

    // No entry actions added - should handle gracefully
    region->setRootState(testState);

    // Activation should succeed even with no entry actions
    auto result = region->activate();
    EXPECT_TRUE(result.isSuccess) << "Activation should succeed even with no entry actions";

    // Verify no scripts were executed (because there were no actions)
    EXPECT_EQ(mockExecutor->getExecutedScripts().size(), 0) << "No scripts should be executed when no actions exist";
}

// Test executeActionNode with action that throws exception
TEST_F(ConcurrentRegionTest, ExecuteActionNode_ActionExecuteThrowsException_HandledGracefully) {
    // Create a script action that will cause an exception in the mock executor
    auto scriptAction = std::make_shared<ScriptAction>("throw new Error('test exception')", "exception_action");

    auto testState = std::make_shared<StateNode>("testState", Type::ATOMIC);
    // W3C SCXML 3.8: Use block-based API
    std::vector<std::shared_ptr<IActionNode>> entryBlock = {scriptAction};
    testState->addEntryActionBlock(entryBlock);

    // Configure mock to throw exception for this script
    mockExecutor->setScriptExecutionResult(false);  // Make it fail

    region->setRootState(testState);

    // Activation should still succeed (error handled gracefully)
    auto result = region->activate();
    EXPECT_TRUE(result.isSuccess) << "Activation should succeed even when entry action fails";

    // Verify the action was attempted
    const auto &executedScripts = mockExecutor->getExecutedScripts();
    EXPECT_EQ(executedScripts.size(), 1) << "Script should have been attempted";
    EXPECT_EQ(executedScripts[0], "throw new Error('test exception')") << "Correct script should have been executed";
}

// Test executeActionNode with action that returns false
TEST_F(ConcurrentRegionTest, ExecuteActionNode_ActionExecuteReturnsFalse_HandledGracefully) {
    auto scriptAction = std::make_shared<ScriptAction>("return false", "false_action");

    auto testState = std::make_shared<StateNode>("testState", Type::ATOMIC);
    // W3C SCXML 3.8: Use block-based API
    std::vector<std::shared_ptr<IActionNode>> entryBlock = {scriptAction};
    testState->addEntryActionBlock(entryBlock);

    // Configure mock to return false
    mockExecutor->setScriptExecutionResult(false);

    region->setRootState(testState);

    // Activation should still succeed (failure handled gracefully)
    auto result = region->activate();
    EXPECT_TRUE(result.isSuccess) << "Activation should succeed even when entry action returns false";

    // Verify the action was attempted
    const auto &executedScripts = mockExecutor->getExecutedScripts();
    EXPECT_EQ(executedScripts.size(), 1) << "Script should have been attempted";
    EXPECT_EQ(executedScripts[0], "return false") << "Correct script should have been executed";
}

// Test executeActionNode with valid ScriptAction
TEST_F(ConcurrentRegionTest, ExecuteActionNode_ValidScriptAction_ReturnsTrue) {
    auto scriptAction = std::make_shared<ScriptAction>("console.log('test')", "script_action");

    auto testState = std::make_shared<StateNode>("testState", Type::ATOMIC);
    // W3C SCXML 3.8: Use block-based API
    std::vector<std::shared_ptr<IActionNode>> entryBlock = {scriptAction};
    testState->addEntryActionBlock(entryBlock);

    // Configure mock to succeed
    mockExecutor->setScriptExecutionResult(true);

    region->setRootState(testState);

    // Activation should succeed
    auto result = region->activate();
    EXPECT_TRUE(result.isSuccess) << "Activation should succeed with valid script action";

    // Verify the action was executed
    const auto &executedScripts = mockExecutor->getExecutedScripts();
    EXPECT_EQ(executedScripts.size(), 1) << "Script should have been executed";
    EXPECT_EQ(executedScripts[0], "console.log('test')") << "Correct script should have been executed";
}

// Test executeActionNode with valid AssignAction
TEST_F(ConcurrentRegionTest, ExecuteActionNode_ValidAssignAction_ReturnsTrue) {
    auto assignAction = std::make_shared<AssignAction>("testVar", "42", "assign_action");

    auto testState = std::make_shared<StateNode>("testState", Type::ATOMIC);
    // W3C SCXML 3.8: Use block-based API
    std::vector<std::shared_ptr<IActionNode>> entryBlock = {assignAction};
    testState->addEntryActionBlock(entryBlock);

    // Configure mock to succeed for variable assignment
    mockExecutor->setVariableAssignmentResult(true);

    region->setRootState(testState);

    // Activation should succeed
    auto result = region->activate();
    EXPECT_TRUE(result.isSuccess) << "Activation should succeed with valid assign action";

    // Verify the assignment was executed
    const auto &assignments = mockExecutor->getAssignedVariables();
    EXPECT_EQ(assignments.size(), 1) << "Variable assignment should have been executed";
    EXPECT_EQ(assignments.at("testVar"), "42") << "Correct variable should have been assigned";
}

// Test activation with null ExecutionContext
TEST_F(ConcurrentRegionTest, NullExecutionContext_ActivationSucceeds) {
    auto scriptAction = std::make_shared<ScriptAction>("console.log('test')", "script_action");

    auto testState = std::make_shared<StateNode>("testState", Type::ATOMIC);
    // W3C SCXML 3.8: Use block-based API
    std::vector<std::shared_ptr<IActionNode>> entryBlock = {scriptAction};
    testState->addEntryActionBlock(entryBlock);

    // Create region without execution context
    auto regionWithoutContext = std::make_unique<ConcurrentRegion>("testRegion", testState, nullptr);

    // Activation should succeed without crashing
    // Note: We cannot directly verify action execution was skipped because mockExecutor
    // is not connected to regionWithoutContext. The test passes if activation succeeds
    // without throwing exceptions, which indicates graceful handling of nullptr context.
    auto result = regionWithoutContext->activate();
    EXPECT_TRUE(result.isSuccess) << "Activation should succeed even without execution context";
}

// Test multiple entry actions execution order
TEST_F(ConcurrentRegionTest, ExecuteActionNode_MultipleActions_ExecutedInOrder) {
    auto action1 = std::make_shared<ScriptAction>("action1", "action_1");
    auto action2 = std::make_shared<ScriptAction>("action2", "action_2");
    auto action3 = std::make_shared<ScriptAction>("action3", "action_3");

    auto testState = std::make_shared<StateNode>("testState", Type::ATOMIC);
    // W3C SCXML 3.8: Use block-based API - each action in same block
    std::vector<std::shared_ptr<IActionNode>> entryBlock = {action1, action2, action3};
    testState->addEntryActionBlock(entryBlock);

    mockExecutor->setScriptExecutionResult(true);

    region->setRootState(testState);

    auto result = region->activate();
    EXPECT_TRUE(result.isSuccess) << "Activation should succeed with multiple actions";

    // Verify execution order
    const auto &executedScripts = mockExecutor->getExecutedScripts();
    EXPECT_EQ(executedScripts.size(), 3) << "All three actions should have been executed";
    EXPECT_EQ(executedScripts[0], "action1") << "First action should execute first";
    EXPECT_EQ(executedScripts[1], "action2") << "Second action should execute second";
    EXPECT_EQ(executedScripts[2], "action3") << "Third action should execute third";
}

// Test mixed action types execution
TEST_F(ConcurrentRegionTest, ExecuteActionNode_MixedActionTypes_AllExecuted) {
    auto scriptAction = std::make_shared<ScriptAction>("console.log('script')", "script_action");
    auto assignAction = std::make_shared<AssignAction>("var1", "value1", "assign_action");

    auto testState = std::make_shared<StateNode>("testState", Type::ATOMIC);
    // W3C SCXML 3.8: Use block-based API - mixed actions in same block
    std::vector<std::shared_ptr<IActionNode>> entryBlock = {scriptAction, assignAction};
    testState->addEntryActionBlock(entryBlock);

    mockExecutor->setScriptExecutionResult(true);
    mockExecutor->setVariableAssignmentResult(true);

    region->setRootState(testState);

    auto result = region->activate();
    EXPECT_TRUE(result.isSuccess) << "Activation should succeed with mixed action types";

    // Verify both types were executed
    const auto &executedScripts = mockExecutor->getExecutedScripts();
    const auto &assignments = mockExecutor->getAssignedVariables();

    EXPECT_EQ(executedScripts.size(), 1) << "Script action should have been executed";
    EXPECT_EQ(executedScripts[0], "console.log('script')") << "Correct script should have been executed";

    EXPECT_EQ(assignments.size(), 1) << "Assignment action should have been executed";
    EXPECT_EQ(assignments.at("var1"), "value1") << "Correct assignment should have been made";
}

// ============================================================================
// W3C SCXML 3.8: Exit Actions Tests
// ============================================================================

// Test deactivate with valid exit actions
TEST_F(ConcurrentRegionTest, DeactivateWithValidExitActions) {
    auto scriptAction = std::make_shared<ScriptAction>("console.log('exiting')", "exit_action");

    auto testState = std::make_shared<StateNode>("testState", Type::ATOMIC);
    // W3C SCXML 3.8: Use block-based API for exit actions
    std::vector<std::shared_ptr<IActionNode>> exitBlock = {scriptAction};
    testState->addExitActionBlock(exitBlock);

    mockExecutor->setScriptExecutionResult(true);
    region->setRootState(testState);

    // First activate the region
    auto activateResult = region->activate();
    EXPECT_TRUE(activateResult.isSuccess) << "Activation should succeed";
    EXPECT_TRUE(region->isActive()) << "Region should be active";

    // Clear history to isolate exit actions
    mockExecutor->clearHistory();

    // Now deactivate and verify exit actions
    auto deactivateResult = region->deactivate(executionContext);
    EXPECT_TRUE(deactivateResult.isSuccess) << "Deactivation should succeed with valid exit action";
    EXPECT_FALSE(region->isActive()) << "Region should be inactive after deactivation";

    // Verify the exit action was executed
    const auto &executedScripts = mockExecutor->getExecutedScripts();
    EXPECT_EQ(executedScripts.size(), 1) << "Exit script should have been executed";
    EXPECT_EQ(executedScripts[0], "console.log('exiting')") << "Correct exit script should have been executed";
}

// Test deactivate with multiple exit actions in order
TEST_F(ConcurrentRegionTest, DeactivateWithMultipleExitActions) {
    auto action1 = std::make_shared<ScriptAction>("exit1", "exit_1");
    auto action2 = std::make_shared<ScriptAction>("exit2", "exit_2");
    auto action3 = std::make_shared<ScriptAction>("exit3", "exit_3");

    auto testState = std::make_shared<StateNode>("testState", Type::ATOMIC);
    // W3C SCXML 3.8: Use block-based API - all exit actions in same block
    std::vector<std::shared_ptr<IActionNode>> exitBlock = {action1, action2, action3};
    testState->addExitActionBlock(exitBlock);

    mockExecutor->setScriptExecutionResult(true);
    region->setRootState(testState);

    // Activate first
    region->activate();
    mockExecutor->clearHistory();

    // Deactivate and verify
    auto result = region->deactivate(executionContext);
    EXPECT_TRUE(result.isSuccess) << "Deactivation should succeed with multiple exit actions";

    // Verify execution order (W3C SCXML 3.13: document order)
    const auto &executedScripts = mockExecutor->getExecutedScripts();
    EXPECT_EQ(executedScripts.size(), 3) << "All three exit actions should have been executed";
    EXPECT_EQ(executedScripts[0], "exit1") << "First exit action should execute first";
    EXPECT_EQ(executedScripts[1], "exit2") << "Second exit action should execute second";
    EXPECT_EQ(executedScripts[2], "exit3") << "Third exit action should execute third";
}

// Test deactivate with mixed action types
TEST_F(ConcurrentRegionTest, DeactivateWithMixedExitActionTypes) {
    auto scriptAction = std::make_shared<ScriptAction>("console.log('exit')", "exit_script");
    auto assignAction = std::make_shared<AssignAction>("exitVar", "exitValue", "exit_assign");

    auto testState = std::make_shared<StateNode>("testState", Type::ATOMIC);
    // W3C SCXML 3.8: Mixed exit actions in same block
    std::vector<std::shared_ptr<IActionNode>> exitBlock = {scriptAction, assignAction};
    testState->addExitActionBlock(exitBlock);

    mockExecutor->setScriptExecutionResult(true);
    mockExecutor->setVariableAssignmentResult(true);
    region->setRootState(testState);

    // Activate first
    region->activate();
    mockExecutor->clearHistory();

    // Deactivate and verify
    auto result = region->deactivate(executionContext);
    EXPECT_TRUE(result.isSuccess) << "Deactivation should succeed with mixed exit action types";

    // Verify both types were executed
    const auto &executedScripts = mockExecutor->getExecutedScripts();
    const auto &assignments = mockExecutor->getAssignedVariables();

    EXPECT_EQ(executedScripts.size(), 1) << "Exit script action should have been executed";
    EXPECT_EQ(executedScripts[0], "console.log('exit')") << "Correct exit script should have been executed";

    EXPECT_EQ(assignments.size(), 1) << "Exit assignment action should have been executed";
    EXPECT_EQ(assignments.at("exitVar"), "exitValue") << "Correct exit assignment should have been made";
}

// Test deactivate with null ExecutionContext
TEST_F(ConcurrentRegionTest, DeactivateWithNullExecutionContext) {
    auto scriptAction = std::make_shared<ScriptAction>("console.log('exit')", "exit_action");

    auto testState = std::make_shared<StateNode>("testState", Type::ATOMIC);
    // W3C SCXML 3.8: Use block-based API
    std::vector<std::shared_ptr<IActionNode>> exitBlock = {scriptAction};
    testState->addExitActionBlock(exitBlock);

    mockExecutor->setScriptExecutionResult(true);
    region->setRootState(testState);

    // Activate first
    region->activate();
    mockExecutor->clearHistory();

    // Deactivate with nullptr ExecutionContext - should skip exit actions gracefully
    auto result = region->deactivate(nullptr);
    EXPECT_TRUE(result.isSuccess) << "Deactivation should succeed even with null execution context";
    EXPECT_FALSE(region->isActive()) << "Region should be inactive after deactivation";

    // Verify exit actions were skipped (StateExitExecutor skips actions when executionContext is null)
    const auto &executedScripts = mockExecutor->getExecutedScripts();
    EXPECT_EQ(executedScripts.size(), 0) << "Exit actions should be skipped when executionContext is null";
}

// Test deactivate when already inactive
TEST_F(ConcurrentRegionTest, DeactivateWhenAlreadyInactive) {
    auto testState = std::make_shared<StateNode>("testState", Type::ATOMIC);
    region->setRootState(testState);

    // Region starts inactive, try to deactivate
    EXPECT_FALSE(region->isActive()) << "Region should start inactive";

    auto result = region->deactivate(executionContext);
    EXPECT_TRUE(result.isSuccess) << "Deactivation should succeed even when already inactive";
    EXPECT_FALSE(region->isActive()) << "Region should remain inactive";
}

// ============================================================================
// W3C SCXML 3.2: State Lifecycle Tests
// ============================================================================

// Test reset functionality
TEST_F(ConcurrentRegionTest, Reset_ResetsToInactiveState) {
    auto testState = std::make_shared<StateNode>("testState", Type::ATOMIC);
    region->setRootState(testState);

    // Activate the region
    region->activate();
    EXPECT_TRUE(region->isActive()) << "Region should be active after activation";

    // Reset the region
    auto result = region->reset();
    EXPECT_TRUE(result.isSuccess) << "Reset should succeed";
    EXPECT_FALSE(region->isActive()) << "Region should be inactive after reset";
    EXPECT_EQ(region->getCurrentState(), "") << "Current state should be cleared after reset";
    EXPECT_FALSE(region->isInFinalState()) << "Should not be in final state after reset";
}

// Test getActiveStates returns current configuration
TEST_F(ConcurrentRegionTest, GetActiveStates_ReturnsCurrentConfiguration) {
    auto testState = std::make_shared<StateNode>("testState", Type::ATOMIC);
    region->setRootState(testState);

    // Initially inactive, no active states
    auto activeStatesInactive = region->getActiveStates();
    EXPECT_TRUE(activeStatesInactive.empty()) << "Active states should be empty when inactive";

    // Activate and check active states
    region->activate();
    auto activeStatesActive = region->getActiveStates();
    EXPECT_FALSE(activeStatesActive.empty()) << "Active states should not be empty when active";
    EXPECT_EQ(region->getCurrentState(), "testState") << "Current state should be testState";

    // Deactivate and check again
    region->deactivate(executionContext);
    auto activeStatesAfterDeactivate = region->getActiveStates();
    EXPECT_TRUE(activeStatesAfterDeactivate.empty()) << "Active states should be empty after deactivation";
}

// ============================================================================
// W3C SCXML 3.4: Final State Detection Tests
// ============================================================================

// Test isInFinalState after entering final state
TEST_F(ConcurrentRegionTest, IsInFinalState_DetectsFinalState) {
    auto finalState = std::make_shared<StateNode>("finalState", Type::FINAL);
    region->setRootState(finalState);

    // Initially not in final state
    EXPECT_FALSE(region->isInFinalState()) << "Should not be in final state when inactive";

    // Activate - final state should be detected automatically
    region->activate();

    // W3C SCXML 3.4: Final state is detected when region enters a final state
    EXPECT_TRUE(region->isInFinalState()) << "Should be in final state after activating with FINAL root state";
}

// Test isInFinalState before entering final state
TEST_F(ConcurrentRegionTest, IsInFinalState_FalseForNonFinalState) {
    auto normalState = std::make_shared<StateNode>("normalState", Type::ATOMIC);
    region->setRootState(normalState);

    // Activate normal state
    region->activate();

    EXPECT_FALSE(region->isInFinalState()) << "Should not be in final state for normal atomic state";
    EXPECT_TRUE(region->isActive()) << "Region should be active";
}

// ============================================================================
// W3C SCXML Compliance: Validation Tests
// ============================================================================

// Test validate with null root state
TEST_F(ConcurrentRegionTest, Validate_DetectsNullRootState) {
    // Create region without root state
    auto regionNoRoot = std::make_unique<ConcurrentRegion>("testRegion", nullptr, executionContext);

    auto errors = regionNoRoot->validate();
    EXPECT_FALSE(errors.empty()) << "Validation should fail for null root state";

    bool foundRootStateError = false;
    for (const auto &error : errors) {
        if (error.find("root state") != std::string::npos || error.find("rootState") != std::string::npos) {
            foundRootStateError = true;
            break;
        }
    }
    EXPECT_TRUE(foundRootStateError) << "Should have error about missing root state";
}

// Test validate with valid configuration
TEST_F(ConcurrentRegionTest, Validate_PassesForValidConfiguration) {
    auto testState = std::make_shared<StateNode>("testState", Type::ATOMIC);
    region->setRootState(testState);

    auto errors = region->validate();

    // Valid configuration should have no validation errors
    if (!errors.empty()) {
        for (const auto &error : errors) {
            LOG_ERROR("Unexpected validation error: {}", error);
        }
    }

    EXPECT_TRUE(errors.empty()) << "Valid configuration should pass validation with no errors";
}

}  // namespace SCE