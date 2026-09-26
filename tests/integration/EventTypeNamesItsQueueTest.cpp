// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1: `_event.type` names the queue an event was taken from —
// Interpreter path.
//
// The document queues an external event first and two internal ones after
// it; the internal queue drains first, so each event is processed while the
// other queue still holds something. Measured 2026-09-26, this channel already
// decided the type at dequeue; the fixture pins it.
//
// Sibling of `EventTypeNamesItsQueueAotTest.cpp` (C++ AOT).
//
// Fixture:
// integration_resources/event_type_names_its_queue/event_type_names_its_queue.scxml

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

class EventTypeNamesItsQueueTest : public ::testing::Test {
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

    /// Whether an expression over those records holds, compared in the
    /// datamodel rather than as text.
    bool holds(const char *expression) const {
        return read(expression) == "true";
    }

    std::string describe() const {
        return "ended in " + sm_->terminalState().value_or("<none>") + ", intCode=" + read("intCode") +
               " sendCode=" + read("sendCode") + " extCode=" + read("extCode") +
               " (1 internal, 2 external, 3 other; wanted 1 / 1 / 2)";
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
};

TEST_F(EventTypeNamesItsQueueTest, EachEventIsTypedByTheQueueItWasTakenFrom) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/event_type_names_its_queue/event_type_names_its_queue.scxml";
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
    // The document queues its own events; drain whatever `start` left queued.
    eventRaiser->processQueuedEvents();

    EXPECT_EQ(sm_->terminalState().value_or(""), "done") << "`ext` must carry the run to `done`. " << describe();
    EXPECT_TRUE(holds("intCode === 1")) << "`int` came off the internal queue while `ext` waited on the external one. "
                                        << describe();
    EXPECT_TRUE(holds("sendCode === 1"))
        << "a `<send target=\"#_internal\">` with a payload rides the internal queue too. " << describe();
    EXPECT_TRUE(holds("extCode === 2")) << "`ext` came off the external queue. " << describe();
}

}  // namespace Tests
}  // namespace SCE
