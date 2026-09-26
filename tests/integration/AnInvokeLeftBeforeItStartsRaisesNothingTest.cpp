// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: an invoke whose state leaves within its macrostep is never
// attempted, so the §6.4.1 error.execution for a type no processor runs is
// never raised — Interpreter path.
//
// Measured 2026-09-26, this channel already raised nothing; the fixture pins
// it.
//
// Sibling of `AnInvokeLeftBeforeItStartsRaisesNothingAotTest.cpp` (C++ AOT).
//
// Fixture:
// integration_resources/an_invoke_left_before_it_starts_raises_nothing/an_invoke_left_before_it_starts_raises_nothing.scxml

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

class AnInvokeLeftBeforeItStartsRaisesNothingTest : public ::testing::Test {
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

TEST_F(AnInvokeLeftBeforeItStartsRaisesNothingTest, TheLeftStatesInvokeIsNeverAttempted) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/an_invoke_left_before_it_starts_raises_nothing/"
                                "an_invoke_left_before_it_starts_raises_nothing.scxml";
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
    ASSERT_TRUE(sm_->isStateActive("s1")) << "`s0` leaves on an eventless transition within the first macrostep";

    ASSERT_TRUE(sm_->raiseExternalEvent("finish", ""));
    eventRaiser->processQueuedEvents();

    EXPECT_EQ(sm_->terminalState().value_or(""), "done") << "`finish` must carry the run to `done`";
    const std::string errors = read("errors");
    EXPECT_EQ(read("errors === 0"), "true")
        << "`s0` left before its macrostep ended, yet its invoke was attempted and raised error.execution (errors="
        << errors << "); W3C SCXML 6.4 cancels an invoke whose state has left";
}

}  // namespace Tests
}  // namespace SCE
