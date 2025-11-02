#include "actions/ForeachAction.h"
#include "actions/IfAction.h"
#include "factory/NodeFactory.h"
#include "model/SCXMLModel.h"
#include "parsing/SCXMLParser.h"
#include <gtest/gtest.h>
#include <sstream>

namespace RSM {
namespace Tests {

class SCXMLParserBasicTest : public ::testing::Test {
protected:
    void SetUp() override {
        nodeFactory_ = std::make_shared<NodeFactory>();
        parser_ = std::make_unique<SCXMLParser>(nodeFactory_);
    }

    std::shared_ptr<NodeFactory> nodeFactory_;
    std::unique_ptr<SCXMLParser> parser_;
};

// Test basic SCXML document parsing
TEST_F(SCXMLParserBasicTest, ParseSimpleStateMachine) {
    std::string scxmlContent = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="start">
    <state id="start">
        <transition event="go" target="end"/>
    </state>
    <final id="end"/>
</scxml>)";

    auto model = parser_->parseContent(scxmlContent);
    ASSERT_NE(model, nullptr);
    EXPECT_FALSE(parser_->hasErrors());

    // Verify basic model properties
    EXPECT_EQ(model->getInitialState(), "start");
}

// Test parser error handling
TEST_F(SCXMLParserBasicTest, ParseInvalidXML) {
    std::string invalidContent = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0">
    <state id="start"
        <transition event="go" target="end"/>
    </state>
</scxml>)";

    auto model = parser_->parseContent(invalidContent);
    EXPECT_EQ(model, nullptr);
    EXPECT_TRUE(parser_->hasErrors());

    auto errors = parser_->getErrorMessages();
    EXPECT_FALSE(errors.empty());
}

// Test state hierarchy parsing
TEST_F(SCXMLParserBasicTest, ParseNestedStates) {
    std::string scxmlContent = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="parent">
    <state id="parent" initial="child1">
        <state id="child1">
            <transition event="next" target="child2"/>
        </state>
        <state id="child2">
            <transition event="done" target="end"/>
        </state>
    </state>
    <final id="end"/>
</scxml>)";

    auto model = parser_->parseContent(scxmlContent);
    ASSERT_NE(model, nullptr);
    EXPECT_FALSE(parser_->hasErrors());

    // Verify parent state exists
    auto parentState = model->findStateById("parent");
    ASSERT_NE(parentState, nullptr) << "Parent state 'parent' not found";
    EXPECT_EQ(parentState->getId(), "parent");
    EXPECT_EQ(parentState->getInitialState(), "child1") << "Parent initial state incorrect";

    // Verify nested child states exist as children of parent
    auto children = parentState->getChildren();
    ASSERT_EQ(children.size(), 2) << "Parent should have exactly 2 children";

    // Verify child1 exists and has correct structure
    auto child1 = model->findStateById("child1");
    ASSERT_NE(child1, nullptr) << "Child state 'child1' not found";
    EXPECT_EQ(child1->getParent(), parentState) << "child1 parent pointer incorrect";

    // Verify child2 exists and has correct structure
    auto child2 = model->findStateById("child2");
    ASSERT_NE(child2, nullptr) << "Child state 'child2' not found";
    EXPECT_EQ(child2->getParent(), parentState) << "child2 parent pointer incorrect";

    // Verify final state exists at top level
    auto endState = model->findStateById("end");
    ASSERT_NE(endState, nullptr) << "Final state 'end' not found";
    EXPECT_TRUE(endState->isFinalState()) << "State 'end' should be final state";
}

