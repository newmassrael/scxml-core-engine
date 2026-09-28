// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML C.1: an event a session sends to itself names its origin, and a
// target expression that evaluates to nothing reaches no one — C++ Interpreter.
//
// The reference the other channels are held to: the Interpreter creates the
// sender's target once, before the delay decision, so the immediate and the
// delayed event carry the same origin, and `SendHelper::isUnreachableTarget`
// refuses the blank one.
//
// Fixture: integration_resources/a_self_sent_event_names_its_origin/a_self_sent_event_names_its_origin.scxml

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>
#include <string>
#include <thread>
#include <utility>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

class ASelfSentEventNamesItsOriginTest : public ::testing::Test {
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
    std::string read(const std::string &name) const {
        auto result = ScriptEngineProvider::getScriptEngine().evaluateExpression(sm_->getSessionId(), name).get();
        return result.isSuccess() ? result.getValueAsString() : std::string("<unreadable>");
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
};

TEST_F(ASelfSentEventNamesItsOriginTest, ASelfSentEventNamesItsOrigin) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/a_self_sent_event_names_its_origin/"
                                "a_self_sent_event_names_its_origin.scxml";

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

    ASSERT_TRUE(sm_->loadSCXML(fixture)) << "canonical fixture not loadable: " << fixture;
    ASSERT_TRUE(sm_->start());
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (std::chrono::steady_clock::now() < deadline && sm_->isRunning()) {
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
        eventRaiser->processQueuedEvents();
    }

    EXPECT_EQ(sm_->terminalState().value_or(""), "done") << "the run must end in `done`";
    const std::vector<std::pair<std::string, int>> expected = {
        {"immediateOk", 1}, {"replied", 1}, {"delayedOk", 1}, {"unreachable", 1}, {"strayed", 0}};
    for (const auto &[name, value] : expected) {
        EXPECT_EQ(read(name + " === " + std::to_string(value)), "true")
            << name << " = " << read(name) << ", want " << value;
    }
}

}  // namespace Tests
}  // namespace SCE
