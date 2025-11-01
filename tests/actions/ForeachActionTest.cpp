#include "actions/ForeachAction.h"
#include "actions/AssignAction.h"
#include "mocks/MockActionExecutor.h"
#include <gtest/gtest.h>
#include <memory>

using namespace RSM;
using namespace RSM::Test;

class ForeachActionTest : public ::testing::Test {
protected:
    void SetUp() override {
        mockExecutor = std::make_shared<MockActionExecutor>("foreach_test_session");
        context = std::make_shared<MockExecutionContext>(mockExecutor);
    }

    std::shared_ptr<MockActionExecutor> mockExecutor;
    std::shared_ptr<MockExecutionContext> context;
};

TEST_F(ForeachActionTest, ConstructorAndBasicProperties) {
    ForeachAction action("myArray", "item", "index", "test_foreach");

    EXPECT_EQ(action.getId(), "test_foreach");
    EXPECT_EQ(action.getActionType(), "foreach");
    EXPECT_EQ(action.getArray(), "myArray");
    EXPECT_EQ(action.getItem(), "item");
    EXPECT_EQ(action.getIndex(), "index");
    EXPECT_EQ(action.getIterationActionCount(), 0);
}

TEST_F(ForeachActionTest, ConstructorDefaults) {
    ForeachAction action;

    EXPECT_TRUE(action.getId().empty());
    EXPECT_EQ(action.getActionType(), "foreach");
    EXPECT_TRUE(action.getArray().empty());
    EXPECT_TRUE(action.getItem().empty());
    EXPECT_TRUE(action.getIndex().empty());
    EXPECT_EQ(action.getIterationActionCount(), 0);
}

TEST_F(ForeachActionTest, ValidationTests_RequiredAttributes) {
    // Valid foreach
    ForeachAction validAction("users", "user");
    auto errors = validAction.validate();
    EXPECT_TRUE(errors.empty());

    // Empty array attribute
    ForeachAction emptyArrayAction("", "item");
    errors = emptyArrayAction.validate();
    EXPECT_FALSE(errors.empty());
    EXPECT_TRUE(errors[0].find("array") != std::string::npos);

    // Empty item attribute
    ForeachAction emptyItemAction("array", "");
    errors = emptyItemAction.validate();
    EXPECT_FALSE(errors.empty());
    EXPECT_TRUE(errors[0].find("item") != std::string::npos);

    // Both empty
    ForeachAction bothEmptyAction("", "");
    errors = bothEmptyAction.validate();
    EXPECT_GE(errors.size(), 2);
}

TEST_F(ForeachActionTest, ValidationTests_VariableNaming_Item) {
    // Valid item names
    ForeachAction validAlpha("arr", "item");
    EXPECT_TRUE(validAlpha.validate().empty());

    ForeachAction validUnderscore("arr", "_item");
    EXPECT_TRUE(validUnderscore.validate().empty());

    ForeachAction validMixed("arr", "item123");
    EXPECT_TRUE(validMixed.validate().empty());

    // Invalid: starts with number
    ForeachAction invalidNumber("arr", "123item");
    auto errors = invalidNumber.validate();
    EXPECT_FALSE(errors.empty());
    EXPECT_TRUE(errors[0].find("must start with") != std::string::npos ||
                errors[0].find("letter or underscore") != std::string::npos);

    // Invalid: contains hyphen
    ForeachAction invalidHyphen("arr", "item-name");
    errors = invalidHyphen.validate();
    EXPECT_FALSE(errors.empty());
    EXPECT_TRUE(errors[0].find("invalid characters") != std::string::npos);

    // Invalid: contains space
    ForeachAction invalidSpace("arr", "item name");
    errors = invalidSpace.validate();
    EXPECT_FALSE(errors.empty());
}

TEST_F(ForeachActionTest, ValidationTests_VariableNaming_Index) {
    // Valid index names
    ForeachAction validAlpha("arr", "item", "i");
    EXPECT_TRUE(validAlpha.validate().empty());

    ForeachAction validUnderscore("arr", "item", "_index");
    EXPECT_TRUE(validUnderscore.validate().empty());

    // Invalid: starts with number
    ForeachAction invalidNumber("arr", "item", "0index");
    auto errors = invalidNumber.validate();
    EXPECT_FALSE(errors.empty());
    EXPECT_TRUE(errors[0].find("must start with") != std::string::npos ||
                errors[0].find("letter or underscore") != std::string::npos);

    // Invalid: contains special character
    ForeachAction invalidSpecial("arr", "item", "index!");
    errors = invalidSpecial.validate();
    EXPECT_FALSE(errors.empty());
}

TEST_F(ForeachActionTest, ValidationTests_ItemIndexConflict) {
    // Item and index cannot be the same
    ForeachAction conflictAction("array", "var", "var");
    auto errors = conflictAction.validate();
    EXPECT_FALSE(errors.empty());
    EXPECT_TRUE(errors[0].find("Item") != std::string::npos);
    EXPECT_TRUE(errors[0].find("index") != std::string::npos);
    EXPECT_TRUE(errors[0].find("must be different") != std::string::npos);
}

