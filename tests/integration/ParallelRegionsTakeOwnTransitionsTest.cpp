// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4: every region of a `<parallel>` takes its own enabled
// transition in the same microstep — Interpreter path.
//
// Both engines reach a transition's domain through the same shared helper, so
// the clause is pinned on this channel too rather than assumed to follow from
// the AOT sibling. `findLCA` answered a self-transition's domain with the state
// itself, where Appendix D `findLCCA` searches `getProperAncestors` and can
// only ever answer with an ancestor; the exit set computed from that answer ran
// to the document root and named the enclosing `<parallel>`, which is enough
// for conflict resolution to preempt a sibling region's transition on the same
// event.
//
// The observable is `settled`, which the document reaches only when both
// regions' assignments have run — a configuration check alone would still pass
// for a region that moved without executing its transition content.
//
// Sibling of `ParallelRegionsTakeOwnTransitionsAotTest.cpp` (C++ AOT channel).
//
// Fixture:
// integration_resources/parallel_regions_take_own_transitions/parallel_regions_take_own_transitions.scxml

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

class ParallelRegionsTakeOwnTransitionsTest : public ::testing::Test {
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

TEST_F(ParallelRegionsTakeOwnTransitionsTest, EveryRegionTakesItsOwnTransition) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/parallel_regions_take_own_transitions/"
                                "parallel_regions_take_own_transitions.scxml";
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

    ASSERT_TRUE(sm->isStateActive("working"))
        << "the fixture is supposed to start with the deeper region in `working`; it did not, "
           "so nothing below is testing what it claims. active:"
        << describe();
    ASSERT_TRUE(sm->isStateActive("within"))
        << "the fixture is supposed to start with the shallower region in `within`; it did not, "
           "so nothing below is testing what it claims";

    // The raiser is in queued mode (see setImmediateMode(false) above), so each
    // event sits on the external queue until the drain runs — the same
    // macrostep boundary the engine uses for its own sends.
    ASSERT_TRUE(sm->raiseExternalEvent("e", ""));
    eventRaiser->processQueuedEvents();

    EXPECT_TRUE(sm->isStateActive("judging"))
        << "the deeper region lost its leaf. W3C SCXML 3.4 has every region take its own enabled "
           "transition on `e`; the sibling region's external self-transition must not preempt this "
           "one. Appendix D reaches a self-transition's domain through `findLCCA`, whose candidates "
           "come from `getProperAncestors` and therefore never include the state itself — an exit "
           "set that names the enclosing `<parallel>` is the symptom of answering otherwise. active:"
        << describe();
    EXPECT_TRUE(sm->isStateActive("within"))
        << "the shallower region left `within`, which is both the source and the target of its own "
           "external self-transition";

    ASSERT_TRUE(sm->raiseExternalEvent("check", ""));
    eventRaiser->processQueuedEvents();

    EXPECT_TRUE(sm->isStateActive("settled"))
        << "`check` did not carry the machine to `settled`, which the document guards on both "
           "regions' assignments having run. Reaching `judging` without `n == 1 && m == 1` means a "
           "region changed state while its transition content was skipped.";
}

}  // namespace Tests
}  // namespace SCE
