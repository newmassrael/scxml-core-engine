// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.2.8.1: a payload's rung is read while it is still true.
//
// ⚠⚠ READ THIS FIRST. This file is a SECOND witness, not the only one. The
// hazard `EventContextGuard` describes is already caught by
// `UndecodablePayloadIsReportedTest` — measured, below — so nothing here is
// load-bearing for it. What this adds is the nested shape, and the record of
// three mutation placements and which of them anything catches.
//
// The guard's constructor reads the rung immediately after binding, and says:
//
//     the binding just chose a rung, and this is the only frame that knows
//     which event it belonged to. Read immediately rather than later — the
//     guard's destructor binds the SAVED event on the way out, which would
//     overwrite it.
//
// The debt register held that this needed a NESTED delivery whose own payload is
// undecodable, on the reasoning that `isNested_` is always false in the sibling
// fixture so the restore branch never runs and a moved read is invisible there.
// That reasoning is wrong, and the reason it is wrong is worth more than the
// fixture it asked for: `lastPayloadReading_` is STICKY. It survives until the
// next `setCurrentEvent`, so moving the read changes when it is sampled relative
// to the sequence of bindings, and the counts move with no nesting at all.
//
// Three placements, each applied by hand to `StateMachine.cpp` and measured
// 2026-08-26 against this tree:
//
//   A. READ BEFORE THE BINDING — caught. `UndecodablePayloadIsReportedTest`
//      fails three assertions (counts, and the named event becomes the next
//      delivery's). This file fails too, naming `done.state.wrapper`.
//   B. READ AFTER THE RESTORE, in the destructor — caught. That fixture fails
//      two (1 became 2, 2 became 3, the name became `answer`). This file fails
//      one (1 became 2).
//   C. READ IN THE DESTRUCTOR BUT BEFORE THE RESTORE — survives everything,
//      and benignly: the inner guard's restore calls `setCurrentEvent(savedEvent_)`,
//      which RE-READS the outer payload and resurrects its rung, so the outer
//      guard then samples exactly what it would have sampled in its
//      constructor. Same count, same name. Nothing is wrong to catch.
//
// So the hazard is witnessed, and the facts below are about this document rather
// than about a gap:
//
//   1. NESTING IS AVAILABLE, and this file's document obtains it. Entering a
//      compound state's `<final>` queues `done.state.<parent>` internally, and
//      every `drainInternalEvents` call site sits inside the guard that
//      `processEvent` built — the engine logs
//      "EventContextGuard: Nested event processing - saving _event='outer',
//      setting new _event='done.state.wrapper'". So the hard half of the
//      precondition is solved, and that is worth keeping.
//
//   2. BUT A NESTED DELIVERY CANNOT CARRY A MALFORMED PAYLOAD. `<raise>` carries
//      no payload (`docs/SCE_ACCEPTED_SUBSET.md`: "An event raised by NAME
//      carries no payload"). `<send>` without a target goes to the EXTERNAL
//      queue, so it arrives top-level. And `<donedata>` — the route the debt
//      register named as confirmed — cannot: `DoneDataHelper::emitContentLiteral`
//      ships "a canonical JSON scalar" for a literal body and `evaluateContent`
//      JSON-serializes an evaluated one, both by documented design for the wire
//      path. Measured: `<content>{"done":</content>` arrives as `"{\"done\":"`,
//      which reads as Structured. Under `datamodel="null"` the parser takes the
//      literal branch and the same serialization still applies.
//
//   3. SO THE BAD PAYLOAD RIDES ON THE OUTER DELIVERY HERE, and the nested one
//      carries a clean payload whose only job is to be a second
//      `setCurrentEvent` landing a different rung over the first. That is what
//      placement B trips on, and it is the shape a document can actually have.

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
/// The name the engine must report: the delivery the host actually made.
constexpr const char *OUTER_EVENT = "outer";
/// The nested delivery, which carries a payload that reads cleanly and so
/// OVERWRITES `lastPayloadReading_` on its way past.
constexpr const char *INNER_EVENT = "done.state.wrapper";

