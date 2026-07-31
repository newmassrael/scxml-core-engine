// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4 autoforward carries `done.invoke.<id>` — Interpreter path.
//
// Appendix D's `mainEventLoop` forwards every event it dequeues from the
// external queue to each `autoforward` child without testing the event's
// name; the sole exclusion is the cancel event, expressed as control flow
// (`continue`). §6.4.2 places `done.invoke.<id>` on the external queue of
// the invoking session — "the external service ... MUST return a special
// event 'done.invoke.id' to the external event queue of the invoking
// process" — so a sibling child that is still running must receive it.
// `error.*` and `done.state.*` stay out of the forwarded set because they
// belong to the internal queue, not because of how they are spelled.
//
// The IRP suite cannot see this: test229 checks only that a name crosses,
// test230 is a manual test, and neither runs two concurrent invokes.
//
// Fixture: integration_resources/autoforward_done_invoke/autoforward_done_invoke.scxml
// — the canonical source all seven channels compile.

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

class AutoforwardDoneInvokeTest : public ::testing::Test {
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

TEST_F(AutoforwardDoneInvokeTest, DoneInvokeFromASiblingReachesTheAutoforwardChild) {
    const std::string fixture =
        std::string(SCE_PROJECT_ROOT) + "/integration_resources/autoforward_done_invoke/autoforward_done_invoke.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();

    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());

    // §scxml-6.2: `<send target="#_parent">` and the parent's targetless
    // `<send event="probe"/>` both need the dispatcher chain
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

    // §scxml-3.13: reaching a top-level `<final>` halts processing, so the
    // state name freezes on `pass` or `fail`.
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (std::chrono::steady_clock::now() < deadline) {
        if (!sm->isRunning()) {
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    ASSERT_FALSE(sm->isRunning()) << "parent did not halt within 5s — the watcher child reported neither "
                                  << "verdict, so `done.invoke.inv_short` never reached the parent's "
                                  << "external queue at all";

    EXPECT_EQ(sm->getCurrentState(), "pass")
        << "the watcher saw only `probe`: `done.invoke.inv_short` was withheld from a live "
        << "`autoforward` child. W3C Appendix D `mainEventLoop` forwards every event dequeued "
        << "from the external queue and excludes only the cancel event, and §6.4.2 places "
        << "`done.invoke.<id>` on that queue — so no name-based platform-event filter belongs "
        << "on the forwarding path.";
}

}  // namespace Tests
}  // namespace SCE
