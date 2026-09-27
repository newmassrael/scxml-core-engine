// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 4.9 + 4.6: an error raised inside a <foreach> body ends the
// block that contains the <foreach>, and it is the only error raised —
// Interpreter path.
//
// Sibling of `AnErrorInsideAForeachEndsItsBlockAotTest.cpp` (C++ AOT).
//
// Fixture:
// integration_resources/an_error_inside_a_foreach_ends_its_block/an_error_inside_a_foreach_ends_its_block.scxml

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
#include <utility>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

class AnErrorInsideAForeachEndsItsBlockTest : public ::testing::Test {
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

TEST_F(AnErrorInsideAForeachEndsItsBlockTest, TheErrorEndsTheBlockAndIsTheOnlyOne) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/an_error_inside_a_foreach_ends_its_block/"
                                "an_error_inside_a_foreach_ends_its_block.scxml";
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
    for (const char *event : {"go", "t", "finish"}) {
        ASSERT_TRUE(sm_->raiseExternalEvent(event, ""));
        eventRaiser->processQueuedEvents();
    }

    EXPECT_EQ(sm_->terminalState().value_or(""), "done") << "`finish` must carry the run to `done`";
    const std::vector<std::pair<std::string, int>> expected = {
        {"iters1", 1}, {"after1", 0}, {"iters2", 1}, {"after2", 0},
        {"iters3", 1}, {"after3", 0}, {"errors", 3}, {"sent", 2},
    };
    for (const auto &[name, value] : expected) {
        EXPECT_EQ(read(name + " === " + std::to_string(value)), "true")
            << name << " = " << read(name) << ", want " << value;
    }
}

}  // namespace Tests
}  // namespace SCE
