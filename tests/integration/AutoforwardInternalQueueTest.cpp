// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4 autoforward skips internal-queue events — Interpreter path.
//
// Appendix D's `mainEventLoop` forwards only what it dequeues from the
// external queue; the internal drain above it has no forwarding step at
// all. §6.2 raises `error.execution` onto the internal queue when `<send>`
// names an unsupported type, so it must never reach an `autoforward`
// child — and it must be excluded by where it was raised, not by a filter
// that recognises its name.
//
// This is the half of the contract a name filter cannot express. An engine
// that routes platform events onto the external queue for an unrelated
// reason — keeping them from being delivered inline, say — satisfies every
// name-blind forwarding rule and still leaks them to children.
//
// Sibling of `AutoforwardDoneInvokeTest.cpp`, which pins the positive half:
// one fails if `done.invoke` is withheld, the other if `error.execution`
// leaks. Together they leave no room for a name-based filter.
//
// Fixture: integration_resources/autoforward_internal_queue/autoforward_internal_queue.scxml

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

class AutoforwardInternalQueueTest : public ::testing::Test {
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

TEST_F(AutoforwardInternalQueueTest, AnInternalQueueEventIsNeverAutoforwarded) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/autoforward_internal_queue/autoforward_internal_queue.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();

    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());

    // §scxml-6.2: the child's `<send target="#_parent">` and the parent's
    // targetless `<send event="probe"/>` both need the dispatcher chain
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

    ASSERT_FALSE(sm->isRunning()) << "parent did not halt within 5s — the watcher child reported neither "
                                  << "verdict, so neither `error.execution` nor `probe` reached it";

    EXPECT_EQ(sm->getCurrentState(), "pass")
        << "the watcher saw `error.execution`: an internal-queue event was autoforwarded. "
        << "W3C Appendix D `mainEventLoop` forwards only what it dequeues from the external "
        << "queue, and §6.2 raises `error.execution` onto the internal one — check that the "
        << "event was not routed onto the external queue for some unrelated reason (keeping "
        << "it from inline delivery, say), which would leak it past any name-blind forward.";
}

}  // namespace Tests
}  // namespace SCE
