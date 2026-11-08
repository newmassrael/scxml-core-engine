// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-4.7.1: a <log> carries a label and an expression, and the message the
// Interpreter writes is the label, then the value the expression returns, as
// the generated engines write it (tools/codegen/templates/actions/log.jinja2).
//
// The ActionParser used to keep one of the two attributes: the expr, or else
// the label, which it then evaluated as an expression. And the executor logged
// an expression's own source in place of a value that read as empty. Two <log>s
// of one value under different labels were then indistinguishable, and two
// expressions that both yield nothing read as two different values: the manual
// W3C tests 230 and 307 read their verdicts from exactly those messages
// (tests/w3c/ManualVerification.cpp).

#include "actions/LogAction.h"
#include "common/Logger.h"
#include "factory/NodeFactory.h"
#include "parsing/ActionParser.h"
#include "parsing/PugiXMLParser.h"
#include "runtime/ActionExecutorImpl.h"
#include "scripting/JSEngine.h"

#include <gtest/gtest.h>

#include <memory>
#include <string>
#include <vector>

namespace {

std::shared_ptr<SCE::LogAction> parseLog(const std::string &attributes) {
    SCE::PugiXMLParser xmlParser;
    auto doc = xmlParser.parseContent("<log xmlns=\"http://www.w3.org/2005/07/scxml\" " + attributes + "/>");
    EXPECT_TRUE(doc) << "fixture must parse as XML";
    if (!doc) {
        return nullptr;
    }
    SCE::ActionParser actionParser(std::make_shared<SCE::NodeFactory>());
    return std::dynamic_pointer_cast<SCE::LogAction>(actionParser.parseActionNode(doc->getRootElement()));
}

TEST(LogParsing, TheLabelIsKeptBesideTheExpr) {
    // The label as the document has it, its trailing space included
    auto both = parseLog("label=\"name is \" expr=\"_event.name\"");
    ASSERT_NE(both, nullptr);
    EXPECT_EQ(both->getLabel(), "name is ");
    EXPECT_EQ(both->getExpr(), "_event.name");
}

TEST(LogParsing, ALabelAloneIsNotAnExpression) {
    auto labelOnly = parseLog("label=\"asked\"");
    ASSERT_NE(labelOnly, nullptr);
    EXPECT_EQ(labelOnly->getLabel(), "asked");
    EXPECT_TRUE(labelOnly->getExpr().empty()) << "the label was taken for the expression";
}

class LogMessageTest : public ::testing::Test {
protected:
    void SetUp() override {
        jsEngine = &SCE::JSEngine::instance();
        jsEngine->reset();
        ASSERT_TRUE(jsEngine->createSession(sessionId, ""));
        executor = std::make_unique<SCE::ActionExecutorImpl>(sessionId, SCE::JSEngine::instance());
    }

    void TearDown() override {
        if (jsEngine && jsEngine->hasSession(sessionId)) {
            jsEngine->destroySession(sessionId);
        }
        if (jsEngine) {
            jsEngine->shutdown();
        }
    }

    // What the logger captured while the executor ran each <log>, in order
    std::string loggedBy(const std::vector<std::shared_ptr<SCE::LogAction>> &actions) {
        SCE::Logger::startCapture();
        for (const auto &action : actions) {
            EXPECT_TRUE(executor->executeLogAction(*action));
        }
        SCE::Logger::stopCapture();
        return SCE::Logger::getCapturedLogs();
    }

    SCE::JSEngine *jsEngine{nullptr};
    std::string sessionId{"log_message_test_session"};
    std::unique_ptr<SCE::ActionExecutorImpl> executor;
};

TEST_F(LogMessageTest, TheMessageIsTheLabelThenTheValue) {
    auto labelled = std::make_shared<SCE::LogAction>("1 + 1");
    labelled->setLabel("sum");
    auto unlabelled = std::make_shared<SCE::LogAction>("'plain'");
    auto labelOnly = std::make_shared<SCE::LogAction>();
    labelOnly->setLabel("no value");

    const std::string logs = loggedBy({labelled, unlabelled, labelOnly});
    ASSERT_FALSE(logs.empty()) << "the logger backend captured nothing";
    EXPECT_NE(logs.find("SCXML: sum: 2\n"), std::string::npos) << logs;
    EXPECT_NE(logs.find("SCXML: plain\n"), std::string::npos) << logs;
    EXPECT_NE(logs.find("SCXML: no value\n"), std::string::npos) << logs;
}

TEST_F(LogMessageTest, AnEmptyValueIsLoggedEmptyNotAsItsExpression) {
    auto undefinedValue = std::make_shared<SCE::LogAction>("undefined");
    undefinedValue->setLabel("value");

    const std::string logs = loggedBy({undefinedValue});
    ASSERT_FALSE(logs.empty()) << "the logger backend captured nothing";
    EXPECT_NE(logs.find("SCXML: value: \n"), std::string::npos) << logs;
    EXPECT_EQ(logs.find("SCXML: value: undefined"), std::string::npos) << logs;
}

}  // namespace