// Test action parsing
TEST_F(SCXMLParserBasicTest, ParseActionsInTransitions) {
    std::string scxmlContent = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="start">
    <state id="start">
        <transition event="go" target="end">
            <script>console.log('transitioning');</script>
            <assign location="result" expr="'success'"/>
        </transition>
    </state>
    <final id="end"/>
</scxml>)";

    auto model = parser_->parseContent(scxmlContent);
    ASSERT_NE(model, nullptr);
    EXPECT_FALSE(parser_->hasErrors());

    // Verify start state exists
    auto startState = model->findStateById("start");
    ASSERT_NE(startState, nullptr) << "Start state not found";

    // Verify transition exists
    auto transitions = startState->getTransitions();
    ASSERT_EQ(transitions.size(), 1) << "Start state should have exactly 1 transition";

    auto transition = transitions[0];
    ASSERT_NE(transition, nullptr);
    EXPECT_EQ(transition->getEvent(), "go") << "Transition event incorrect";

    // Verify actions were parsed
    auto actions = transition->getActionNodes();
    ASSERT_EQ(actions.size(), 2) << "Transition should have exactly 2 actions (script + assign)";

    // Verify script action
    EXPECT_EQ(actions[0]->getActionType(), "script") << "First action should be script";

    // Verify assign action
    EXPECT_EQ(actions[1]->getActionType(), "assign") << "Second action should be assign";
}

// Test guard conditions
TEST_F(SCXMLParserBasicTest, ParseGuardConditions) {
    std::string scxmlContent = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="start">
    <datamodel>
        <data id="counter" expr="0"/>
    </datamodel>
    <state id="start">
        <transition event="increment" cond="counter &lt; 10" target="start">
            <assign location="counter" expr="counter + 1"/>
        </transition>
        <transition event="increment" cond="counter >= 10" target="end"/>
    </state>
    <final id="end"/>
</scxml>)";

    auto model = parser_->parseContent(scxmlContent);
    ASSERT_NE(model, nullptr);
    EXPECT_FALSE(parser_->hasErrors());

    // Verify start state exists
    auto startState = model->findStateById("start");
    ASSERT_NE(startState, nullptr) << "Start state not found";

    // Verify two transitions with guards
    auto transitions = startState->getTransitions();
    ASSERT_EQ(transitions.size(), 2) << "Start state should have exactly 2 transitions";

    // Verify first transition guard (counter < 10)
    auto transition1 = transitions[0];
    ASSERT_NE(transition1, nullptr);
    EXPECT_EQ(transition1->getEvent(), "increment");
    EXPECT_EQ(transition1->getGuard(), "counter < 10") << "First transition guard incorrect";

    // Verify second transition guard (counter >= 10)
    auto transition2 = transitions[1];
    ASSERT_NE(transition2, nullptr);
    EXPECT_EQ(transition2->getEvent(), "increment");
    EXPECT_EQ(transition2->getGuard(), "counter >= 10") << "Second transition guard incorrect";
}

// Test data model parsing
TEST_F(SCXMLParserBasicTest, ParseDataModel) {
    std::string scxmlContent = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="start">
    <datamodel>
        <data id="name" expr="'test'"/>
        <data id="count" expr="42"/>
        <data id="flag" expr="true"/>
    </datamodel>
    <state id="start">
        <transition event="done" target="end"/>
    </state>
    <final id="end"/>
</scxml>)";

    auto model = parser_->parseContent(scxmlContent);
    ASSERT_NE(model, nullptr);
    EXPECT_FALSE(parser_->hasErrors());

    // Verify data model items exist
    auto dataModelItems = model->getDataModelItems();
    ASSERT_EQ(dataModelItems.size(), 3) << "Should have exactly 3 data model items";

    // Find and verify each data item
    bool foundName = false, foundCount = false, foundFlag = false;
    for (const auto &item : dataModelItems) {
        if (item->getId() == "name") {
            foundName = true;
            EXPECT_EQ(item->getExpr(), "'test'") << "Data 'name' expr incorrect";
        } else if (item->getId() == "count") {
            foundCount = true;
            EXPECT_EQ(item->getExpr(), "42") << "Data 'count' expr incorrect";
        } else if (item->getId() == "flag") {
            foundFlag = true;
            EXPECT_EQ(item->getExpr(), "true") << "Data 'flag' expr incorrect";
        }
    }

    EXPECT_TRUE(foundName) << "Data item 'name' not found";
    EXPECT_TRUE(foundCount) << "Data item 'count' not found";
    EXPECT_TRUE(foundFlag) << "Data item 'flag' not found";
}

