// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// An element of another vocabulary is not read as SCXML's executable
// content — the interpreter's half of what
// sce-build/tests/a_foreign_element_is_not_read_as_scxml.rs holds for the
// generated machines.
//
// A document may carry executable content from other namespaces: the
// Recommendation says its schema allows elements from arbitrary namespaces
// inside blocks of executable content, and SCE skips what it does not read.
// ActionParser matched a local name alone, so `<x:raise event="bogus"/>`
// raised `bogus`, `<x:else/>` split an `<if>`, and `<x:if>` was an `<if>`
// (measured 2026-09-24, by reading the parser: `matchNodeName` accepts any
// prefix, and the partition stripped it before comparing).

#include "actions/IfAction.h"
#include "actions/RaiseAction.h"
#include "factory/NodeFactory.h"
#include "model/SCXMLModel.h"
#include "parsing/SCXMLParser.h"

#include <gtest/gtest.h>

#include <memory>
#include <string>
#include <vector>

namespace {

constexpr const char *kScxml = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:x="urn:example:x"
       version="1.0" datamodel="ecmascript" initial="a">
  <state id="a">
    <onentry>
      <x:raise event="bogus"/>
      <raise event="real"/>
      <if cond="true">
        <x:else/>
        <raise event="then"/>
      </if>
      <x:if cond="true">
        <raise event="nested"/>
      </x:if>
    </onentry>
    <transition event="go" target="b"/>
  </state>
  <final id="b"/>
</scxml>
)";

// `state`'s entry actions, in document order.
std::vector<std::shared_ptr<SCE::IActionNode>> entryActions(SCE::SCXMLModel &model, const std::string &state) {
    std::vector<std::shared_ptr<SCE::IActionNode>> actions;
    auto *node = model.findStateById(state);
    EXPECT_NE(node, nullptr) << "state " << state;
    if (!node) {
        return actions;
    }
    for (const auto &block : node->getEntryActionBlocks()) {
        actions.insert(actions.end(), block.begin(), block.end());
    }
    return actions;
}

std::string raisedEvent(const std::shared_ptr<SCE::IActionNode> &action) {
    auto raise = std::dynamic_pointer_cast<SCE::RaiseAction>(action);
    return raise ? raise->getEvent() : std::string("<not a raise>");
}

}  // namespace

TEST(ForeignExecutableContentTest, OnlySCXMLsOwnElementsAreReadAsActions) {
    SCE::SCXMLParser parser(std::make_shared<SCE::NodeFactory>());
    auto model = parser.parseContent(kScxml);
    ASSERT_NE(model, nullptr);

    auto actions = entryActions(*model, "a");
    ASSERT_EQ(actions.size(), 2u) << "the SCXML <raise> and the SCXML <if> — not the foreign <raise> or <if>";
    EXPECT_EQ(raisedEvent(actions[0]), "real");
    EXPECT_NE(std::dynamic_pointer_cast<SCE::IfAction>(actions[1]), nullptr);
}

TEST(ForeignExecutableContentTest, AForeignElseDoesNotSplitAnIf) {
    SCE::SCXMLParser parser(std::make_shared<SCE::NodeFactory>());
    auto model = parser.parseContent(kScxml);
    ASSERT_NE(model, nullptr);

    auto actions = entryActions(*model, "a");
    ASSERT_EQ(actions.size(), 2u);
    auto ifAction = std::dynamic_pointer_cast<SCE::IfAction>(actions[1]);
    ASSERT_NE(ifAction, nullptr);

    const auto &branches = ifAction->getBranches();
    ASSERT_EQ(branches.size(), 1u) << "the <if>'s own branch and no <else>";
    EXPECT_FALSE(branches[0].isElseBranch);
    ASSERT_EQ(branches[0].actions.size(), 1u);
    EXPECT_EQ(raisedEvent(branches[0].actions[0]), "then");
}
