// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.4.3: the value a `srcexpr` computes is the child that runs.
//
// This engine parses at run time, so it has always honoured the value: it
// loads the document the expression names and runs it. `sce:candidates`
// says nothing to it — the declaration exists for targets that must know
// the set at build time (docs/SCE_ACCEPTED_SUBSET.md §2.13), and this
// channel ignores it.
//
// That is exactly why the case belongs here as well. The fixture is the
// same document the six AOT channels compile, and this run is the statement
// of what they are being held to: not a shape invented for the generator,
// but what an engine that can read the document at run time already does.
// When the two disagree it is the generated side that is wrong, and a
// fixture driven only there could not say so.
//
// Fixture: integration_resources/invoke_candidate_selects_the_child/invoke_candidate_selects_the_child.scxml

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>
#include <string>
#include <thread>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

TEST(InvokeCandidateSelectsTheChildTest, TheEvaluatedValueSelectsWhichCandidateRuns) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/invoke_candidate_selects_the_child/"
                                "invoke_candidate_selects_the_child.scxml";

    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());

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
    sm->setEventRaiser(eventRaiser);
    sm->setEventDispatcher(
        std::make_shared<EventDispatcherImpl>(scheduler, std::make_shared<EventTargetFactoryImpl>(eventRaiser)));

    // ⚠ From the FILE, not from its text. This engine resolves a `src` the
    // document names against the document's own location, so handing it the
    // bytes with `loadSCXMLFromString` throws that location away and every
    // relative child becomes unreachable. Measured: the first draft did
    // exactly that and the run reported `<invoke> could not load
    // 'file:chosen.scxml'` — a defect in the driver, reported as though the
    // engine had chosen wrongly.
    ASSERT_TRUE(sm->loadSCXML(fixture)) << "canonical fixture not loadable: " << fixture;
    ASSERT_TRUE(sm->start());

    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (std::chrono::steady_clock::now() < deadline) {
        if (!sm->isRunning()) {
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    EXPECT_EQ(sm->getCurrentState(), "pass")
        << "this engine loads the document the value names: `wrongChild` means it loaded "
           "the other one, `noChild` means it could not load at all, and resting in "
           "`probe` means it started nothing — and whichever it is, the clause the AOT "
           "channels are held to is not what this one does.";
}

}  // namespace Tests
}  // namespace SCE