// Test final states
TEST_F(SCXMLParserBasicTest, ParseFinalStates) {
    std::string scxmlContent = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="start">
    <state id="start">
        <transition event="success" target="success_end"/>
        <transition event="failure" target="failure_end"/>
    </state>
    <final id="success_end">
        <donedata>
            <content expr="'completed successfully'"/>
        </donedata>
    </final>
    <final id="failure_end">
        <donedata>
            <content expr="'failed'"/>
        </donedata>
    </final>
</scxml>)";

    auto model = parser_->parseContent(scxmlContent);
    ASSERT_NE(model, nullptr);
    EXPECT_FALSE(parser_->hasErrors());

    // Verify success_end final state
    auto successEnd = model->findStateById("success_end");
    ASSERT_NE(successEnd, nullptr) << "Final state 'success_end' not found";
    EXPECT_TRUE(successEnd->isFinalState()) << "State 'success_end' should be final state";

    // Verify success_end donedata
    const auto &successDoneData = successEnd->getDoneData();
    EXPECT_FALSE(successDoneData.isEmpty()) << "success_end should have donedata";
    EXPECT_TRUE(successDoneData.hasContent()) << "success_end donedata should have content";
    EXPECT_EQ(successDoneData.getContent(), "'completed successfully'") << "success_end donedata content incorrect";

    // Verify failure_end final state
    auto failureEnd = model->findStateById("failure_end");
    ASSERT_NE(failureEnd, nullptr) << "Final state 'failure_end' not found";
    EXPECT_TRUE(failureEnd->isFinalState()) << "State 'failure_end' should be final state";

    // Verify failure_end donedata
    const auto &failureDoneData = failureEnd->getDoneData();
    EXPECT_FALSE(failureDoneData.isEmpty()) << "failure_end should have donedata";
    EXPECT_TRUE(failureDoneData.hasContent()) << "failure_end donedata should have content";
    EXPECT_EQ(failureDoneData.getContent(), "'failed'") << "failure_end donedata content incorrect";
}

// Test onentry/onexit actions
TEST_F(SCXMLParserBasicTest, ParseOnentryOnexitActions) {
    std::string scxmlContent = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="active">
    <state id="active">
        <onentry>
            <script>console.log('entering active');</script>
            <assign location="entered" expr="true"/>
        </onentry>
        <onexit>
            <script>console.log('exiting active');</script>
            <assign location="exited" expr="true"/>
        </onexit>
        <transition event="done" target="end"/>
    </state>
    <final id="end"/>
</scxml>)";

    auto model = parser_->parseContent(scxmlContent);
    ASSERT_NE(model, nullptr);
    EXPECT_FALSE(parser_->hasErrors());

    // Verify active state exists
    auto activeState = model->findStateById("active");
    ASSERT_NE(activeState, nullptr) << "Active state not found";

    // Verify onentry actions
    auto entryBlocks = activeState->getEntryActionBlocks();
    ASSERT_EQ(entryBlocks.size(), 1) << "Should have 1 onentry block";
    ASSERT_EQ(entryBlocks[0].size(), 2) << "Onentry block should have 2 actions";
    EXPECT_EQ(entryBlocks[0][0]->getActionType(), "script") << "First onentry action should be script";
    EXPECT_EQ(entryBlocks[0][1]->getActionType(), "assign") << "Second onentry action should be assign";

    // Verify onexit actions
    auto exitBlocks = activeState->getExitActionBlocks();
    ASSERT_EQ(exitBlocks.size(), 1) << "Should have 1 onexit block";
    ASSERT_EQ(exitBlocks[0].size(), 2) << "Onexit block should have 2 actions";
    EXPECT_EQ(exitBlocks[0][0]->getActionType(), "script") << "First onexit action should be script";
    EXPECT_EQ(exitBlocks[0][1]->getActionType(), "assign") << "Second onexit action should be assign";
}

