// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10 + 6.2: a <send>'s payload is the data of the event it sends,
// and a data-less event dequeued before it carries none — Interpreter path.
//
// Sibling of `APayloadRidesOnItsOwnEventAotTest.cpp` (C++ AOT).
//
// Fixture:
// integration_resources/a_payload_rides_on_its_own_event/a_payload_rides_on_its_own_event.scxml

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

class APayloadRidesOnItsOwnEventTest : public ::testing::Test {
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

    std::string describe() const {
        return "ended in " + sm_->terminalState().value_or("<none>") + ", got=" + read("got") +
               " stolen=" + read("stolen") + " plains=" + read("plains") + " (wanted done / 4 / 0 / 4)";
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
};

TEST_F(APayloadRidesOnItsOwnEventTest, EachPayloadArrivesOnItsOwnEvent) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/a_payload_rides_on_its_own_event/"
                                "a_payload_rides_on_its_own_event.scxml";
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
    ASSERT_TRUE(sm_->raiseExternalEvent("finish", ""));
    eventRaiser->processQueuedEvents();

    EXPECT_EQ(sm_->terminalState().value_or(""), "done") << "`finish` must carry the run to `done`. " << describe();
    EXPECT_EQ(read("got === 4"), "true") << "each payload event arrives carrying its own payload. " << describe();
    EXPECT_EQ(read("stolen === 0"), "true") << "no data-less event arrives carrying a payload. " << describe();
    EXPECT_EQ(read("plains === 4"), "true") << "every data-less event arrives, and arrives empty. " << describe();
}

}  // namespace Tests
}  // namespace SCE
