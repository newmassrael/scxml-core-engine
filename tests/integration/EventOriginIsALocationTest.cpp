// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML C.1 `_event.origin` is an address — Interpreter local-invoke path.
//
// Appendix C.1 requires the origin of a delivered event to match the
// location the sending session published in its `_ioprocessors`, and that
// location to be a usable `<send>` target. The public IRP suite checks
// neither half across sessions: test336 and test350 both send to the
// session they already are, so any value at all round-trips.
//
// Fixture: integration_resources/event_origin_is_a_location/event_origin_is_a_location.scxml
// — the canonical source every backend compiles, so the C++ Interpreter and
// AOT channels answer the same question as the Rust / Go / Kotlin / Python /
// C11 channels rather than hand-kept copies of it.

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

class EventOriginIsALocationTest : public ::testing::Test {
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

TEST_F(EventOriginIsALocationTest, OriginIsTheSendersPublishedLocationAndRoutesBack) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/event_origin_is_a_location/event_origin_is_a_location.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();

    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());

    // §scxml-6.2: both directions of this fixture are `<send>` — the child
    // to `#_parent`, the parent to the address it just received — so the
    // dispatcher chain (scheduler -> target factory -> dispatcher) is what
    // makes the round trip possible at all.
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

    // A routing violation cannot raise an event to fail on — a target that
    // resolves nowhere simply delivers nothing — so it shows up here, as
    // the parent still waiting in `await_reply`.
    ASSERT_FALSE(sm->isRunning()) << "parent never halted: it accepted `_event.origin` as an address and sent "
                                  << "`reply` to it, and nothing came back. §scxml-C-1 requires the published "
                                  << "location to be a usable <send> target; current state is "
                                  << sm->getCurrentState();

    EXPECT_EQ(sm->getCurrentState(), "pass")
        << "`_event.origin` did not carry the sender's published `_ioprocessors` location. "
        << "§scxml-C-1 requires the origin to match that location, which is what makes it "
        << "an address a peer can answer; a bare session id matches nothing the sender "
        << "published. Current state is " << sm->getCurrentState();
}

}  // namespace Tests
}  // namespace SCE