// Test eventless transitions
TEST_F(SCXMLParserBasicTest, ParseEventlessTransitions) {
    std::string scxmlContent = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="start">
    <state id="start">
        <transition target="automatic" cond="true"/>
    </state>
    <state id="automatic">
        <transition target="end"/>
    </state>
    <final id="end"/>
</scxml>)";

    auto model = parser_->parseContent(scxmlContent);
    ASSERT_NE(model, nullptr);
    EXPECT_FALSE(parser_->hasErrors());

    // Verify start state eventless transition
    auto startState = model->findStateById("start");
    ASSERT_NE(startState, nullptr) << "Start state not found";

    auto startTransitions = startState->getTransitions();
    ASSERT_EQ(startTransitions.size(), 1) << "Start state should have 1 transition";

    auto eventlessTransition1 = startTransitions[0];
    EXPECT_TRUE(eventlessTransition1->getEvent().empty()) << "Transition should be eventless (no event attribute)";
    EXPECT_EQ(eventlessTransition1->getGuard(), "true") << "First eventless transition should have cond='true'";

    // Verify automatic state eventless transition
    auto automaticState = model->findStateById("automatic");
    ASSERT_NE(automaticState, nullptr) << "Automatic state not found";

    auto automaticTransitions = automaticState->getTransitions();
    ASSERT_EQ(automaticTransitions.size(), 1) << "Automatic state should have 1 transition";

    auto eventlessTransition2 = automaticTransitions[0];
    EXPECT_TRUE(eventlessTransition2->getEvent().empty()) << "Transition should be eventless (no event attribute)";
    EXPECT_TRUE(eventlessTransition2->getGuard().empty()) << "Second eventless transition should have no condition";
}

// Test explicit initial transition
TEST_F(SCXMLParserBasicTest, ParseExplicitInitialTransition) {
    std::string scxmlContent = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="compound">
    <state id="compound">
        <initial>
            <transition target="s1">
                <script>console.log('initializing');</script>
            </transition>
        </initial>
        <state id="s1">
            <transition event="next" target="s2"/>
        </state>
        <state id="s2"/>
    </state>
</scxml>)";

    auto model = parser_->parseContent(scxmlContent);
    ASSERT_NE(model, nullptr);
    EXPECT_FALSE(parser_->hasErrors());

    // Verify compound state exists
    auto compoundState = model->findStateById("compound");
    ASSERT_NE(compoundState, nullptr) << "Compound state not found";

    // Verify explicit initial transition
    auto initialTransition = compoundState->getInitialTransition();
    ASSERT_NE(initialTransition, nullptr) << "Compound state should have explicit initial transition";

    // Verify initial transition target
    auto targets = initialTransition->getTargets();
    ASSERT_EQ(targets.size(), 1) << "Initial transition should have 1 target";
    EXPECT_EQ(targets[0], "s1") << "Initial transition target should be 's1'";

    // Verify initial transition has action (script)
    auto actions = initialTransition->getActionNodes();
    ASSERT_EQ(actions.size(), 1) << "Initial transition should have 1 action";
    EXPECT_EQ(actions[0]->getActionType(), "script") << "Initial transition action should be script";
}

