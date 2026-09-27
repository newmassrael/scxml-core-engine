// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2: a <send> delay is read as one CSS2 time — Interpreter path.
//
// Sibling of `ADelayIsACss2TimeAotTest.cpp` (C++ AOT).
//
// Fixture: integration_resources/a_delay_is_a_css2_time/a_delay_is_a_css2_time.scxml

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <fstream>
#include <gtest/gtest.h>
#include <memory>
#include <sstream>
#include <string>
#include <thread>
#include <utility>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

class ADelayIsACss2TimeTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();
    }

    void TearDown() override {
        sm_.reset();
        if (engine_) {
            engine_->shutdown();
        }
    }

    /// A value the handlers recorded (W3C SCXML 5.3), for the failure message.
    std::string read(const std::string &name) const {
        auto result = ScriptEngineProvider::getScriptEngine().evaluateExpression(sm_->getSessionId(), name).get();
        return result.isSuccess() ? result.getValueAsString() : std::string("<unreadable>");
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
};

// The Interpreter's scheduler runs on the wall clock, so the run is polled
// until `a` (570 ms) carries it to `done`.
TEST_F(ADelayIsACss2TimeTest, EachDelayIsReadAsOneCss2Time) {
    const std::string fixture =
        std::string(SCE_PROJECT_ROOT) + "/integration_resources/a_delay_is_a_css2_time/a_delay_is_a_css2_time.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();

    sm_ = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());
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
    sm_->setEventRaiser(eventRaiser);
    sm_->setEventDispatcher(
        std::make_shared<EventDispatcherImpl>(scheduler, std::make_shared<EventTargetFactoryImpl>(eventRaiser)));

    ASSERT_TRUE(sm_->loadSCXMLFromString(buffer.str()));
    ASSERT_TRUE(sm_->start());
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (std::chrono::steady_clock::now() < deadline && sm_->isRunning()) {
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
        eventRaiser->processQueuedEvents();
    }

    EXPECT_EQ(sm_->terminalState().value_or(""), "done") << "`a` must carry the run to `done`";
    const std::vector<std::pair<std::string, int>> expected = {{"errors", 2}, {"after", 0}, {"bad", 0}, {"aAfterB", 1}};
    for (const auto &[name, value] : expected) {
        EXPECT_EQ(read(name + " === " + std::to_string(value)), "true")
            << name << " = " << read(name) << ", want " << value;
    }
}

}  // namespace Tests
}  // namespace SCE
