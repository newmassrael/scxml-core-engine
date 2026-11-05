// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.4.2: "A conformant SCXML document MUST specify either "expr"
// or children of <assign>, but not both." An `<assign>` with neither
// specifies no value, so the Interpreter's ActionParser refuses it where the
// document is read, as the code generator's frontend refuses it
// (`sce-build/tests/assign_attributes_are_checked_as_written.rs`), rather
// than running it to evaluate an empty expression.

#include "actions/AssignAction.h"
#include "factory/NodeFactory.h"
#include "parsing/ActionParser.h"
#include "parsing/PugiXMLParser.h"
#include "parsing/SemanticError.h"

#include <gtest/gtest.h>

#include <memory>
#include <string>

namespace {

std::shared_ptr<SCE::IActionNode> parseAssign(const std::string &xml) {
    SCE::PugiXMLParser xmlParser;
    auto doc = xmlParser.parseContent(xml);
    EXPECT_TRUE(doc) << "fixture must parse as XML";
    if (!doc) {
        return nullptr;
    }
    SCE::ActionParser actionParser(std::make_shared<SCE::NodeFactory>());
    return actionParser.parseActionNode(doc->getRootElement());
}

// Neither `expr` nor children, written as an empty element and as one whose
// only content is the whitespace a pretty-printed document puts there.
TEST(AssignNeedsAValueTest, NeitherExprNorChildrenIsRefusedAsTheMissingExpr) {
    for (const char *xml : {"<assign xmlns=\"http://www.w3.org/2005/07/scxml\" location=\"v\"/>",
                            "<assign xmlns=\"http://www.w3.org/2005/07/scxml\" location=\"v\">\n  </assign>"}) {
        try {
            parseAssign(xml);
            ADD_FAILURE() << "an <assign> with neither expr nor children was accepted: " << xml;
        } catch (const SCE::parsing::SemanticMissingAttribute &refused) {
            EXPECT_EQ(refused.element(), "<assign>") << xml;
            EXPECT_EQ(refused.attr(), "expr") << xml;
        }
    }
}

// The controls: each way of giving the value on its own is accepted.
TEST(AssignNeedsAValueTest, EitherExprOrChildrenIsAccepted) {
    auto byExpr = std::dynamic_pointer_cast<SCE::AssignAction>(
        parseAssign("<assign xmlns=\"http://www.w3.org/2005/07/scxml\" location=\"v\" expr=\"1\"/>"));
    ASSERT_NE(byExpr, nullptr);
    EXPECT_EQ(byExpr->getExpr(), "1");

    auto byChildren = std::dynamic_pointer_cast<SCE::AssignAction>(
        parseAssign("<assign xmlns=\"http://www.w3.org/2005/07/scxml\" location=\"v\">21</assign>"));
    ASSERT_NE(byChildren, nullptr);
    EXPECT_TRUE(byChildren->hasInlineContent());
}

}  // namespace
