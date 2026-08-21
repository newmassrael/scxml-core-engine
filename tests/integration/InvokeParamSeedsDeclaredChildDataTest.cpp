// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.4.3: an `<invoke>` `<param>` seeds a declared `<data>` of the
// invoked session with the INVOKING session's value — Interpreter path.
//
// Sibling of `InvokeParamSeedsDeclaredChildDataAotTest.cpp`. Both engines
// ship in production — Interpreter for embedded hosting, AOT for codegen
// consumers — so each is held to the clause independently against one
// canonical fixture. `InvokeExecutor` evaluates each param in the invoking
// session and calls `setInvokeDataVariable` with the resulting VALUE, and it
// did so before the AOT template did — the divergence this fixture found was
// inside one language, on the other engine. This test is the half that says
// the Interpreter is the reference rather than assuming it.
//
// Each phase reaches its own final state so a failure names the sentence
// that broke.
//
// Fixture: integration_resources/invoke_param_seeds_declared_child_data/invoke_param_seeds_declared_child_data.scxml
// — the canonical source all seven channels compile.

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <fstream>
#include <gtest/gtest.h>
#include <sstream>
#include <thread>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

class InvokeParamSeedsDeclaredChildDataTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();
    }

    void TearDown() override {
        if (engine_) {
            engine_->shutdown();
        }
    }

    IScriptEngine *engine_;
};

TEST_F(InvokeParamSeedsDeclaredChildDataTest, InvokeParamCarriesTheInvokingSessionsValue) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/invoke_param_seeds_declared_child_data/"
                                "invoke_param_seeds_declared_child_data.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();

    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());

    // §scxml-6.2: each child answers with `<send target="#_parent">`, which
    // needs the dispatcher chain (scheduler → target factory → dispatcher)
    // wired on the Interpreter — the same setup `SendParamPayloadTest` makes
    // for the same reason. The AOT sibling needs none of it: its child holds a
    // pointer to the parent machine.
    auto scheduler = std::make_shared<EventSchedulerImpl>(
        [](const EventDescriptor &event, std::shared_ptr<IEventTarget> target, const std::string &) -> bool {
            try {
                return target->send(event).get().isSuccess;
            } catch (...) {
                return false;
            }
        });
    auto eventRaiser = std::make_shared<EventRaiserImpl>();
    eventRaiser->setScheduler(scheduler);
    eventRaiser->setImmediateMode(false);
    sm->setEventRaiser(eventRaiser);
    sm->setEventDispatcher(
        std::make_shared<EventDispatcherImpl>(scheduler, std::make_shared<EventTargetFactoryImpl>(eventRaiser)));

    ASSERT_TRUE(sm->loadSCXMLFromString(buffer.str()));
    ASSERT_TRUE(sm->start());

    // §scxml-3.13: reaching a top-level `<final>` halts processing, so the
    // state name freezes on the verdict. Poll `isRunning()` rather than
    // `isInFinalState()`, which short-circuits on the halted flag.
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (std::chrono::steady_clock::now() < deadline) {
        if (!sm->isRunning()) {
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    ASSERT_FALSE(sm->isRunning()) << "parent did not halt within 5s — one of the four children never "
                                  << "reached its `<send target=\"#_parent\">`, so no verdict arrived";

    const std::string reached = sm->getCurrentState();
    EXPECT_NE(reached, "failChildEvaluatedTheExpression")
        << "the child evaluated the author's `<param expr>` text in its own data model and found "
        << "its own `token`: §scxml-6.4.3 says the VALUE of the param element, and only the "
        << "invoking session can produce it.";
    EXPECT_NE(reached, "failParentOnlyExprLost")
        << "a `<param expr>` naming a variable only the parent declares arrived as nothing: the "
        << "same defect as above where the child has no shadow to find.";
    EXPECT_NE(reached, "failUnmatchedParamEnteredTheChild")
        << "a `<param>` naming no top-level `<data>` of the child became a variable there: "
        << "§scxml-6.4.3 says the Processor MUST NOT add it to the invoked session's data model.";
    EXPECT_NE(reached, "failNamelistValueLost")
        << "the `namelist` value did not arrive: §scxml-6.4.1 says the value stored at the "
        << "location is the value, so a rendered string forwarded as an expression becomes an "
        << "identifier lookup in the child.";
    EXPECT_NE(reached, "failShadowSeedLost")
        << "the child saw neither the parent's value nor its own shadow, so its `<data>` default "
        << "stood: nothing was seeded at all.";
    EXPECT_NE(reached, "failDeclaredParamLost")
        << "the param that DOES name a declared `<data>` of the child did not arrive, so the "
        << "filter for the unmatched one took the declared one with it.";
    EXPECT_EQ(reached, "pass");
}

}  // namespace Tests
}  // namespace SCE
