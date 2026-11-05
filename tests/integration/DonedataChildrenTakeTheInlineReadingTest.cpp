// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.5 and B.2: the children of a `<donedata>`'s `<content>` are
// read under a data model as in-line content is read — the value of the
// expression they are when they are one, the XML as written when they open
// with `<`, and their space-normalized text otherwise — Interpreter path.
//
// Measured 2026-11-04, the Interpreter evaluated the children as an
// expression whatever they were, so `<content>hello   world</content>` raised
// error.execution and the done event arrived with no data, where the
// generated engines carry the string "hello world" (sce-build's
// `to_lua_data_content` decides the reading when the machine is built).
// A `<send>`'s in-line `<content>` already took the ordered reading; the
// donedata now takes it through the same helper.

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

class DonedataChildrenTakeTheInlineReadingTest : public ::testing::Test {
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

    /// A document whose compound state `p` completes at once through its
    /// `<final>`, whose donedata `<content>` holds `children`. The parent's
    /// done.state.p handler keeps `_event.data` in `got` and ends in `done`;
    /// an error.execution raised while the donedata is read ends in `failed`.
    /// A `probe`, when given, is a cond read of `_event.data` where it is
    /// bound, taken first and ending in `probed`.
    static std::string documentWithContent(const std::string &children, const std::string &probe) {
        const std::string probeTransition =
            probe.empty() ? std::string()
                          : "    <transition event=\"done.state.p\" cond=\"" + probe + "\" target=\"probed\"/>\n";
        return R"scxml(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="p">
  <datamodel>
    <data id="got"/>
  </datamodel>
  <state id="p" initial="f">
    <final id="f">
      <donedata><content>)scxml" +
               children + R"scxml(</content></donedata>
    </final>
    <transition event="error.execution" target="failed"/>
)scxml" + probeTransition +
               R"scxml(    <transition event="done.state.p" target="done">
      <assign location="got" expr="_event.data"/>
    </transition>
  </state>
  <final id="done"/>
  <final id="probed"/>
  <final id="failed"/>
</scxml>)scxml";
    }

    void run(const std::string &children, const std::string &probe = std::string()) {
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

        ASSERT_TRUE(sm_->loadSCXMLFromString(documentWithContent(children, probe)));
        ASSERT_TRUE(sm_->start());
        // The run needs nothing from the host; drain whatever `start` left queued.
        eventRaiser->processQueuedEvents();
    }

    /// An expression evaluated in the finished session, as a string.
    std::string read(const char *expression) const {
        auto result = ScriptEngineProvider::getScriptEngine().evaluateExpression(sm_->getSessionId(), expression).get();
        return result.isSuccess() ? result.getValueAsString() : std::string("<unreadable>");
    }

    std::string describe() const {
        return "ended in " + sm_->terminalState().value_or("<none>") + ", typeof got=" + read("typeof got");
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
};

// Text that is no expression is the string it spells, space-normalized: the
// third reading, and the one the generated engines carry.
TEST_F(DonedataChildrenTakeTheInlineReadingTest, TextThatIsNoExpressionIsItsNormalizedString) {
    run("hello   world");

    EXPECT_EQ(sm_->terminalState().value_or(""), "done")
        << "reading the children must raise no error.execution. " << describe();
    EXPECT_EQ(read("got === 'hello world'"), "true")
        << "_event.data must be the space-normalized string. " << describe();
}

// Children that are a value are that value, as a `<send>`'s are.
TEST_F(DonedataChildrenTakeTheInlineReadingTest, AJsonValueIsThatValue) {
    run(R"({"k": 7})");

    EXPECT_EQ(sm_->terminalState().value_or(""), "done") << describe();
    EXPECT_EQ(read("got.k === 7"), "true") << "_event.data must be the object the children spell. " << describe();
}

// Children that open with `<` travel as written, and the DOM is built where
// `_event.data` is bound: the document handle `<data>` XML gets too
// (`DOMBinding::createDOMObject`), read where the event is bound, as W3C
// test561 reads its `<send>`'s. It is read in the cond rather than through
// `got`: an `<assign>` of `_event.data` to a name stores the value the engine
// hands back (`AssignmentExecutionHelper`), a plain copy that keeps no DOM
// handle, so `got.documentElement` is undefined there.
TEST_F(DonedataChildrenTakeTheInlineReadingTest, XmlChildrenArriveAsTheirDocument) {
    run(R"(<result code="7">finished</result>)", "_event.data.documentElement.tagName === 'result' &amp;&amp; "
                                                 "_event.data.documentElement.getAttribute('code') === '7'");

    EXPECT_EQ(sm_->terminalState().value_or(""), "probed")
        << "_event.data must be the document the children are (`done` means the done event arrived and was not "
           "it; `failed` that reading the children raised error.execution). "
        << describe();
}

}  // namespace Tests
}  // namespace SCE
