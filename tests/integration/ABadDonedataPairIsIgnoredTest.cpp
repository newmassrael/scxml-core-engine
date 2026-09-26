// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.7: a <donedata> pair that cannot be evaluated raises
// error.execution and is ignored; the done events are raised all the same —
// Interpreter path.
//
// Measured 2026-09-27, this channel treated an empty `location` as a
// structural error that withheld done.state.<parent> and the <parallel>'s
// done event; §5.7 says to ignore the pair, not the event.
//
// Sibling of `ABadDonedataPairIsIgnoredAotTest.cpp` (C++ AOT).
//
// Fixture:
// integration_resources/a_bad_donedata_pair_is_ignored/a_bad_donedata_pair_is_ignored.scxml

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

class ABadDonedataPairIsIgnoredTest : public ::testing::Test {
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
        return "ended in " + sm_->terminalState().value_or("<none>") + ", errors=" + read("errors") +
               " shape=" + read("shape") + " (wanted done / 2 / 1)";
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
};

TEST_F(ABadDonedataPairIsIgnoredTest, TheBadPairsAreDroppedAndTheDoneEventsStillArrive) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) + "/integration_resources/a_bad_donedata_pair_is_ignored/"
                                                                "a_bad_donedata_pair_is_ignored.scxml";
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
    // The run needs nothing from the host; drain whatever `start` left queued.
    eventRaiser->processQueuedEvents();

    EXPECT_EQ(sm_->terminalState().value_or(""), "done")
        << "done.state.p must still arrive and carry the run to `done`. " << describe();
    EXPECT_EQ(read("errors === 2"), "true") << "each ignored pair raises its own error.execution. " << describe();
    EXPECT_EQ(read("shape === 1"), "true")
        << "done.state.r1 must carry the surviving pair and neither ignored one. " << describe();
}

}  // namespace Tests
}  // namespace SCE
