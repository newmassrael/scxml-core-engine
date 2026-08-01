// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.5 + 6.3.1: `<donedata>` survives a late completion — Interpreter path.
//
// The sibling `DonedataLocalInvokeTest.cpp` pins the payload shapes on a child
// whose initial configuration is already its top-level `<final>`. That child is
// done before its first macrostep, so the lift and the raise sit in the same
// call and the fixture cannot see a child that finishes later.
//
// §6.3.1 raises `done.invoke.<id>` whenever the child reaches a final state,
// and §5.5 puts that final state's `<donedata>` on the event. Neither sentence
// is scoped to a child that finalises during start-up, so an engine that lifts
// the stash only there satisfies the sibling and still hands the parent an
// empty `_event.data` for every child that answers an event first — which is
// what an invoked session normally does.
//
// Here the child opens the exchange with `ready`, the parent answers over
// `<send target="#_inv_late">`, and the child reaches `settled` two macrosteps
// in. The payload and the guard are copied from the sibling's `inv_param`
// phase, so a shape the sibling already proves green cannot be what fails
// here — only the timing differs.
//
// Fixture: integration_resources/donedata_late_completion/donedata_late_completion.scxml

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

class DonedataLateCompletionTest : public ::testing::Test {
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

TEST_F(DonedataLateCompletionTest, DonedataRidesACompletionAfterTheInvokeStarted) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/donedata_late_completion/donedata_late_completion.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();

    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());

    // §scxml-6.2: the parent's `<send target="#_inv_late">` and the child's
    // `<send target="#_parent">` both need the dispatcher chain
    // (scheduler -> target factory -> dispatcher).
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

    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (std::chrono::steady_clock::now() < deadline) {
        if (!sm->isRunning()) {
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    ASSERT_FALSE(sm->isRunning()) << "parent did not halt within 5s — it never saw `done.invoke.inv_late` at all, "
                                  << "so the child was not driven to its `<final>`";

    EXPECT_EQ(sm->getCurrentState(), "pass")
        << "the parent's `done.invoke.inv_late` guard did not see `_event.data.result === 42`, so the "
        << "child's `<donedata>` was dropped on a completion that happened after the invoke was "
        << "started. W3C SCXML 6.3.1 raises `done.invoke.<id>` wherever the child reaches its final "
        << "state and 5.5 puts that state's donedata on the event; neither is scoped to children that "
        << "finalise during start-up.";
}

}  // namespace Tests
}  // namespace SCE
