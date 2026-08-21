// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: autoforward is owed to the external event, not to the door it
// came through — Interpreter path.
//
// The four sibling `autoforward_*` stems all let the machine forward events it
// queued for itself, so every one of them drives the engine through its
// external drain. This one hands the machine an event from outside, through
// the host-facing `processEvent()`, and asks whether the `autoforward` child
// sees it. Appendix D's `mainEventLoop` binds the preliminary step
// (`applyFinalize` + the autoforward `send`) to the external event it is about
// to select transitions for; an engine with more than one way in has to run
// the step at each of them or the child goes blind to whatever the host
// delivers.
//
// Sibling of `HostEventReachesTheChildAotTest.cpp` (C++ AOT channel), which is
// where this defect was measured on 2026-08-21: the AOT engine had the step
// written inline in its queue drain, so its `processEvent()` skipped it.
// Both engines ship in production, so each is held to the position
// independently against one canonical fixture.
//
// Fixture: integration_resources/host_event_reaches_the_child/host_event_reaches_the_child.scxml

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <fstream>
#include <gtest/gtest.h>
#include <sstream>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

class HostEventReachesTheChildTest : public ::testing::Test {
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

TEST_F(HostEventReachesTheChildTest, AnEventTheHostHandsOverReachesTheAutoforwardChild) {
    const std::string fixture =
        std::string(SCE_PROJECT_ROOT) +
        "/integration_resources/host_event_reaches_the_child/host_event_reaches_the_child.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();

    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());

    // §scxml-6.2: the child's `<send target="#_parent">` and the parent's
    // `<send target="#_inv_probe">` both need the dispatcher chain
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

    // The child opens the exchange, so let its `ready` move the parent into
    // `armed` — the one state that can be handed an event from outside.
    // Draining rather than sleeping: the handshake is the machine's own work,
    // so a fixed wait would only make a broken fixture look like a slow one.
    for (int i = 0; i < 50 && sm->getCurrentState() != "armed" && sm->isRunning(); ++i) {
        if (!eventRaiser->hasQueuedEvents()) {
            break;
        }
        eventRaiser->processQueuedEvents();
    }
    ASSERT_EQ(sm->getCurrentState(), "armed")
        << "the probe child never sent `ready`, so the fixture never reached the state where a "
        << "host event can be handed over — this is a broken handshake, not a forwarding verdict";

    // The axis: the host's own entry point. This engine's other door is
    // `raiseExternalEvent` + `processQueuedEvents`, which is the drain the
    // preliminary step was written into; this one goes straight to transition
    // selection.
    sm->processEvent("hostPing");

    // The child's answer lands on the parent's external queue, and a host that
    // drives with `processEvent` owns the drain that takes it off again.
    // Bounded rather than timed: every pass here is the machine's own work, so
    // a queue that has not emptied after this many is not slow.
    for (int i = 0; i < 50 && sm->isRunning() && eventRaiser->hasQueuedEvents(); ++i) {
        eventRaiser->processQueuedEvents();
    }

    ASSERT_FALSE(sm->isRunning()) << "parent did not halt — the probe child answered neither "
                                  << "verdict, so neither `hostPing` nor `marker` reached it";

    EXPECT_EQ(sm->getCurrentState(), "pass")
        << "the probe child answered `sawMarkerOnly`, so the event the host handed to "
        << "`processEvent` was never forwarded to it: the child only ever saw the `marker` the "
        << "parent's own transition body sent. W3C Appendix D `mainEventLoop` runs the autoforward "
        << "`send` against the external event before it selects transitions for it, whichever door "
        << "the event arrived through — an engine that runs that step only in its queue drain "
        << "leaves an `autoforward` child blind to everything its host delivers.";
}

}  // namespace Tests
}  // namespace SCE
