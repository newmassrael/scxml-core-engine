// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4 + 3.10: a <parallel> that declares a <history> child still
// completes — Interpreter path.
//
// Appendix D defines getChildStates(state) as the <state>, <final> and
// <parallel> children of a state; <history> and <initial> are pseudo-states
// and are not among them. §3.10 nonetheless makes <history> a legal child of
// <parallel>, so the two rules meet in exactly one place: a <parallel> with a
// history child. isInFinalState(p) asks every child STATE of p whether it is
// in a final state, and a history child is not one of them.
//
// This engine used to get it wrong in its own way: it wrapped every child of a
// <parallel> in a region object, the history child included, and that region
// could never reach a final state — so done.state.<id>, which waits for every
// region, never fired. The W3C suite has no fixture that pairs <parallel> with
// <history>, which is why nothing caught it then. That region layer is gone;
// completion is now CompletionAlgorithms::isInFinalState over the document's
// child states, and this is the behaviour pinned where the old unit test
// pinned the wrapper.

#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <gtest/gtest.h>

namespace SCE {
namespace Tests {

namespace {

/// Both regions finish on their own; only the parallel's done event reaches
/// `pass`. The history child is declared FIRST, so a child-state list that
/// kept it would meet it before either region.
constexpr const char *DOCUMENT = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="ecmascript" initial="p">

  <parallel id="p">
    <history id="h" type="shallow">
      <transition target="r1"/>
    </history>
    <state id="r1" initial="a1">
      <state id="a1">
        <transition target="f1"/>
      </state>
      <final id="f1"/>
    </state>
    <state id="r2" initial="a2">
      <state id="a2">
        <transition target="f2"/>
      </state>
      <final id="f2"/>
    </state>
    <transition event="done.state.p" target="pass"/>
  </parallel>

  <final id="pass"/>
</scxml>
)";

}  // namespace

class ParallelWithAHistoryChildCompletesTest : public ::testing::Test {
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

    IScriptEngine *engine_ = nullptr;
};

TEST_F(ParallelWithAHistoryChildCompletesTest, EveryRegionFinalRaisesTheParallelsDoneEvent) {
    auto sm = std::make_shared<StateMachine>(*engine_);
    sm->setEventRaiser(std::make_shared<EventRaiserImpl>());
    ASSERT_TRUE(sm->loadSCXMLFromString(DOCUMENT));
    ASSERT_TRUE(sm->start());

    EXPECT_EQ(sm->terminalState().value_or(""), "pass")
        << "both regions of `p` reached their <final>, so done.state.p fires and takes the machine to `pass`. "
           "Stuck in `p` means the <history> child was counted as a child state that has to finish too — "
           "Appendix D's getChildStates leaves pseudo-states out";
    EXPECT_FALSE(sm->isRunning()) << "`pass` is a top-level <final>";
}

}  // namespace Tests
}  // namespace SCE
