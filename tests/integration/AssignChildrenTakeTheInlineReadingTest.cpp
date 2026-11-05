// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.4 and B.2: the children of an `<assign>` specify the value its
// `expr` would have, and are read as every in-line value is read — the value
// of the expression they are when they are one, and their space-normalized
// text otherwise — Interpreter path.
//
// Measured 2026-11-05 by reading both engines: the Interpreter quoted the
// children as a string whatever they were, so `<assign location="n">21
// </assign>` stored the string "21", where the generated engines store the
// number (sce-build's `to_*_assign_content` decides the reading when the
// machine is built). An assignment has no XML rung there: children that open
// with `<` are no expression and are their space-normalized text.

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <gtest/gtest.h>
#include <memory>
#include <string>

namespace SCE {
namespace Tests {

namespace {

// Every assignment runs on entry; an error.execution raised by any of them
// ends the run in `failed`, and otherwise the machine rests in `s`.
constexpr const char *kDocument = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="s">
  <datamodel>
    <data id="n"/>
    <data id="o" expr="({})"/>
    <data id="t"/>
    <data id="x"/>
  </datamodel>
  <state id="s">
    <onentry>
      <assign location="n">21</assign>
      <assign location="o.n">
        21
      </assign>
      <assign location="t">hello   world</assign>
      <assign location="x"><a>b  c</a></assign>
    </onentry>
    <transition event="error.execution" target="failed"/>
  </state>
  <final id="failed"/>
</scxml>)";

}  // namespace

class AssignChildrenTakeTheInlineReadingTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();

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

        ASSERT_TRUE(sm_->loadSCXMLFromString(kDocument));
        ASSERT_TRUE(sm_->start());
        // The run needs nothing from the host; drain whatever `start` left queued.
        eventRaiser->processQueuedEvents();
    }

    void TearDown() override {
        sm_.reset();
        if (engine_) {
            engine_->shutdown();
        }
    }

    /// An expression evaluated in the session, as a string.
    std::string read(const char *expression) const {
        auto result = ScriptEngineProvider::getScriptEngine().evaluateExpression(sm_->getSessionId(), expression).get();
        return result.isSuccess() ? result.getValueAsString() : std::string("<unreadable>");
    }

    std::string describe() const {
        return "ended in " + sm_->terminalState().value_or("<running>") + ", n=" + read("n") + " o.n=" + read("o.n") +
               " t=" + read("t") + " x=" + read("x");
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
};

TEST_F(AssignChildrenTakeTheInlineReadingTest, NoAssignmentRaisesAnError) {
    EXPECT_NE(sm_->terminalState().value_or(""), "failed") << describe();
}

// Children that are an expression are its value, at a bare name and at a
// member path alike.
TEST_F(AssignChildrenTakeTheInlineReadingTest, ChildrenThatAreAnExpressionAreItsValue) {
    EXPECT_EQ(read("n === 21"), "true") << describe();
    EXPECT_EQ(read("o.n === 21"), "true") << describe();
}

// Text that is no expression is the string it spells, space-normalized.
TEST_F(AssignChildrenTakeTheInlineReadingTest, TextThatIsNoExpressionIsItsNormalizedString) {
    EXPECT_EQ(read("t === 'hello world'"), "true") << describe();
}

// Markup is no expression either: its text, space-normalized, as the
// generated engines store it.
TEST_F(AssignChildrenTakeTheInlineReadingTest, MarkupIsItsNormalizedText) {
    EXPECT_EQ(read("x === '<a xmlns=\"http://www.w3.org/2005/07/scxml\">b c</a>'"), "true") << describe();
}

}  // namespace Tests
}  // namespace SCE
