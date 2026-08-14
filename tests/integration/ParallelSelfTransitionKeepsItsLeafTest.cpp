// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4: a region that took an external self-transition still holds an
// atomic state, so it answers the next event too — Interpreter path.
//
// The defect this axis exists for was found on the AOT channel, and pinning it
// here is not an assumption that the Interpreter shares the bug. It is the
// same reason its sibling fixture is pinned on both: a region left holding no
// atomic state is a configuration a `<parallel>` must never reach, and each
// engine maintains that configuration through its own code. An engine that is
// right today is not thereby asked less.
//
// Sibling of `ParallelSelfTransitionKeepsItsLeafAotTest.cpp` (C++ AOT).
//
// Fixture:
// integration_resources/parallel_self_transition_keeps_its_leaf/parallel_self_transition_keeps_its_leaf.scxml

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <algorithm>
#include <fstream>
#include <gtest/gtest.h>
#include <sstream>
#include <string>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

class ParallelSelfTransitionKeepsItsLeafTest : public ::testing::Test {
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

TEST_F(ParallelSelfTransitionKeepsItsLeafTest, TheSelfTransitionedRegionAnswersTheNextEvent) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/parallel_self_transition_keeps_its_leaf/"
                                "parallel_self_transition_keeps_its_leaf.scxml";
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

    const auto describe = [&sm]() {
        std::string out;
        for (const auto &s : sm->getActiveStates()) {
            out += " " + s;
        }
        return out;
    };

    ASSERT_TRUE(sm->isStateActive("within"))
        << "the fixture is supposed to start with the self-transitioning region in `within`; it "
           "did not, so nothing below is testing what it claims. active:"
        << describe();
    ASSERT_TRUE(sm->isStateActive("working"))
        << "the fixture is supposed to start with the deeper region in `working`; it did not, so "
           "nothing below is testing what it claims. active:"
        << describe();

    // The raiser is in queued mode (see setImmediateMode(false) above), so each
    // event sits on the external queue until the drain runs — the same
    // macrostep boundary the engine uses for its own sends.
    ASSERT_TRUE(sm->raiseExternalEvent("e", ""));
    eventRaiser->processQueuedEvents();

    EXPECT_TRUE(sm->isStateActive("within"))
        << "the self-transitioning region lost its leaf on the first event. `within` is both the "
           "source and the target of its own external self-transition, so the microstep exits and "
           "re-enters it. active:"
        << describe();
    EXPECT_TRUE(sm->isStateActive("judging"))
        << "the deeper region did not take its own transition on `e`. active:" << describe();

    // The second event is the one this fixture exists for: `judging` has no `e`
    // transition, so nothing but the self-transitioning region can answer, and
    // it can only answer from a leaf.
    ASSERT_TRUE(sm->raiseExternalEvent("e", ""));
    eventRaiser->processQueuedEvents();

    EXPECT_TRUE(sm->isStateActive("within"))
        << "the self-transitioning region is not in `within` after the second `e`. active:" << describe();

    ASSERT_TRUE(sm->raiseExternalEvent("check", ""));
    eventRaiser->processQueuedEvents();

    EXPECT_TRUE(sm->isStateActive("settled"))
        << "`check` did not carry the machine to `settled`, which the document guards on "
           "`n == 1 && m == 2`. `m` reaches 2 only if the self-transitioning region still had a "
           "leaf to transition from when the second `e` arrived. active:"
        << describe();
}

}  // namespace Tests
}  // namespace SCE
