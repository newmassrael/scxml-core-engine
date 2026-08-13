// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4 + 3.7: `done.state.<parallel>` is delivered, not merely
// declared — Interpreter path.
//
// The AOT sibling asks this of a generated machine, where "declared" and
// "delivered" are two different mechanisms: an enumerator the parser registers
// and a raise the emitter writes. The Interpreter has no enumeration at all —
// it builds its event set at run time — so for it the question collapses to
// the one that matters everywhere: when the last region reaches its `<final>`,
// does a transition selected on the parallel's completion actually run?
//
// Worth asking on both precisely because the two engines get here by different
// routes. The generated side can fail by naming an event nothing raises; this
// side can fail by raising one nothing selects.
//
// Sibling of `ParallelDoneStateIsDeliveredAotTest.cpp` (C++ AOT channel).
//
// Fixture:
// integration_resources/parallel_done_state_is_delivered/parallel_done_state_is_delivered.scxml

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <fstream>
#include <gtest/gtest.h>
#include <sstream>
#include <string>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

TEST(ParallelDoneStateIsDeliveredTest, CompletionCarriesTheMachineToATopLevelFinal) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/parallel_done_state_is_delivered/"
                                "parallel_done_state_is_delivered.scxml";
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

    ASSERT_TRUE(sm->isStateActive("a1") && sm->isStateActive("b1"))
        << "the fixture is supposed to start inside the parallel; it did not, "
           "so nothing below is testing what it claims";

    // Queued mode (see setImmediateMode(false) above), so the event sits on the
    // external queue until the drain runs — the same macrostep boundary the
    // engine uses for its own sends, which is where the completion event this
    // test is about will also appear.
    ASSERT_TRUE(sm->raiseExternalEvent("go", ""));
    eventRaiser->processQueuedEvents();

    // One assertion, because the two ways this can fail are not separately
    // observable: completion is selected within the SAME macrostep as the
    // regions' finals, so by the time the drain returns the parallel has been
    // exited and `a2`/`b2` are gone. Measured — asserting them as a
    // precondition failed against an engine that had already done the right
    // thing.
    //
    // What tells the two apart is which states ARE active: `a1`/`b1` means
    // `go` moved nothing, `a2`/`b2` means the parallel completed and the
    // completion event went nowhere.
    EXPECT_TRUE(sm->isStateActive("settled"))
        << "every region reaching its `<final>` completes the parallel, so `done.state.run` "
           "had to be raised AND selected — `settled` is reachable by nothing else. Still "
           "inside the parallel: a1="
        << sm->isStateActive("a1") << " a2=" << sm->isStateActive("a2") << " b1=" << sm->isStateActive("b1")
        << " b2=" << sm->isStateActive("b2");
}

}  // namespace Tests
}  // namespace SCE