// W3C SCXML 3.12.1: Test nested actions document order (3-level nesting)
TEST_F(SCXMLParserBasicTest, ParseNestedActionsDocumentOrder) {
    std::string scxmlContent = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="start">
    <datamodel>
        <data id="items" expr="[1,2,3]"/>
    </datamodel>
    <state id="start">
        <onentry>
            <foreach array="items" item="i">
                <if cond="i > 1">
                    <script>console.log('processing');</script>
                    <assign location="result" expr="result + i"/>
                    <log expr="'item: ' + i"/>
                </if>
            </foreach>
        </onentry>
        <transition event="done" target="end"/>
    </state>
    <final id="end"/>
</scxml>)";

    auto model = parser_->parseContent(scxmlContent);
    ASSERT_NE(model, nullptr);
    EXPECT_FALSE(parser_->hasErrors());

    // Verify start state exists
    auto startState = model->findStateById("start");
    ASSERT_NE(startState, nullptr) << "Start state not found";

    // Verify onentry block exists
    auto entryBlocks = startState->getEntryActionBlocks();
    ASSERT_EQ(entryBlocks.size(), 1) << "Should have 1 onentry block";
    ASSERT_GE(entryBlocks[0].size(), 1) << "Onentry block should have at least 1 action (foreach)";

    // Verify foreach is the first action
    EXPECT_EQ(entryBlocks[0][0]->getActionType(), "foreach") << "First onentry action should be foreach";

    // Verify foreach contains if with nested actions (dynamic_cast to access internals)
    auto foreachAction = entryBlocks[0][0];
    ASSERT_NE(foreachAction, nullptr);

    // Cast to ForeachAction to access iteration actions
    auto foreachPtr = std::dynamic_pointer_cast<RSM::ForeachAction>(foreachAction);
    ASSERT_NE(foreachPtr, nullptr) << "Failed to cast to ForeachAction";

    // Verify foreach has 1 iteration action (the if)
    const auto &iterationActions = foreachPtr->getIterationActions();
    ASSERT_EQ(iterationActions.size(), 1) << "Foreach should have 1 iteration action (if)";
    EXPECT_EQ(iterationActions[0]->getActionType(), "if") << "Foreach iteration action should be if";

    // Cast to IfAction to verify nested actions
    auto ifPtr = std::dynamic_pointer_cast<RSM::IfAction>(iterationActions[0]);
    ASSERT_NE(ifPtr, nullptr) << "Failed to cast to IfAction";

    // Verify if branch has 3 actions in document order: script → assign → log
    const auto &branches = ifPtr->getBranches();
    ASSERT_GE(branches.size(), 1) << "If should have at least 1 branch";
    const auto &ifBranchActions = branches[0].actions;
    ASSERT_EQ(ifBranchActions.size(), 3) << "If branch should have 3 actions";
    EXPECT_EQ(ifBranchActions[0]->getActionType(), "script") << "1st action in if should be script";
    EXPECT_EQ(ifBranchActions[1]->getActionType(), "assign") << "2nd action in if should be assign";
    EXPECT_EQ(ifBranchActions[2]->getActionType(), "log") << "3rd action in if should be log";
}

