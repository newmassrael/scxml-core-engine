// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.4 and B.2: `<assign location="v" expr="_event.data"/>` leaves
// `v` holding the DOM a `<send>`'s XML `<content>` arrived as — Interpreter
// path.
//
// Read 2026-11-06: the Interpreter's `<assign>` reached
// `AssignmentExecutionHelper`, which evaluated the expression of a bare-name
// location into a `ScriptValue` and wrote it back with `setVariable`. A
// `ScriptValue` has no kind for a DOM, so `v` held an empty object while the
// member path `holder.doc`, assigned inside the engine, kept the document.
// The engine now runs every assignment itself (`IScriptEngine::assign`).
//
// Sibling of `AnAssignmentKeepsTheDocumentItIsGivenAotTest.cpp`.
//
// Fixture:
// integration_resources/an_assignment_keeps_the_document_it_is_given/an_assignment_keeps_the_document_it_is_given.scxml
// (canonical, shared with every channel).

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

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

class AnAssignmentKeepsTheDocumentItIsGivenTest : public ::testing::Test {
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

    /// An expression evaluated in the session, as a string, for the failure
    /// message.
    std::string read(const char *expression) const {
        auto result = ScriptEngineProvider::getScriptEngine().evaluateExpression(sm_->getSessionId(), expression).get();
        return result.isSuccess() ? result.getValueAsString() : std::string("<unreadable>");
    }

    std::string describe() const {
        return "ended in " + sm_->terminalState().value_or("<running>") + ", v.nodeType=" + read("v && v.nodeType") +
               " holder.doc.nodeType=" + read("holder.doc && holder.doc.nodeType");
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
};

TEST_F(AnAssignmentKeepsTheDocumentItIsGivenTest, BothLocationsHoldTheDocumentTheEventCarried) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/an_assignment_keeps_the_document_it_is_given/"
                                "an_assignment_keeps_the_document_it_is_given.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();

    sm_ = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());

    // §scxml-6.2: the session's `<send>` to itself needs the dispatcher chain
    // (scheduler, target factory, dispatcher).
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

    // A top-level `<final>` halts processing, so the state freezes on the verdict.
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (std::chrono::steady_clock::now() < deadline && sm_->isRunning()) {
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
        eventRaiser->processQueuedEvents();
    }
    ASSERT_FALSE(sm_->isRunning()) << "the session did not halt within 5s: its own `doc` event never arrived";

    const std::string reached = sm_->terminalState().value_or("");
    EXPECT_NE(reached, "nameLost") << "`<assign location=\"v\" expr=\"_event.data\"/>` did not leave the document in "
                                      "`v`: the value was read out of the data model and written back. "
                                   << describe();
    EXPECT_NE(reached, "memberLost") << "`<assign location=\"holder.doc\" expr=\"_event.data\"/>` did not leave the "
                                        "document in `holder.doc`. "
                                     << describe();
    EXPECT_EQ(reached, "kept") << describe();
}

}  // namespace Tests
}  // namespace SCE
