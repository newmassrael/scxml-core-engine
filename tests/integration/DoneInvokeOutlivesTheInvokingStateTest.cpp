// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.4: the done.invoke of a child that ended on its own is delivered
// even when the invoking state is left before it is taken — Interpreter path.
//
// W3C SCXML Test 236 is the case: the child enters a top-level <final>, its
// exit handler sends `childToParent`, and done.invoke follows as the last
// event it generates. The parent takes `childToParent` first and leaves the
// state holding the <invoke>, which cancels the invocation — and then takes
// done.invoke in the state it moved to.
//
// This Interpreter's cancel discarded both what a cancelled session sends
// from then on and what it had already sent (W3C SCXML Test 252 is about the
// first). For a session that had already ended, the second half threw away
// its done.invoke. It went unnoticed while the raiser delivered queued
// events: it took the whole queue into a local list before dispatching any of
// it, so the purge found nothing left to remove, and it handed each event over
// without its origin session, so the filter could not recognise it. Once the
// main event loop took events one at a time, origin included, the purge
// matched done.invoke and Test 236 failed on every channel built on this
// engine. The cancel now discards nothing from a session that is no longer
// running: there is nothing left to stop, and what it sent before it ended
// stays the parent's.
//
// No delay decides the verdict: `timeout` only bounds the wait, and every
// fail state names the order that reached it.

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <thread>

namespace SCE {
namespace Tests {

namespace {

constexpr const char *DOCUMENT = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="ecmascript" initial="invoking">

  <state id="invoking">
    <onentry>
      <send event="timeout" delay="1s"/>
    </onentry>
    <invoke id="child" type="http://www.w3.org/TR/scxml/">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
               datamodel="ecmascript" initial="ended">
          <final id="ended">
            <onexit>
              <send target="#_parent" event="childToParent"/>
            </onexit>
          </final>
        </scxml>
      </content>
    </invoke>
    <!-- Leaving this state cancels the invocation, after the child ended. -->
    <transition event="childToParent" target="left"/>
    <transition event="done.invoke" target="failDoneBeforeTheExitHandler"/>
    <transition event="timeout" target="failNothingFromTheChild"/>
  </state>

  <state id="left">
    <transition event="done.invoke.child" target="pass"/>
    <transition event="timeout" target="failDoneInvokeDiscarded"/>
    <transition event="*" target="failAnotherEvent"/>
  </state>

  <final id="pass"/>
  <final id="failDoneBeforeTheExitHandler"/>
  <final id="failNothingFromTheChild"/>
  <final id="failDoneInvokeDiscarded"/>
  <final id="failAnotherEvent"/>
</scxml>
)";

}  // namespace

class DoneInvokeOutlivesTheInvokingStateTest : public ::testing::Test {
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

TEST_F(DoneInvokeOutlivesTheInvokingStateTest, TheDoneInvokeOfAChildThatEndedFirstIsDelivered) {
    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());

    // §scxml-6.2: the child answers with `<send target="#_parent">` and the
    // parent bounds its wait with a delayed `<send>`, so the dispatcher chain
    // (scheduler → target factory → dispatcher) is wired the way
    // `InvokeParamSeedsDeclaredChildDataTest` wires it.
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

    ASSERT_TRUE(sm->loadSCXMLFromString(DOCUMENT));
    ASSERT_TRUE(sm->start());

    // §scxml-3.13: a top-level <final> halts processing, so the state name
    // freezes on the verdict.
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (sm->isRunning() && std::chrono::steady_clock::now() < deadline) {
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }
    ASSERT_FALSE(sm->isRunning()) << "the parent reached no verdict within 5s, not even its own timeout";

    const std::string reached = sm->terminalState().value_or("");
    EXPECT_NE(reached, "failDoneInvokeDiscarded")
        << "done.invoke never arrived after the parent left the invoking state: cancelling an "
        << "invocation whose session had already ended discarded the event that session sent last "
        << "(§scxml-6.4, W3C SCXML Test 236).";
    EXPECT_NE(reached, "failDoneBeforeTheExitHandler")
        << "done.invoke arrived ahead of the event the child's exit handler sent: §scxml-6.4 has the "
        << "exit handlers run first.";
    EXPECT_NE(reached, "failNothingFromTheChild") << "neither event the child sends ever reached the parent.";
    EXPECT_NE(reached, "failAnotherEvent") << "an event other than done.invoke arrived after `childToParent`.";
    EXPECT_EQ(reached, "pass");
}

}  // namespace Tests
}  // namespace SCE