// W3C SCXML 4.3: Test if/elseif/else document order preservation
TEST_F(SCXMLParserBasicTest, ParseIfElseifElseDocumentOrder) {
    std::string scxmlContent = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="start">
    <state id="start">
        <onentry>
            <if cond="false">
                <raise event="foo"/>
                <assign location="v1" expr="1"/>
            <elseif cond="true"/>
                <raise event="bar"/>
                <assign location="v2" expr="2"/>
            <else/>
                <raise event="baz"/>
                <assign location="v3" expr="3"/>
            </if>
        </onentry>
        <transition event="bar" target="pass"/>
        <transition event="*" target="fail"/>
    </state>
    <final id="pass"/>
    <final id="fail"/>
</scxml>)";

    auto model = parser_->parseContent(scxmlContent);
    ASSERT_NE(model, nullptr);
    EXPECT_FALSE(parser_->hasErrors());

    // Verify start state exists
    auto startState = model->findStateById("start");
    ASSERT_NE(startState, nullptr) << "Start state not found";

    // Verify onentry block with if action
    auto entryBlocks = startState->getEntryActionBlocks();
    ASSERT_EQ(entryBlocks.size(), 1) << "Should have 1 onentry block";
    ASSERT_GE(entryBlocks[0].size(), 1) << "Onentry should have at least 1 action (if)";

    // Verify if is the first action
    EXPECT_EQ(entryBlocks[0][0]->getActionType(), "if") << "First onentry action should be if";

    // Cast to IfAction to verify branch separation
    auto ifPtr = std::dynamic_pointer_cast<RSM::IfAction>(entryBlocks[0][0]);
    ASSERT_NE(ifPtr, nullptr) << "Failed to cast to IfAction";

    // Verify 3 branches: if, elseif, else
    const auto &branches = ifPtr->getBranches();
    ASSERT_EQ(branches.size(), 3) << "Should have 3 branches (if, elseif, else)";

    // Verify if branch (cond="false") has 2 actions: raise foo, assign v1
    EXPECT_EQ(branches[0].condition, "false") << "1st branch should have cond='false'";
    EXPECT_FALSE(branches[0].isElseBranch) << "1st branch should not be else";
    ASSERT_EQ(branches[0].actions.size(), 2) << "If branch should have 2 actions";
    EXPECT_EQ(branches[0].actions[0]->getActionType(), "raise") << "If: 1st action should be raise";
    EXPECT_EQ(branches[0].actions[1]->getActionType(), "assign") << "If: 2nd action should be assign";

    // Verify elseif branch (cond="true") has 2 actions: raise bar, assign v2
    EXPECT_EQ(branches[1].condition, "true") << "2nd branch should have cond='true'";
    EXPECT_FALSE(branches[1].isElseBranch) << "2nd branch should not be else";
    ASSERT_EQ(branches[1].actions.size(), 2) << "Elseif branch should have 2 actions";
    EXPECT_EQ(branches[1].actions[0]->getActionType(), "raise") << "Elseif: 1st action should be raise";
    EXPECT_EQ(branches[1].actions[1]->getActionType(), "assign") << "Elseif: 2nd action should be assign";

    // Verify else branch has 2 actions: raise baz, assign v3
    EXPECT_TRUE(branches[2].isElseBranch) << "3rd branch should be else";
    ASSERT_EQ(branches[2].actions.size(), 2) << "Else branch should have 2 actions";
    EXPECT_EQ(branches[2].actions[0]->getActionType(), "raise") << "Else: 1st action should be raise";
    EXPECT_EQ(branches[2].actions[1]->getActionType(), "assign") << "Else: 2nd action should be assign";

    // This validates that parser correctly separates branches using getChildren() for direct children
    // instead of findChildElements() which would recursively collect from all branches
}

// W3C SCXML 6.4 + B.2: Test invoke content XML serialization
TEST_F(SCXMLParserBasicTest, ParseInvokeContentSerialization) {
    std::string scxmlContent = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="start">
    <state id="start">
        <invoke type="http://www.w3.org/2005/07/scxml" id="child">
            <content>
                <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="sub">
                    <state id="sub">
                        <onentry>
                            <send target="#_parent" event="childReady"/>
                        </onentry>
                        <transition event="done" target="subEnd"/>
                    </state>
                    <final id="subEnd"/>
                </scxml>
            </content>
        </invoke>
        <transition event="childReady" target="end"/>
    </state>
    <final id="end"/>
</scxml>)";

    auto model = parser_->parseContent(scxmlContent);
    ASSERT_NE(model, nullptr);
    EXPECT_FALSE(parser_->hasErrors());

    // Verify start state exists
    auto startState = model->findStateById("start");
    ASSERT_NE(startState, nullptr) << "Start state not found";

    // Verify invoke exists
    auto invokes = startState->getInvoke();
    ASSERT_EQ(invokes.size(), 1) << "Start state should have 1 invoke";

    auto invoke = invokes[0];
    ASSERT_NE(invoke, nullptr);
    EXPECT_EQ(invoke->getId(), "child") << "Invoke ID should be 'child'";

    // Verify invoke has content
    const std::string &content = invoke->getContent();
    EXPECT_FALSE(content.empty()) << "Invoke should have content";

    // Verify content contains nested SCXML structure (XML serialization preserved)
    EXPECT_NE(content.find("<scxml"), std::string::npos) << "Content should contain <scxml> tag";
    EXPECT_NE(content.find("<state"), std::string::npos) << "Content should contain <state> tag";
    EXPECT_NE(content.find("<onentry>"), std::string::npos) << "Content should contain <onentry> tag";
    EXPECT_NE(content.find("<send"), std::string::npos) << "Content should contain <send> tag";
    EXPECT_NE(content.find("target=\"#_parent\""), std::string::npos) << "Content should contain parent target";
    EXPECT_NE(content.find("event=\"childReady\""), std::string::npos) << "Content should contain childReady event";

    // This verifies that IXMLElement::serializeChildContent() correctly
    // preserves nested XML structure for invoke content element
}

