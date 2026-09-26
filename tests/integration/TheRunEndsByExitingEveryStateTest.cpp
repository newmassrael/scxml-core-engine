// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D exitInterpreter: a run ends by exiting every state it
// is still in, the way exitStates exits one — Interpreter path.
//
// Reached two ways, and both are driven here: the run enters a top-level
// `<final>` after a step, or the host stops it. Either way every remaining
// state's `<onexit>` runs innermost first and the configuration ends empty;
// where the run ended is `terminalState()`, and the datamodel the handlers
// wrote stays readable.
//
// Sibling of `TheRunEndsByExitingEveryStateAotTest.cpp` (C++ AOT).
//
// Fixture:
// integration_resources/the_run_ends_by_exiting_every_state/the_run_ends_by_exiting_every_state.scxml

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

class TheRunEndsByExitingEveryStateTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();

        const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                    "/integration_resources/the_run_ends_by_exiting_every_state/"
                                    "the_run_ends_by_exiting_every_state.scxml";
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
        eventRaiser_ = std::make_shared<EventRaiserImpl>();
        eventRaiser_->setScheduler(scheduler);
        eventRaiser_->setImmediateMode(false);
        sm_->setEventRaiser(eventRaiser_);
        sm_->setEventDispatcher(
            std::make_shared<EventDispatcherImpl>(scheduler, std::make_shared<EventTargetFactoryImpl>(eventRaiser_)));

        ASSERT_TRUE(sm_->loadSCXMLFromString(buffer.str()));
        ASSERT_TRUE(sm_->start());
        ASSERT_TRUE(sm_->isStateActive("inner")) << "the run has to start inside `inner`";
    }

    void TearDown() override {
        sm_.reset();
        if (engine_) {
            engine_->shutdown();
        }
    }

    /// A value the handlers recorded (W3C SCXML 5.3), read from the datamodel
    /// the ended run left behind.
    std::string read(const char *name) const {
        auto result = ScriptEngineProvider::getScriptEngine().evaluateExpression(sm_->getSessionId(), name).get();
        return result.isSuccess() ? result.getValueAsString() : std::string("<unreadable>");
    }

    /// Whether an expression over those records holds. The verdicts compare in
    /// the datamodel rather than as text, which would depend on whether the
    /// engine hands a number back as an integer or a double.
    bool holds(const char *expression) const {
        return read(expression) == "true";
    }

    /// Everything the verdicts are taken from, printed together so a failure
    /// says what the run left behind and not only which clause broke.
    std::string describe() const {
        std::string active;
        for (const auto &s : sm_->getActiveStates()) {
            active += " " + s;
        }
        return "active:[" + active + " ] ended in " + sm_->terminalState().value_or("<none>") +
               " running=" + (sm_->isRunning() ? "true" : "false") + " order=" + read("order") +
               " finalExits=" + read("finalExits") + " selfInFinal=" + read("selfInFinal");
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
    std::shared_ptr<EventRaiserImpl> eventRaiser_;
};

/// The run ends in a top-level `<final>` after a step: the final is exited too.
TEST_F(TheRunEndsByExitingEveryStateTest, ARunThatReachesItsFinalExitsTheFinal) {
    ASSERT_TRUE(sm_->raiseExternalEvent("finish", ""));
    eventRaiser_->processQueuedEvents();

    EXPECT_EQ(sm_->terminalState().value_or(""), "done") << describe();
    EXPECT_FALSE(sm_->isRunning()) << describe();
    EXPECT_TRUE(sm_->getActiveStates().empty())
        << "exitInterpreter deletes every state it exits, the final included. " << describe();
    EXPECT_TRUE(holds("finalExits === 1"))
        << "the final's own <onexit> must run exactly once as the run ends. " << describe();
    EXPECT_TRUE(holds("selfInFinal === 1"))
        << "`done` must still be in the configuration during its own <onexit>. " << describe();
    EXPECT_TRUE(holds("order === 12")) << "`finish` exits `inner` then `outer`. " << describe();
}

/// The host stops a run that has not ended: every state is exited, innermost first.
TEST_F(TheRunEndsByExitingEveryStateTest, AStoppedRunExitsEveryStateInnermostFirst) {
    sm_->stop();

    EXPECT_FALSE(sm_->terminalState().has_value()) << "a stopped run did not end in a final. " << describe();
    EXPECT_FALSE(sm_->isRunning()) << describe();
    EXPECT_TRUE(sm_->getActiveStates().empty()) << describe();
    EXPECT_TRUE(holds("order === 12"))
        << "stop() must run `inner`'s <onexit> and then `outer`'s: 0 is a stop that exited nothing, "
           "21 one that exited in document order instead of exit order. "
        << describe();
    EXPECT_TRUE(holds("finalExits === 0")) << "the run never entered `done`. " << describe();
}

}  // namespace Tests
}  // namespace SCE
