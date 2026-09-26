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
// Two drivers, because this engine has two ways for an internal event to
// reach its transitions. In auto mode the main event loop takes it off the
// queue itself. In interactive mode the host steps it, and it arrives through
// the raiser's dispatch — the one place an internal event still enters this
// machine through a callback, so the one place that callback can lose the
// queue it came off.
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

        const std::string fixture =
            std::string(SCE_PROJECT_ROOT) +
            "/integration_resources/autoforward_internal_queue/autoforward_internal_queue.scxml";
        std::ifstream in(fixture);
        ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
        std::ostringstream buffer;
        buffer << in.rdbuf();

        sm_ = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());

        // §scxml-6.2: the child's `<send target="#_parent">` and the parent's
        // targetless `<send event="probe"/>` both need the dispatcher chain
        // (scheduler -> target factory -> dispatcher).
        scheduler_ = std::make_shared<EventSchedulerImpl>(
            [](const EventDescriptor &event, std::shared_ptr<IEventTarget> target, const std::string &) -> bool {
                try {
                    return target->send(event).get().isSuccess;
                } catch (...) {
                    return false;
                }
            });
        raiser_ = std::make_shared<EventRaiserImpl>();
        raiser_->setScheduler(scheduler_);
        raiser_->setImmediateMode(false);
        sm_->setEventRaiser(raiser_);
        sm_->setEventDispatcher(
            std::make_shared<EventDispatcherImpl>(scheduler_, std::make_shared<EventTargetFactoryImpl>(raiser_)));

        ASSERT_TRUE(sm_->loadSCXMLFromString(buffer.str()));
    }

    void TearDown() override {
        sm_.reset();
        if (scheduler_) {
            scheduler_->shutdown(true);
        }
        if (engine_) {
            engine_->shutdown();
        }
    }

    /// The verdict both drivers read. The watcher child reports `sawInternal`
    /// or `sawProbeOnly`, and each is a top-level `<final>` of the parent, so
    /// a halted parent has a verdict and a running one has none.
    void expectTheWatcherSawOnlyTheProbe() {
        ASSERT_FALSE(sm_->isRunning()) << "parent did not halt within 5s — the watcher child reported neither "
                                       << "verdict, so neither `error.execution` nor `probe` reached it";

        EXPECT_EQ(sm_->terminalState().value_or(""), "pass")
            << "the watcher saw `error.execution`: an internal-queue event was autoforwarded. "
            << "W3C Appendix D `mainEventLoop` forwards only what it dequeues from the external "
            << "queue, and §6.2 raises `error.execution` onto the internal one — check that the "
            << "event was not routed onto the external queue for some unrelated reason (keeping "
            << "it from inline delivery, say), which would leak it past any name-blind forward.";
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<EventSchedulerImpl> scheduler_;
    std::shared_ptr<EventRaiserImpl> raiser_;
    std::shared_ptr<StateMachine> sm_;
};

TEST_F(AutoforwardInternalQueueTest, AnInternalQueueEventIsNeverAutoforwarded) {
    ASSERT_TRUE(sm_->start());

    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (std::chrono::steady_clock::now() < deadline) {
        if (!sm_->isRunning()) {
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    expectTheWatcherSawOnlyTheProbe();
}

/// The same contract with the host stepping. Each event reaches the machine
/// through the raiser's dispatch, which carries the queue the event came off
/// into `processDispatchedEvent`. A callback that entered through the
/// host-facing `processEvent` instead would declare `error.execution`
/// external — every event a host hands over is — and forward it.
TEST_F(AutoforwardInternalQueueTest, AnInternalEventTheHostStepsIsNeverAutoforwarded) {
    ASSERT_TRUE(sm_->start(/*autoProcessQueuedEvents=*/false));

    // Bounded by a deadline rather than a step count: the child's replies go
    // through the dispatcher, and a step that finds the queue empty is a wait,
    // not a verdict.
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (sm_->isRunning() && std::chrono::steady_clock::now() < deadline) {
        if (!raiser_->processNextQueuedEvent()) {
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
        }
    }

    expectTheWatcherSawOnlyTheProbe();
}

}  // namespace Tests
}  // namespace SCE