// W3C SCXML 3.12.1: Test mixed action types document order
TEST_F(SCXMLParserBasicTest, ParseMixedActionTypesOrder) {
    std::string scxmlContent = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="start">
    <state id="start">
        <transition event="go" target="end">
            <script>console.log('1st');</script>
            <assign location="v1" expr="1"/>
            <if cond="true">
                <raise event="e1"/>
            </if>
            <foreach array="[1]" item="i">
                <assign location="v2" expr="2"/>
            </foreach>
            <send event="e2"/>
            <log expr="'6th'"/>
            <raise event="e3"/>
        </transition>
    </state>
    <final id="end"/>
</scxml>)";

    auto model = parser_->parseContent(scxmlContent);
    ASSERT_NE(model, nullptr);
    EXPECT_FALSE(parser_->hasErrors());

    // Verify start state exists
    auto startState = model->findStateById("start");
    ASSERT_NE(startState, nullptr) << "Start state not found";

    // Verify transition exists
    auto transitions = startState->getTransitions();
    ASSERT_EQ(transitions.size(), 1) << "Start state should have 1 transition";

    auto transition = transitions[0];
    ASSERT_NE(transition, nullptr);

    // Verify all 7 actions were parsed in document order
    auto actions = transition->getActionNodes();
    ASSERT_EQ(actions.size(), 7) << "Transition should have exactly 7 actions";

    // Verify action types in document order
    EXPECT_EQ(actions[0]->getActionType(), "script") << "1st action should be script";
    EXPECT_EQ(actions[1]->getActionType(), "assign") << "2nd action should be assign";
    EXPECT_EQ(actions[2]->getActionType(), "if") << "3rd action should be if";
    EXPECT_EQ(actions[3]->getActionType(), "foreach") << "4th action should be foreach";
    EXPECT_EQ(actions[4]->getActionType(), "send") << "5th action should be send";
    EXPECT_EQ(actions[5]->getActionType(), "log") << "6th action should be log";
    EXPECT_EQ(actions[6]->getActionType(), "raise") << "7th action should be raise";
}

// W3C SCXML 3.12.1: Test action block with multiple actions parsing
TEST_F(SCXMLParserBasicTest, ParseActionBlockWithMultipleActions) {
    std::string scxmlContent = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="start">
    <state id="start">
        <onentry>
            <assign location="v1" expr="1"/>
            <assign location="v2" expr="2"/>
            <assign location="v3" expr="3"/>
            <script>console.log('all assigned');</script>
        </onentry>
        <transition event="done" target="end"/>
    </state>
    <final id="end"/>
</scxml>)";

    auto model = parser_->parseContent(scxmlContent);
    ASSERT_NE(model, nullptr);
    EXPECT_FALSE(parser_->hasErrors());

    // Verify start state exists
    auto startState = model->findStateById("start");
    ASSERT_NE(startState, nullptr) << "Start state not found";

    // Verify onentry block exists
    auto entryBlocks = startState->getEntryActionBlocks();
    ASSERT_EQ(entryBlocks.size(), 1) << "Should have 1 onentry block";

    // Verify all 4 actions were parsed in document order
    ASSERT_EQ(entryBlocks[0].size(), 4) << "Onentry block should have exactly 4 actions";
    EXPECT_EQ(entryBlocks[0][0]->getActionType(), "assign") << "1st action should be assign";
    EXPECT_EQ(entryBlocks[0][1]->getActionType(), "assign") << "2nd action should be assign";
    EXPECT_EQ(entryBlocks[0][2]->getActionType(), "assign") << "3rd action should be assign";
    EXPECT_EQ(entryBlocks[0][3]->getActionType(), "script") << "4th action should be script";

    // Note: Parser parses all actions in document order
    // Error handling (halting subsequent actions on error) is runtime responsibility
}

}  // namespace Tests
}  // namespace RSM