/// A compound state whose `<final>` carries a payload that reads cleanly.
///
/// Reaching that `<final>` queues `done.state.wrapper` on the INTERNAL queue,
/// and the drain happens inside the `EventContextGuard` that `processEvent`
/// built for `outer` — which the engine says out loud:
///
///     EventContextGuard: Nested event processing - saving _event='outer',
///     setting new _event='done.state.wrapper'
///
/// `<content>42</content>` and not a broken payload, deliberately. Donedata
/// CANNOT carry a malformed payload on any route — `DoneDataHelper` ships
/// "a canonical JSON scalar" for a literal body and JSON-serializes an
/// evaluated one, so whatever a `<donedata>` says arrives as valid JSON. It
/// does not need to be malformed: its job here is to be a SECOND
/// `setCurrentEvent` that lands a different reading over the first.
constexpr const char *DOCUMENT = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="ecmascript" initial="waiting">

  <state id="waiting">
    <transition event="outer" target="wrapper"/>
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

class NestedDeliveryKeepsTheOuterPayloadReportTest : public ::testing::Test {
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

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
};

/// The precondition, asserted rather than assumed: this document really does
/// produce a nested delivery.
///
/// Worth its own test because it is the reusable half. `undecodable_payload_is_reported`
/// cannot have it — that fixture declares itself single-axis, its executable
/// content is `<assign>` only, and it has no `<raise>`, so `isNested_` is always
/// false there. A scheduling change that drained `done.state` outside the guard
/// would silently take the nesting away, and this says so instead.
TEST_F(NestedDeliveryKeepsTheOuterPayloadReportTest, TheInnerDeliveryIsNestedInsideTheOuterOne) {
    const auto result = sm_->processEvent(OUTER_EVENT, TRUNCATED_OBJECT);
    ASSERT_TRUE(result.success) << "the outer delivery matched `waiting`'s transition";

    ASSERT_EQ(sm_->getCurrentState(), "settled")
        << "the run must have gone waiting -> wrapper -> (done.state.wrapper) -> settled. "
           "If it stopped at `wrapper`, the internal `done.state` event was never drained, there "
           "was no second setCurrentEvent, and every assertion in the next test would pass "
           "against a reading nothing had a chance to overwrite";
}

/// What this DOES pin: a nested delivery does not cost the outer one its report.
///
/// `lastPayloadReading_` is a single field and every `setCurrentEvent` overwrites
/// it, so a second delivery landing inside the first is a real way to lose the
/// first one's rung. This asserts it is not lost.
///
/// ⚠ It does NOT distinguish a read taken in the constructor from one taken in
/// the destructor — measured, see point 3 of the file header. Do not read a
/// green here as covering that.
TEST_F(NestedDeliveryKeepsTheOuterPayloadReportTest, TheOuterReportSurvivesTheNestedDelivery) {
    ASSERT_TRUE(sm_->processEvent(OUTER_EVENT, TRUNCATED_OBJECT).success);

    EXPECT_EQ(sm_->getStatistics().undecodablePayloads, 1u)
        << "the host sent `" << TRUNCATED_OBJECT << "` on `" << OUTER_EVENT
        << "`, which announces an object and does not parse as one. A ZERO here is the shape this "
           "file exists to catch: the nested `"
        << INNER_EVENT
        << "` delivery called setCurrentEvent again and replaced the rung, so a reading taken "
           "after the guard's constructor no longer has the outer payload's to report";

    EXPECT_EQ(sm_->getStatistics().lastUndecodablePayloadEvent, OUTER_EVENT)
        << "the payload that did not parse rode on `" << OUTER_EVENT << "`, and the engine named `"
        << sm_->getStatistics().lastUndecodablePayloadEvent
        << "`. Naming the nested event means the rung was read against a delivery that did not "
           "lose anything";
}

}  // namespace Tests
}  // namespace SCE
