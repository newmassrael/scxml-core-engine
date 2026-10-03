// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The name an event arrives under (§scxml-5.10, §scxml-3.12.1) — Interpreter
// channel.
//
// A transition on `request` takes `request.new` by whole-token matching, and
// `_event.name` is then the name the event was sent under, not the descriptor it
// was matched through. The public IRP suite never reads a name the document does
// not write, so a machine that is told the shorter one passes all of it.
//
// Fixture:
// integration_resources/an_event_keeps_the_name_it_was_sent_under/an_event_keeps_the_name_it_was_sent_under.scxml — the
// canonical source all seven channels compile, so the Interpreter and AOT channels are held to the same machine as the
// Rust / Go / Kotlin / Python / C11 channels rather than to hand-kept copies.

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

class AnEventKeepsTheNameItWasSentUnderTest : public ::testing::Test {
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

TEST_F(AnEventKeepsTheNameItWasSentUnderTest, ANameTheDocumentDoesNotWriteIsToldWhole) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/an_event_keeps_the_name_it_was_sent_under/"
                                "an_event_keeps_the_name_it_was_sent_under.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();

    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());

    // §scxml-6.2: `<send target="#_listener">` needs the dispatcher chain
    // (scheduler → target factory → dispatcher), as the autoforward sibling's
    // child does for `#_parent`.
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

    // §scxml-3.13: reaching a top-level `<final>` halts processing, so the state
    // name freezes on `pass` or `fail`.
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (std::chrono::steady_clock::now() < deadline) {
        if (!sm->isRunning()) {
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    ASSERT_FALSE(sm->isRunning()) << "parent did not halt within 5s — the child never heard `request.new`";

    EXPECT_EQ(sm->terminalState().value_or(""), "pass")
        << "the child reported `arrivedShortened`: `request.new` took the transition on `request` "
        << "but `_event.name` told the child `request`. W3C §5.10 makes the name the one the event "
        << "was sent under, and §3.12.1 only decides which transition takes it.";
}

}  // namespace Tests
}  // namespace SCE
