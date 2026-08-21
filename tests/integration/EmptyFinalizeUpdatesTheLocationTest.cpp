// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.5.2: what an EMPTY `<finalize>` does, and what an absent one does
// not — Interpreter path.
//
// With no executable content the Processor "MUST update the data model each
// time an event is received from the child process ... for each item in the
// 'namelist' attribute and each such `<param>` element ... as if by
// `<assign>` with any return value that has a name that matches", and then:
// "Note that the automatic update does not take place if the `<finalize>`
// element is absent as opposed to empty."
//
// The corpus holds two `<finalize>` documents (W3C 233/234) and zero empty
// ones, so the automatic update had no witness anywhere. Measured 2026-08-22
// it had no implementation either: this engine gates on
// `finalizeScript.empty()`, which makes an empty element and a missing one the
// same thing.
//
// Sibling of `EmptyFinalizeUpdatesTheLocationAotTest.cpp`.
//
// Fixture: integration_resources/empty_finalize_updates_the_location/empty_finalize_updates_the_location.scxml

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

class EmptyFinalizeUpdatesTheLocationTest : public ::testing::Test {
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

TEST_F(EmptyFinalizeUpdatesTheLocationTest, AnEmptyFinalizeUpdatesTheLocationAndAnAbsentOneDoesNot) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/empty_finalize_updates_the_location/"
                                "empty_finalize_updates_the_location.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();

    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());

    // Each child answers with `<send target="#_parent">` and each phase's own
    // verdict rides a delayed `<send>`, so the dispatcher chain is required.
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

    ASSERT_TRUE(sm->loadSCXMLFromString(buffer.str()));
    ASSERT_TRUE(sm->start());

    // Two 3s timeouts back to back, so the budget has to outlast both or a
    // silent child reads as a hang rather than as its own verdict state.
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(20);
    while (std::chrono::steady_clock::now() < deadline) {
        if (!sm->isRunning()) {
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    ASSERT_FALSE(sm->isRunning()) << "parent did not halt within 20s — neither child answered and "
                                  << "neither delayed timeout fired";

    const std::string reached = sm->getCurrentState();
    EXPECT_NE(reached, "failNotUpdated")
        << "the empty `<finalize/>` left `tally` at its old value: §scxml-6.5.2 makes an empty "
        << "element mean the automatic update — for each `namelist` item the Processor updates the "
        << "location as if by `<assign>` with the matching return value.";
    EXPECT_NE(reached, "failUpdatedWithoutFinalize")
        << "`guard` moved with no `<finalize>` element at all: the clause's note is a prohibition — "
        << "\"the automatic update does not take place if the <finalize> element is absent as "
        << "opposed to empty\".";
    EXPECT_NE(reached, "failUnmatchedNameWrote")
        << "an event carrying no matching name still wrote `keeper`: §scxml-6.5.2 says \"with ANY "
        << "return value that has a name that matches\", so an unconditional write blanks the "
        << "parent's data model on every unrelated answer the child sends.";
    EXPECT_NE(reached, "failUnmatchedChildSilent")
        << "the third child never answered, so the guarded-write half was never exercised.";
    EXPECT_NE(reached, "failEmptyChildSilent")
        << "the first child never answered, so the empty-`<finalize>` half was never exercised.";
    EXPECT_NE(reached, "failAbsentChildSilent")
        << "the second child never answered, so the absent-`<finalize>` half was never exercised.";
    EXPECT_EQ(reached, "pass");
}

}  // namespace Tests
}  // namespace SCE
