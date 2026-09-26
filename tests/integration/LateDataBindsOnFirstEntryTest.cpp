// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.3 / Appendix D enterStates, late binding: a state's <data> is
// bound on that state's FIRST entry and never again — Interpreter path.
//
// `s` is entered, its `v` changed to 5, `s` left and entered again; its
// <onentry> records the `v` it sees each time. Measured 2026-09-26, this
// channel already kept the first-entry rule, but assigned only `<data>` with
// an `expr` — the fixture's `c`, bound from inline content, stayed unassigned.
//
// Sibling of `LateDataBindsOnFirstEntryAotTest.cpp` (C++ AOT).
//
// Fixture:
// integration_resources/late_data_binds_on_first_entry/late_data_binds_on_first_entry.scxml

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

class LateDataBindsOnFirstEntryTest : public ::testing::Test {
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

    /// Whether an expression over those records holds. Compared in the
    /// datamodel rather than as text, which would depend on whether the engine
    /// hands a number back as an integer or a double.
    bool holds(const char *expression) const {
        return read(expression) == "true";
    }

    std::string describe() const {
        return "entries=" + read("entries") + " seen=" + read("seen") + " contentSeen=" + read("contentSeen") +
               " v=" + read("v") + " (wanted 2 / 15 / 7 / 5)";
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
};

TEST_F(LateDataBindsOnFirstEntryTest, AStateBindsItsDataOnlyOnItsFirstEntry) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) + "/integration_resources/late_data_binds_on_first_entry/"
                                                                "late_data_binds_on_first_entry.scxml";
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
    ASSERT_TRUE(sm_->isStateActive("idle")) << "the run has to start in `idle`";

    // Enter `s`, change its `v`, leave it, enter it again — one event at a
    // time, since each is a separate step of the scenario.
    for (const char *event : {"go", "bump", "back", "go"}) {
        ASSERT_TRUE(sm_->raiseExternalEvent(event, ""));
        eventRaiser->processQueuedEvents();
    }
    ASSERT_TRUE(sm_->isStateActive("s")) << "the second `go` has to leave the machine in `s`. " << describe();

    EXPECT_TRUE(holds("entries === 2")) << "both entries of `s` must have run. " << describe();
    EXPECT_TRUE(holds("seen === 15"))
        << "`s` saw v=1 on its first entry and must see the 5 it was changed to on its second: 11 is a "
           "processor that binds late data on every entry, and no value at all one that never binds it. "
        << describe();
    EXPECT_TRUE(holds("contentSeen == 7"))
        << "`c` is bound from inline content, not an expr, and must be bound too. " << describe();
}

}  // namespace Tests
}  // namespace SCE
