// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.7.1 + 4.9: a <send> whose <param> cannot be read still sends
// its message without that pair, and the error ends its block; a valid
// location param is sent — Interpreter path.
//
// Measured 2026-09-27, this channel already dropped the pair and sent the
// message, but let the rest of the block run.
//
// Sibling of `ABadSendParamEndsItsBlockAotTest.cpp` (C++ AOT).
//
// Fixture:
// integration_resources/a_bad_send_param_ends_its_block/a_bad_send_param_ends_its_block.scxml

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <fstream>
#include <gtest/gtest.h>
#include <memory>
#include <sstream>
#include <string>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

class ABadSendParamEndsItsBlockTest : public ::testing::Test {
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
    std::string read(const char *name) const {
        auto result = ScriptEngineProvider::getScriptEngine().evaluateExpression(sm_->getSessionId(), name).get();
        return result.isSuccess() ? result.getValueAsString() : std::string("<unreadable>");
    }

    std::string describe() const {
        return "ended in " + sm_->terminalState().value_or("<none>") + ", errors=" + read("errors") +
               " partials=" + read("partials") + " bares=" + read("bares") + " after=" + read("after") +
               " carried=" + read("carried") + " (wanted done / 3 / 2 / 1 / 0 / 1)";
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
};

TEST_F(ABadSendParamEndsItsBlockTest, TheMessageGoesAndTheBlockStops) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/a_bad_send_param_ends_its_block/"
                                "a_bad_send_param_ends_its_block.scxml";
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
    eventRaiser->processQueuedEvents();
    ASSERT_TRUE(sm_->raiseExternalEvent("finish", ""));
    eventRaiser->processQueuedEvents();

    EXPECT_EQ(sm_->terminalState().value_or(""), "done") << "`finish` must carry the run to `done`. " << describe();
    EXPECT_EQ(read("errors === 3"), "true") << "each unreadable <param> raises one error.execution. " << describe();
    EXPECT_EQ(read("partials === 2"), "true")
        << "both internal sends go, carrying the good pair without the bad one. " << describe();
    EXPECT_EQ(read("bares === 1"), "true") << "the external send goes with its empty pair left out. " << describe();
    EXPECT_EQ(read("after === 0"), "true")
        << "the <param> error ends the block, so nothing after the <send> runs. " << describe();
    EXPECT_EQ(read("carried === 1"), "true") << "a valid location param is sent with its value. " << describe();
}

}  // namespace Tests
}  // namespace SCE
