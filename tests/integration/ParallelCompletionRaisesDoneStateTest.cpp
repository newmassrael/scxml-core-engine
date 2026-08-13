// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4 + 3.7: a `<parallel>` completing raises `done.state.<id>` —
// Interpreter path.
//
// The AOT sibling pins the same clause against a generated machine, where the
// defect showed up as an undeclared enumerator. The Interpreter builds its
// event set at run time and so cannot fail that way, which is exactly why it
// is worth asking here too: the two engines arrive at "the parallel is done"
// by different routes, and only one of them was ever going to fail loudly.
//
// Sibling of `ParallelCompletionRaisesDoneStateAotTest.cpp` (C++ AOT channel).
//
// Fixture:
// integration_resources/parallel_completion_raises_done_state/parallel_completion_raises_done_state.scxml

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

TEST(ParallelCompletionRaisesDoneStateTest, EveryRegionFinalCompletesTheParallel) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/parallel_completion_raises_done_state/"
                                "parallel_completion_raises_done_state.scxml";
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

    ASSERT_TRUE(sm->isStateActive("a1")) << "the fixture is supposed to start inside the parallel; it did not, "
                                            "so nothing below is testing what it claims";
    ASSERT_TRUE(sm->isStateActive("b1")) << "the fixture is supposed to start inside the parallel; it did not, "
                                            "so nothing below is testing what it claims";

    // Queued mode (see setImmediateMode(false) above), so the event sits on the
    // external queue until the drain runs — the same macrostep boundary the
    // engine uses for its own sends.
    ASSERT_TRUE(sm->raiseExternalEvent("go", ""));
    eventRaiser->processQueuedEvents();

    EXPECT_TRUE(sm->isStateActive("a2")) << "a region did not reach its `<final>` on `go`";
    EXPECT_TRUE(sm->isStateActive("b2")) << "a region did not reach its `<final>` on `go`";
}

}  // namespace Tests
}  // namespace SCE
