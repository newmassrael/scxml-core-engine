// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 `<send>` `<param>` payload delivery — Interpreter path.
//
// Sibling of `SendParamPayloadAotTest.cpp`. Both engines ship in
// production — Interpreter for embedded hosting, AOT for codegen consumers
// — so each is held to the payload contract independently against one
// canonical fixture.
//
// Two paths, reaching distinct final states so a failure names which one:
// a `<send target="#_parent">` from a child that needs no script engine,
// and a `<send target="#_internal">` whose params must arrive as
// `_event.data` on the receiving transition.
//
// Fixture: integration_resources/send_param_payload/send_param_payload.scxml
// — the canonical source all six backends compile.

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

class SendParamPayloadTest : public ::testing::Test {
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

TEST_F(SendParamPayloadTest, SendParamsReachEventDataFromChildAndInternalQueue) {
    const std::string fixture =
        std::string(SCE_PROJECT_ROOT) + "/integration_resources/send_param_payload/send_param_payload.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();

    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());

    // §scxml-6.2: `<send target="#_parent">` needs the dispatcher chain
    // (scheduler → target factory → dispatcher), same as the autoforward
    // sibling — this fixture's child machine actually sends.
    auto scheduler = std::make_shared<EventSchedulerImpl>(
        [](const EventDescriptor &event, std::shared_ptr<IEventTarget> target, const std::string &) -> bool {
            (void)event;
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
    // state name freezes on the verdict. Poll `isRunning()` for the same
    // reason the autoforward sibling does.
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (std::chrono::steady_clock::now() < deadline) {
        if (!sm->isRunning()) {
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    ASSERT_FALSE(sm->isRunning()) << "parent did not halt within 5s — it never saw `fromChild` or "
                                  << "never saw its own `loopback`";

    const std::string reached = sm->getCurrentState();
    EXPECT_NE(reached, "failChildPayload")
        << "`fromChild` arrived without `_event.data.value`: a `datamodel=\"null\"` child needs no "
        << "script engine, but its `<send>` still has to carry the params it declares.";
    EXPECT_NE(reached, "failInternalPayload")
        << "`loopback` arrived without `_event.data.carried`: a `<send target=\"#_internal\">` must "
        << "raise its params as event data, not build them and drop them at the internal-raise boundary.";
    EXPECT_EQ(reached, "pass");
}

}  // namespace Tests
}  // namespace SCE
