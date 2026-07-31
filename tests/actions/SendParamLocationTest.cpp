// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Regression test for <send><param location="..."/>.
//
// §scxml-5.7.1 lets a <param> carry either 'expr' or 'location'. The
// Interpreter's ActionParser used to accept a send param only when it
// carried BOTH 'name' and 'expr', so a location-valued param was dropped
// with a warning and never reached the sent event — while the AOT path
// stored it (`sce-build/src/parser.rs` Param::location) and every backend
// template evaluated it (`param.expr or param.location`). The same
// document therefore carried different payloads depending on the engine.
//
// No W3C IRP fixture writes `<send><param location=.../>`, which is why
// the divergence went unnoticed; these tests pin the parse result and the
// value-expression selection directly.

#include "actions/SendAction.h"
#include "factory/NodeFactory.h"
#include "parsing/ActionParser.h"
#include "parsing/PugiXMLParser.h"

#include <gtest/gtest.h>

#include <memory>
#include <string>

namespace {

std::shared_ptr<SCE::SendAction> parseSendElement(const std::string &xml) {
    SCE::PugiXMLParser xmlParser;
    auto doc = xmlParser.parseContent(xml);
    EXPECT_TRUE(doc) << "fixture must parse";
    if (!doc) {
        return nullptr;
    }

    SCE::ActionParser actionParser(std::make_shared<SCE::NodeFactory>());
    auto action = actionParser.parseActionNode(doc->getRootElement());
    return std::dynamic_pointer_cast<SCE::SendAction>(action);
}

TEST(SendParamLocationTest, LocationParamIsParsed) {
    auto send = parseSendElement("<send xmlns=\"http://www.w3.org/2005/07/scxml\" event=\"e\" target=\"#_internal\">"
                                 "  <param name=\"aParam\" location=\"Var1\"/>"
                                 "</send>");
    ASSERT_NE(send, nullptr);

    const auto &params = send->getParamsWithExpr();
    ASSERT_EQ(params.size(), 1u) << "a location-valued param must survive parsing";
    EXPECT_EQ(params[0].name, "aParam");
    EXPECT_TRUE(params[0].expr.empty());
    EXPECT_EQ(params[0].location, "Var1");
    // The evaluated value comes from the location, matching the AOT emit.
    EXPECT_EQ(params[0].valueExpr(), "Var1");
}

TEST(SendParamLocationTest, ExprParamStillWins) {
    auto send = parseSendElement("<send xmlns=\"http://www.w3.org/2005/07/scxml\" event=\"e\" target=\"#_internal\">"
                                 "  <param name=\"aParam\" expr=\"1 + 1\"/>"
                                 "</send>");
    ASSERT_NE(send, nullptr);

    const auto &params = send->getParamsWithExpr();
    ASSERT_EQ(params.size(), 1u);
    EXPECT_EQ(params[0].expr, "1 + 1");
    EXPECT_TRUE(params[0].location.empty());
    EXPECT_EQ(params[0].valueExpr(), "1 + 1");
}

// §scxml-5.7.1 makes 'name' required; a param carrying neither value form
// has nothing to send, so both stay out of the param list rather than
// entering it with an empty expression that would evaluate to undefined.
TEST(SendParamLocationTest, NamelessAndValuelessParamsAreDropped) {
    auto nameless = parseSendElement("<send xmlns=\"http://www.w3.org/2005/07/scxml\" event=\"e\">"
                                     "  <param expr=\"1\"/>"
                                     "</send>");
    ASSERT_NE(nameless, nullptr);
    EXPECT_TRUE(nameless->getParamsWithExpr().empty());

    auto valueless = parseSendElement("<send xmlns=\"http://www.w3.org/2005/07/scxml\" event=\"e\">"
                                      "  <param name=\"aParam\"/>"
                                      "</send>");
    ASSERT_NE(valueless, nullptr);
    EXPECT_TRUE(valueless->getParamsWithExpr().empty());
}

}  // namespace
