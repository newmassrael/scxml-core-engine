// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 6.3: `<cancel>` drops a delayed `<send>` that has not been
// dispatched — Interpreter path.
//
// The Interpreter does not have the AOT engines' question of when the host
// ticks: `EventSchedulerImpl` owns a thread and fires each entry at its own
// deadline, so the two sends here are always dispatched a hundred milliseconds
// apart no matter what the caller does. That is exactly why this channel is
// worth asserting alongside the six pull-driven ones — it pins the verdict the
// document is supposed to reach when nothing can coalesce the deadlines, and a
// pull-driven backend that disagrees with it is diverging from an engine that
// ships in the same repository rather than from a rule written only in a test.
//
// Sibling of `LateTickHonoursCancelAotTest.cpp` (C++ AOT channel). Both
// engines ship in production, so each is held to the clause independently
// against one canonical fixture.
//
// Fixture: integration_resources/late_tick_honours_cancel/late_tick_honours_cancel.scxml

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

class LateTickHonoursCancelTest : public ::testing::Test {
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

TEST_F(LateTickHonoursCancelTest, ACancelledSettleTimerIsNeverDelivered) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/late_tick_honours_cancel/late_tick_honours_cancel.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();

    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());

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

    ASSERT_EQ(sm->getCurrentState(), "waiting") << "the machine should be waiting on its two delayed sends";

    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (std::chrono::steady_clock::now() < deadline && sm->isRunning()) {
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
        eventRaiser->processQueuedEvents();
    }

    EXPECT_NE(sm->getCurrentState(), "cancelLost")
        << "`settle` was delivered even though `active`'s `<cancel sendid=\"s1\">` ran "
           "first. W3C SCXML 6.3 cancels a send that has not been dispatched yet";
    EXPECT_EQ(sm->getCurrentState(), "pass")
        << "the machine did not reach `pass`; the settle timer was armed, cancelled by "
           "the earlier `poke`, and `finish` should have carried it home";
}

}  // namespace Tests
}  // namespace SCE
