// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 6.4 + C.1: a delay postpones a <send>, it does not change
// where the send goes — C++ Interpreter.
//
// The test owns the clock. The fixture reads `early` (10ms) before `inner`
// (20ms), and both are sent on entry to the first state, before the two invoked
// children have started. On the wall clock both fall due while the children are
// still starting whenever the host is busy, and the main loop then takes the
// internal `inner` before the external `early` (W3C SCXML 3.13), so `order`
// reads 13 for a reason that has nothing to do with routing. Measured on an
// 8-core host with ten spinning processes: 13 of 20 runs. The other channels
// step a virtual clock; the scheduler's MANUAL mode is that clock here, and
// `forcePoll()` is the step. The two invoked children share that scheduler, so
// they run on the same logical clock as their parent.
//
// Fixture:
// integration_resources/a_delayed_send_reaches_what_its_target_names/a_delayed_send_reaches_what_its_target_names.scxml

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>
#include <string>
#include <thread>
#include <utility>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

class ADelayedSendReachesWhatItsTargetNamesTest : public ::testing::Test {
protected:
    /// What the children's threads get between two steps of the clock. Which event is next
    /// is the clock's to say; the pause only lets a child report to its parent.
    static constexpr std::chrono::milliseconds kSettle{20};

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

TEST_F(ADelayedSendReachesWhatItsTargetNamesTest, ADelayedSendReachesWhatItsTargetNames) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/a_delayed_send_reaches_what_its_target_names/"
                                "a_delayed_send_reaches_what_its_target_names.scxml";

    sm_ = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());
    auto scheduler = std::make_shared<EventSchedulerImpl>(
        [](const EventDescriptor &event, std::shared_ptr<IEventTarget> target, const std::string &) -> bool {
            try {
                return target->send(event).get().isSuccess;
            } catch (...) {
                return false;
            }
        });
    scheduler->setMode(SchedulerMode::MANUAL);
    auto eventRaiser = std::make_shared<EventRaiserImpl>();
    eventRaiser->setScheduler(scheduler);
    eventRaiser->setImmediateMode(false);
    sm_->setEventRaiser(eventRaiser);
    sm_->setEventDispatcher(
        std::make_shared<EventDispatcherImpl>(scheduler, std::make_shared<EventTargetFactoryImpl>(eventRaiser)));

    ASSERT_TRUE(sm_->loadSCXML(fixture)) << "canonical fixture not loadable: " << fixture;
    ASSERT_TRUE(sm_->start());
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (std::chrono::steady_clock::now() < deadline && sm_->isRunning()) {
        std::this_thread::sleep_for(kSettle);
        eventRaiser->processQueuedEvents();
        scheduler->forcePoll();
        eventRaiser->processQueuedEvents();
    }

    EXPECT_EQ(sm_->terminalState().value_or(""), "done") << "the run must end in `done`";
    const std::vector<std::pair<std::string, int>> expected = {
        {"order", 31}, {"innerInternal", 1}, {"lateOk", 1},      {"lateCount", 1},
        {"pongOk", 1}, {"commErrors", 2},    {"lostArrived", 0}, {"afterStranger", 0}};
    for (const auto &[name, value] : expected) {
        EXPECT_EQ(read(name + " === " + std::to_string(value)), "true")
            << name << " = " << read(name) << ", want " << value;
    }
}

}  // namespace Tests
}  // namespace SCE
