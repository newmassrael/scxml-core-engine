// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10 `_sessionid` is the id of a session - Interpreter local-invoke path.
//
// The clause binds `_sessionid` to the system-generated id for the current
// session, and Appendix C.1.1 derives the address a session publishes from
// that id, so two live sessions holding one id publish one address and a
// `<send>` to either reaches both. Every test in the public IRP suite that
// reaches `_sessionid` runs a single session, so none of them can ask.
//
// Fixture: integration_resources/session_ids_are_distinct/session_ids_are_distinct.scxml
// - the canonical source every backend compiles, so the C++ Interpreter and
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

class SessionIdsAreDistinctTest : public ::testing::Test {
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

TEST_F(SessionIdsAreDistinctTest, TwoLiveSessionsAreIssuedDifferentIds) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/session_ids_are_distinct/session_ids_are_distinct.scxml";
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
    ASSERT_FALSE(sm->isRunning()) << "parent never halted: only one child reported its `_sessionid`, so the "
                                  << "two ids were never compared. Current state is " << sm->getCurrentState();

    EXPECT_EQ(sm->getCurrentState(), "pass")
        << "two live sessions reported the same `_sessionid`. The clause binds it to the id of "
        << "the current session, and the published `_ioprocessors` location is derived from it, "
        << "so one id for two sessions is one address for two sessions. Current state is " << sm->getCurrentState();
}

}  // namespace Tests
}  // namespace SCE
