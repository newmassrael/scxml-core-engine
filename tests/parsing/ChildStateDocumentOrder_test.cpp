// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Regression test for child-state collection order.
//
// Both §scxml-3.2 and §scxml-3.3 make "the first child state in document
// order" the default initial state, and Appendix D enters <parallel>
// regions in `getChildStates` order. The collection order of a parent's
// child states is therefore observable behaviour.
//
// The parser used to build that list by calling `findChildElements` once
// per element name and concatenating the results, which groups siblings
// by name: every <state> first, then every <parallel>, then every
// <final>, then every <history>. For a mixed sibling set that is not
// document order, so a document whose first child state is a <parallel>
// or a <final> got the wrong default initial state — latent because the
// W3C IRP fixtures almost always write their <state> children first.
//
// `ParsingCommon::findChildElementsAnyOf` walks the child list once and
// keeps document order. These tests pin the ordering at both levels the
// spec reads it from (document root and compound state) plus the
// namespace filter the single-name lookup used to provide.

#include "factory/NodeFactory.h"
#include "model/SCXMLModel.h"
#include "parsing/ParsingCommon.h"
#include "parsing/PugiXMLParser.h"
#include "parsing/SCXMLParser.h"

#include <gtest/gtest.h>

#include <memory>
#include <string>
#include <vector>

namespace {

std::vector<std::string> childIds(const std::shared_ptr<SCE::IStateNode> &state) {
    std::vector<std::string> ids;
    for (const auto &child : state->getChildren()) {
        ids.push_back(child->getId());
    }
    return ids;
}

// ── Document root: §scxml-3.2 default initial state ────────────────

// StateMachine::start resolves an absent 'initial' attribute to
// `getAllStates()[0]`, which is seeded from the root state list, so the
// first entry must be the first child state in document order.
TEST(ChildStateDocumentOrderTest, RootParallelBeforeStateStaysFirst) {
    static constexpr const char *kScxml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
                                          "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\""
                                          "       version=\"1.0\" datamodel=\"null\">"
                                          "  <parallel id=\"p\">"
                                          "    <state id=\"pa\"/>"
                                          "    <state id=\"pb\"/>"
                                          "  </parallel>"
                                          "  <state id=\"s\"/>"
                                          "</scxml>";

    SCE::SCXMLParser parser(std::make_shared<SCE::NodeFactory>());
    auto model = parser.parseContent(kScxml);
    ASSERT_NE(model, nullptr);

    ASSERT_NE(model->getRootState(), nullptr);
    EXPECT_EQ(model->getRootState()->getId(), "p");

    const auto &allStates = model->getAllStates();
    ASSERT_FALSE(allStates.empty());
    EXPECT_EQ(allStates[0]->getId(), "p");
}

TEST(ChildStateDocumentOrderTest, RootFinalBeforeStateStaysFirst) {
    static constexpr const char *kScxml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
                                          "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\""
                                          "       version=\"1.0\" datamodel=\"null\">"
                                          "  <final id=\"f\"/>"
                                          "  <state id=\"s\"/>"
                                          "</scxml>";

    SCE::SCXMLParser parser(std::make_shared<SCE::NodeFactory>());
    auto model = parser.parseContent(kScxml);
    ASSERT_NE(model, nullptr);

    ASSERT_NE(model->getRootState(), nullptr);
    EXPECT_EQ(model->getRootState()->getId(), "f");
}

// ── Compound state: §scxml-3.3 default initial state ───────────────

TEST(ChildStateDocumentOrderTest, CompoundStateFinalBeforeStateStaysFirst) {
    static constexpr const char *kScxml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
                                          "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\""
                                          "       version=\"1.0\" datamodel=\"null\" initial=\"c\">"
                                          "  <state id=\"c\">"
                                          "    <final id=\"cf\"/>"
                                          "    <state id=\"cs\"/>"
                                          "  </state>"
                                          "</scxml>";

    SCE::SCXMLParser parser(std::make_shared<SCE::NodeFactory>());
    auto model = parser.parseContent(kScxml);
    ASSERT_NE(model, nullptr);

    auto *compound = model->findStateById("c");
    ASSERT_NE(compound, nullptr);
    ASSERT_EQ(compound->getChildren().size(), 2u);
    EXPECT_EQ(compound->getChildren()[0]->getId(), "cf");
    EXPECT_EQ(compound->getChildren()[1]->getId(), "cs");
}

// A four-way mix exercises every name the collector accepts, so a
// re-introduced per-name grouping cannot pass by accident: grouping
// would yield {s2, p1, f1, h1} for this document.
TEST(ChildStateDocumentOrderTest, CompoundStateMixedChildrenKeepDocumentOrder) {
    static constexpr const char *kScxml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
                                          "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\""
                                          "       version=\"1.0\" datamodel=\"null\" initial=\"c\">"
                                          "  <state id=\"c\">"
                                          "    <history id=\"h1\" type=\"shallow\">"
                                          "      <transition target=\"s2\"/>"
                                          "    </history>"
                                          "    <final id=\"f1\"/>"
                                          "    <parallel id=\"p1\">"
                                          "      <state id=\"pa\"/>"
                                          "      <state id=\"pb\"/>"
                                          "    </parallel>"
                                          "    <state id=\"s2\"/>"
                                          "  </state>"
                                          "</scxml>";

    SCE::SCXMLParser parser(std::make_shared<SCE::NodeFactory>());
    auto model = parser.parseContent(kScxml);
    ASSERT_NE(model, nullptr);

    auto *compound = model->findStateById("c");
    ASSERT_NE(compound, nullptr);

    std::shared_ptr<SCE::IStateNode> compoundShared;
    for (const auto &state : model->getAllStates()) {
        if (state->getId() == "c") {
            compoundShared = state;
            break;
        }
    }
    ASSERT_NE(compoundShared, nullptr);

    const std::vector<std::string> expected{"h1", "f1", "p1", "s2"};
    EXPECT_EQ(childIds(compoundShared), expected);
}

// ── Namespace filter parity with the single-name lookup ────────────

// `findChildElements` filters on `isScxmlNamespace`; the any-of variant
// must apply the same filter, or a foreign-namespace element whose local
// name collides with a W3C one would enter the state graph.
TEST(ChildStateDocumentOrderTest, ForeignNamespaceChildrenRejected) {
    static constexpr const char *kXml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
                                        "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\""
                                        "       xmlns:f=\"http://example.com/framework\""
                                        "       version=\"1.0\">"
                                        "  <f:state id=\"foreign\"/>"
                                        "  <state id=\"native\"/>"
                                        "</scxml>";

    SCE::PugiXMLParser xmlParser;
    auto doc = xmlParser.parseContent(kXml);
    ASSERT_NE(doc, nullptr);
    auto root = doc->getRootElement();
    ASSERT_NE(root, nullptr);

    auto collected = SCE::ParsingCommon::findChildElementsAnyOf(root, {"state", "parallel", "final", "history"});
    ASSERT_EQ(collected.size(), 1u);
    EXPECT_EQ(collected[0]->getAttribute("id"), "native");
}

}  // namespace
