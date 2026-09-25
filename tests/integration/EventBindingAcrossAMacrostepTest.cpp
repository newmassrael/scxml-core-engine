// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D and B.2.8.1: what `_event` holds, and what the payload
// report says, across the events one macrostep takes — Interpreter path.
//
// Appendix D's main event loop binds `_event` every time it takes an event —
// `datamodel["_event"] = internalEvent` — and never puts an earlier one back.
// So once a host's event has opened a macrostep that went on to take an
// internal event, `_event` is that internal event: the last one taken.
//
// This Interpreter used to disagree. It delivered an internal event by
// re-entering itself from inside the host's `processEvent`, and on the way out
// of that nested delivery it bound the host's event again. A document reading
// `_event` after the macrostep — the next guard evaluated, the host inspecting
// the datamodel — was answered about an event the loop had already finished
// with. The main event loop now takes internal events itself, one level deep,
// and the restore is gone with the nesting that needed it.
//
// What the second binding must NOT cost is the payload report of the first.
// `lastPayloadReading` is a single field every binding overwrites, so the
// reading is taken at the binding it belongs to (`bindCurrentEvent`), before
// the loop can take anything else.
//
// The document: a host event whose transition enters a compound state whose
// `<final>` carries `<donedata>`, so `done.state.wrapper` lands on the
// internal queue and is taken inside the host's macrostep. The host's event
// carries a payload that announces an object and does not parse as one; the
// done event carries one that reads cleanly — its only job is to be a second
// binding landing a different reading over the first.

#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <gtest/gtest.h>

namespace SCE {
namespace Tests {

namespace {

/// Announces an object and stops — `payloadReadingOfText` calls that
/// `Undecodable` because it opens with `{`.
constexpr const char *TRUNCATED_OBJECT = R"({"done":)";
/// The event the host hands over.
constexpr const char *HOST_EVENT = "outer";
/// The internal event the host's macrostep goes on to take.
constexpr const char *DONE_EVENT = "done.state.wrapper";

constexpr const char *DOCUMENT = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="ecmascript" initial="waiting">

  <state id="waiting">
    <transition event="outer" target="wrapper"/>
    <!-- Matches, does nothing, stays put: a macrostep that takes no other
         event at all. -->
    <transition event="lone"/>
  </state>

  <state id="wrapper" initial="inner">
    <state id="inner">
      <transition target="innerDone"/>
    </state>
    <final id="innerDone">
      <donedata>
        <content>42</content>
      </donedata>
    </final>
    <transition event="done.state.wrapper" target="settled"/>
  </state>

  <final id="settled"/>
</scxml>
)";

}  // namespace

class EventBindingAcrossAMacrostepTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();

        sm_ = std::make_shared<StateMachine>(*engine_);
        sm_->setEventRaiser(std::make_shared<EventRaiserImpl>());
        ASSERT_TRUE(sm_->loadSCXMLFromString(DOCUMENT));
        ASSERT_TRUE(sm_->start());
        ASSERT_EQ(sm_->getCurrentState(), "waiting");
        ASSERT_EQ(sm_->getStatistics().undecodablePayloads, 0u) << "nothing has been delivered before the first event";
    }

    void TearDown() override {
        sm_.reset();
        if (engine_) {
            engine_->shutdown();
        }
    }

    /// Read an expression in the machine's own session, after `processEvent`
    /// has returned — which is where the binding the macrostep left behind
    /// becomes observable.
    std::string readInSession(const std::string &expr) {
        auto result = engine_->evaluateExpression(sm_->getSessionId(), expr).get();
        EXPECT_TRUE(result.isSuccess()) << "`" << expr << "` is readable in this session";
        return result.isSuccess() ? result.getValueAsString() : std::string("<unreadable>");
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
};

/// The precondition, asserted rather than assumed: the host's macrostep really
/// does take a second event. If it stopped at `wrapper`, the done event was
/// never taken, there was no second binding, and nothing below would test
/// anything.
TEST_F(EventBindingAcrossAMacrostepTest, TheDoneEventIsTakenInsideTheHostsMacrostep) {
    const auto result = sm_->processEvent(HOST_EVENT, TRUNCATED_OBJECT);
    ASSERT_TRUE(result.success) << "the host's event matched `waiting`'s transition";

    ASSERT_EQ(sm_->getCurrentState(), "settled")
        << "the macrostep must have gone waiting -> wrapper -> (done.state.wrapper) -> settled";
}

/// A later binding does not cost the host's event its payload report: the
/// reading is taken at the binding it belongs to.
TEST_F(EventBindingAcrossAMacrostepTest, TheHostPayloadReportSurvivesALaterBinding) {
    ASSERT_TRUE(sm_->processEvent(HOST_EVENT, TRUNCATED_OBJECT).success);

    EXPECT_EQ(sm_->getStatistics().undecodablePayloads, 1u)
        << "the host sent `" << TRUNCATED_OBJECT << "` on `" << HOST_EVENT
        << "`, which announces an object and does not parse as one; the `" << DONE_EVENT
        << "` binding that followed read cleanly and must not have replaced that report, nor added one of its own";

    EXPECT_EQ(sm_->getStatistics().lastUndecodablePayloadEvent, HOST_EVENT)
        << "the payload that did not parse rode on `" << HOST_EVENT << "`, and the engine named `"
        << sm_->getStatistics().lastUndecodablePayloadEvent << "`";
}

/// Appendix D binds `_event` every time the loop takes an event and never puts
/// an earlier one back, so after the macrostep `_event` is the last event it
/// took: the done event, not the host's.
TEST_F(EventBindingAcrossAMacrostepTest, TheLastEventTakenStaysBound) {
    ASSERT_TRUE(sm_->processEvent(HOST_EVENT, TRUNCATED_OBJECT).success);
    ASSERT_EQ(sm_->getCurrentState(), "settled") << "the done event has to have been taken";

    EXPECT_EQ(readInSession("_event.name"), DONE_EVENT)
        << "the macrostep's last event was `" << DONE_EVENT << "`, and `_event` reads `" << readInSession("_event.name")
        << "`. `datamodel[\"_event\"] = internalEvent` is the last binding the loop made; an engine that "
           "rebinds the host's event after taking another is answering about an event it has finished with";
}

/// A macrostep that took no other event leaves the host's event bound.
TEST_F(EventBindingAcrossAMacrostepTest, ADeliveryThatTookNothingElseLeavesItselfBound) {
    ASSERT_TRUE(sm_->processEvent("lone", "just a sentence").success)
        << "`lone` matches a targetless transition on `waiting`";
    ASSERT_EQ(sm_->getCurrentState(), "waiting") << "a targetless transition leaves the configuration alone";

    EXPECT_EQ(readInSession("_event.name"), "lone")
        << "nothing else was taken, so `_event` must still be the event just handled";
}

}  // namespace Tests
}  // namespace SCE
