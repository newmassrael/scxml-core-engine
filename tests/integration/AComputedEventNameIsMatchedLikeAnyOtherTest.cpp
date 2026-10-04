// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A computed event name is matched like any other (§scxml-3.12.1, §scxml-5.10) —
// Interpreter channel.
//
// A `<send eventexpr>` names its event at run time, so the document cannot have
// written the name: a descriptor of `request` takes `request.new`, `*` takes a
// name nothing else of the document takes, and `_event.name` is the whole name.
// The document sends six, over the external and the internal queue, now and
// after a delay, and takes each only when it is told the whole name.
//
// Fixture:
// integration_resources/a_computed_event_name_is_matched_like_any_other/a_computed_event_name_is_matched_like_any_other.scxml
// — the canonical source all seven channels compile, so the Interpreter and AOT
// channels are held to the same machine as the Rust / Go / Kotlin / Python / C11
// channels rather than to hand-kept copies.

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

class AComputedEventNameIsMatchedLikeAnyOtherTest : public ::testing::Test {
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

TEST_F(AComputedEventNameIsMatchedLikeAnyOtherTest, AComputedNameIsMatchedByTokenAndToldWhole) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/a_computed_event_name_is_matched_like_any_other/"
                                "a_computed_event_name_is_matched_like_any_other.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();

    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());

    // §scxml-6.2: a delayed `<send>` needs the dispatcher chain
    // (scheduler → target factory → dispatcher).
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
    // name freezes on `pass`.
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (std::chrono::steady_clock::now() < deadline) {
        if (!sm->isRunning()) {
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    ASSERT_FALSE(sm->isRunning()) << "the machine did not halt within 5s — a computed name was dropped, or "
                                     "matched but told shorter than the name it was sent under";

    EXPECT_EQ(sm->terminalState().value_or(""), "pass");
}

}  // namespace Tests
}  // namespace SCE
