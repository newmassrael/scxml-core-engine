// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4 autoforward field preservation — Interpreter local-invoke path.
//
// W3C §6.4 requires an exact copy of the parent's external event to reach
// every `<invoke autoforward="true">` child. The public IRP suite leaves
// this unchecked: test229 only asserts the event name crosses, and test230
// is a manual test whose field comparison is performed by a human reading
// two log dumps. A forwarded copy that arrives stripped of `_event.data`,
// `_event.origin` and `_event.invokeid` therefore passes both.
//
// Fixture: integration_resources/autoforward_event_fields/autoforward_event_fields.scxml
// — the canonical source all six backends compile, so the C++ Interpreter and
// AOT channels are held to the same machine as the Rust / Go / Kotlin / Python
// / C11 channels rather than to hand-kept copies.

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

class AutoforwardEventFieldsTest : public ::testing::Test {
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

TEST_F(AutoforwardEventFieldsTest, ForwardedCopyKeepsDataOriginAndInvokeid) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/autoforward_event_fields/autoforward_event_fields.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();

    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());

    // §scxml-6.2: `<send target="#_parent">` needs the dispatcher chain
    // (scheduler → target factory → dispatcher). The donedata sibling gets
    // by without it because `<donedata>` rides the invoke completion
    // callback; this fixture's child machine actually sends.
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
    // state name freezes on `pass` or `fail`. Poll `isRunning()` for the
    // same reason the donedata sibling does (`isInFinalState()`
    // short-circuits on the halted flag).
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (std::chrono::steady_clock::now() < deadline) {
        if (!sm->isRunning()) {
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    ASSERT_FALSE(sm->isRunning()) << "parent did not halt within 5s — the child never received the "
                                  << "forwarded `childToParent`, so no done.invoke.inv_echo was emitted";

    EXPECT_EQ(sm->getCurrentState(), "pass")
        << "the child reported `stripped`: the autoforwarded copy of `childToParent` "
        << "lost `_event.data.value`, `_event.origin` or `_event.invokeid`. W3C §6.4 "
        << "requires an exact copy — forward the source event's metadata rather than "
        << "its name alone.";
}

}  // namespace Tests
}  // namespace SCE
