// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: cancelling an invocation raises no event in the invoking
// session — Interpreter path.
//
// `p` invokes a child that never finishes; the host sends `leave`, which exits
// `p` and cancels the child, and `s2` counts any `cancel.invoke` that reaches
// it. Measured 2026-09-26, this channel already raised nothing; the fixture
// pins it.
//
// Sibling of `CancellingAnInvokeRaisesNothingAotTest.cpp` (C++ AOT).
//
// Fixture:
// integration_resources/cancelling_an_invoke_raises_nothing/cancelling_an_invoke_raises_nothing.scxml

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <fstream>
#include <gtest/gtest.h>
#include <memory>
#include <sstream>
#include <string>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

class CancellingAnInvokeRaisesNothingTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();
    }

    void TearDown() override {
        sm_.reset();
        if (engine_) {
            engine_->shutdown();
        }
    }

    /// A value the handlers recorded (W3C SCXML 5.3), for the failure message.
    std::string read(const char *name) const {
        auto result = ScriptEngineProvider::getScriptEngine().evaluateExpression(sm_->getSessionId(), name).get();
        return result.isSuccess() ? result.getValueAsString() : std::string("<unreadable>");
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
};

TEST_F(CancellingAnInvokeRaisesNothingTest, LeavingTheInvokingStateRaisesNoCancelEvent) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/cancelling_an_invoke_raises_nothing/"
                                "cancelling_an_invoke_raises_nothing.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();

    sm_ = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());
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
    sm_->setEventRaiser(eventRaiser);
    sm_->setEventDispatcher(
        std::make_shared<EventDispatcherImpl>(scheduler, std::make_shared<EventTargetFactoryImpl>(eventRaiser)));

    ASSERT_TRUE(sm_->loadSCXMLFromString(buffer.str()));
    ASSERT_TRUE(sm_->start());
    eventRaiser->processQueuedEvents();
    ASSERT_TRUE(sm_->isStateActive("p")) << "the run has to start in `p`, with its child invoked";

    for (const char *event : {"leave", "finish"}) {
        ASSERT_TRUE(sm_->raiseExternalEvent(event, ""));
        eventRaiser->processQueuedEvents();
    }

    EXPECT_EQ(sm_->terminalState().value_or(""), "done") << "`finish` must carry the run to `done`";
    const std::string spurious = read("spurious");
    EXPECT_EQ(read("spurious === 0"), "true")
        << "leaving `p` cancelled its child, and a `cancel.invoke` event reached the invoking session (spurious="
        << spurious << "); W3C SCXML 6.4 defines no such event";
}

}  // namespace Tests
}  // namespace SCE