TEST_F(ForeachActionTest, CloneOperation) {
    ForeachAction original("originalArray", "item", "index", "original_id");

    // Add iteration actions
    auto assignAction = std::make_shared<AssignAction>("item.processed", "true", "assign1");
    auto scriptAction = std::make_shared<AssignAction>("counter", "counter + 1", "assign2");
    original.addIterationAction(assignAction);
    original.addIterationAction(scriptAction);

    auto cloned = original.clone();
    ASSERT_NE(cloned, nullptr);

    auto foreachCloned = std::dynamic_pointer_cast<ForeachAction>(cloned);
    ASSERT_NE(foreachCloned, nullptr);

    EXPECT_EQ(foreachCloned->getId(), "original_id");
    EXPECT_EQ(foreachCloned->getActionType(), "foreach");
    EXPECT_EQ(foreachCloned->getArray(), "originalArray");
    EXPECT_EQ(foreachCloned->getItem(), "item");
    EXPECT_EQ(foreachCloned->getIndex(), "index");
    EXPECT_EQ(foreachCloned->getIterationActionCount(), 2);

    // Verify independence (modify clone)
    foreachCloned->setArray("modifiedArray");
    foreachCloned->setItem("newItem");
    foreachCloned->setIndex("newIndex");

    EXPECT_EQ(original.getArray(), "originalArray");
    EXPECT_EQ(original.getItem(), "item");
    EXPECT_EQ(original.getIndex(), "index");

    // Verify deep copy of iteration actions
    foreachCloned->clearIterationActions();
    EXPECT_EQ(foreachCloned->getIterationActionCount(), 0);
    EXPECT_EQ(original.getIterationActionCount(), 2);
}

TEST_F(ForeachActionTest, CloneOperation_EmptyIterationActions) {
    ForeachAction original("array", "item");

    auto cloned = original.clone();
    auto foreachCloned = std::dynamic_pointer_cast<ForeachAction>(cloned);
    ASSERT_NE(foreachCloned, nullptr);

    EXPECT_EQ(foreachCloned->getIterationActionCount(), 0);
}

TEST_F(ForeachActionTest, PropertyModification) {
    ForeachAction action("initialArray", "initialItem", "initialIndex");

    // Test array modification
    EXPECT_EQ(action.getArray(), "initialArray");
    action.setArray("modifiedArray");
    EXPECT_EQ(action.getArray(), "modifiedArray");

    // Test item modification
    EXPECT_EQ(action.getItem(), "initialItem");
    action.setItem("modifiedItem");
    EXPECT_EQ(action.getItem(), "modifiedItem");

    // Test index modification
    EXPECT_EQ(action.getIndex(), "initialIndex");
    action.setIndex("modifiedIndex");
    EXPECT_EQ(action.getIndex(), "modifiedIndex");

    // Clear index (empty string)
    action.setIndex("");
    EXPECT_TRUE(action.getIndex().empty());
}

TEST_F(ForeachActionTest, IterationActionManagement) {
    ForeachAction action("array", "item");

    EXPECT_EQ(action.getIterationActionCount(), 0);
    EXPECT_TRUE(action.getIterationActions().empty());

    // Add first action
    auto action1 = std::make_shared<AssignAction>("var1", "value1");
    action.addIterationAction(action1);
    EXPECT_EQ(action.getIterationActionCount(), 1);
    EXPECT_EQ(action.getIterationActions().size(), 1);

    // Add second action
    auto action2 = std::make_shared<AssignAction>("var2", "value2");
    action.addIterationAction(action2);
    EXPECT_EQ(action.getIterationActionCount(), 2);
    EXPECT_EQ(action.getIterationActions().size(), 2);

    // Add third action
    auto action3 = std::make_shared<AssignAction>("var3", "value3");
    action.addIterationAction(action3);
    EXPECT_EQ(action.getIterationActionCount(), 3);

    // Clear all actions
    action.clearIterationActions();
    EXPECT_EQ(action.getIterationActionCount(), 0);
    EXPECT_TRUE(action.getIterationActions().empty());
}

TEST_F(ForeachActionTest, DescriptionGeneration) {
    // Basic foreach
    ForeachAction action("users", "user", "i", "user_loop");
    std::string desc = action.getDescription();
    EXPECT_TRUE(desc.find("foreach") != std::string::npos);
    EXPECT_TRUE(desc.find("users") != std::string::npos);
    EXPECT_TRUE(desc.find("user") != std::string::npos);

    // Foreach without index
    ForeachAction noIndexAction("data", "item");
    desc = noIndexAction.getDescription();
    EXPECT_TRUE(desc.find("foreach") != std::string::npos);
    EXPECT_TRUE(desc.find("data") != std::string::npos);

    // Foreach with iteration actions
    ForeachAction withActions("items", "item");
    withActions.addIterationAction(std::make_shared<AssignAction>("x", "1"));
    withActions.addIterationAction(std::make_shared<AssignAction>("y", "2"));
    desc = withActions.getDescription();
    EXPECT_TRUE(desc.find("2") != std::string::npos || desc.find("action") != std::string::npos);
}

TEST_F(ForeachActionTest, ValidationTests_ChildActions) {
    ForeachAction action("array", "item");

    // Add valid child action
    auto validChild = std::make_shared<AssignAction>("validVar", "42");
    action.addIterationAction(validChild);
    auto errors = action.validate();
    EXPECT_TRUE(errors.empty());

    // Add invalid child action
    auto invalidChild = std::make_shared<AssignAction>("", "");  // Empty location and expr
    action.addIterationAction(invalidChild);
    errors = action.validate();
    EXPECT_FALSE(errors.empty());
}

TEST_F(ForeachActionTest, ArrayExpressionVariety) {
    // Variable reference
    ForeachAction varAction("myArray", "item");
    EXPECT_TRUE(varAction.validate().empty());

    // Dot notation
    ForeachAction dotAction("data.items", "item");
    EXPECT_TRUE(dotAction.validate().empty());

    // Array literal
    ForeachAction literalAction("[1, 2, 3, 4, 5]", "num");
    EXPECT_TRUE(literalAction.validate().empty());

    // Expression
    ForeachAction exprAction("users.filter(u => u.active)", "user");
    EXPECT_TRUE(exprAction.validate().empty());
}
