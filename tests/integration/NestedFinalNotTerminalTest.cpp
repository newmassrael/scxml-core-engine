// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.7: only a top-level `<final>` ends the session — Interpreter path.
//
// Appendix D `enterStates` sets `running = false` for a `<final>` only when
// `isSCXMLElement(s.parent)`; otherwise it queues `done.state.<parent>` and the
// machine carries on. "Is this state a `<final>` element" is the structural
// question and not the completion criterion.
//
// The fixture rests in the nested final rather than passing through it: a
// machine that continues within the same macrostep is only ever sampled at the
// end, where a right and a wrong predicate agree.
//
// Sibling of `NestedFinalNotTerminalAotTest.cpp` (C++ AOT channel). Both
// engines ship in production, so each is held to the clause independently
// against one canonical fixture.
//
// Fixture: integration_resources/nested_final_not_terminal/nested_final_not_terminal.scxml

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

class NestedFinalNotTerminalTest : public ::testing::Test {
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

TEST_F(NestedFinalNotTerminalTest, ANestedFinalDoesNotEndTheSession) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/nested_final_not_terminal/nested_final_not_terminal.scxml";
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

    ASSERT_EQ(sm->getCurrentState(), "phaseDone")
        << "the fixture is supposed to come to rest in the nested `<final>`; it did not, "
           "so nothing below is testing what it claims";
    EXPECT_TRUE(sm->isRunning()) << "the session ended at `phaseDone`, a `<final>` nested inside `phase`. W3C SCXML "
                                    "Appendix D `enterStates` ends the session only when the final's parent is the "
                                    "`<scxml>` element — a nested one finishes its compound state and queues "
                                    "`done.state.phase`, leaving the machine live.";
    EXPECT_FALSE(sm->isInFinalState()) << "the engine reported completion while resting in a nested `<final>`";

    // The raiser is in queued mode (see setImmediateMode(false) above), so the
    // event sits on the external queue until the drain runs — the same
    // macrostep boundary the engine uses for its own sends.
    ASSERT_TRUE(sm->raiseExternalEvent("resume", ""));
    eventRaiser->processQueuedEvents();

    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (std::chrono::steady_clock::now() < deadline && sm->isRunning()) {
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    EXPECT_EQ(sm->getCurrentState(), "pass")
        << "`resume` did not carry the machine out of the nested final to the top-level one";
}

}  // namespace Tests
}  // namespace SCE
