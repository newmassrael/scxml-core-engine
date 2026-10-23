// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// datamodel="sce-static" (docs/SCE_ACCEPTED_SUBSET.md §2.15) under generated C++: what an
// `<invoke type="scxml">` hands the child session it starts, held where the outcome is
// the machine's alone and a scenario would only restate it — a child that waits for what
// it was not handed, and a hybrid invoke that starts the candidate its value names.

#include <string>

#include <gtest/gtest.h>

#include "static_invoke_hybrid_saved_sm.h"
#include "static_invoke_hybrid_sm.h"
#include "static_invoke_params_sm.h"
#include "static_invoke_string_sm.h"

namespace SCE::Tests {
namespace {

namespace G = ::SCE::Generated;

// An `<invoke type="scxml">` hands its child the values its `<param>`s and `namelist`
// name (§scxml-6.4.1), each to the child's variable of the same name. `worker` ends the
// moment it holds `start = 7` (a `<param>` reading `base`, which is 4 when `working` is
// entered and 7 once its entry action ran: the value is read when the invoke executes, at
// the end of the macrostep) and `enabled = true` (the `namelist`); handed less, it would
// wait and the parent would stay in `working`. `control`, the same child handed nothing,
// keeps what its `<data>` gave it and never ends.
class AStaticChildIsHandedItsParams : public ::testing::Test {
protected:
    using Machine = G::static_invoke_params::static_invoke_params;

    /// Let the child run and report to its parent.
    void settle() {
        for (int i = 0; i < 5; ++i) {
            machine.tick();
        }
    }

    std::string state() const {
        return Machine::PolicyType::getStateName(machine.getCurrentState());
    }

    void SetUp() override {
        machine.initialize();
        settle();
    }

    Machine machine;
};

TEST_F(AStaticChildIsHandedItsParams, AChildEndsOnceItHoldsTheValuesItsInvokeNames) {
    // `worker` ended, so it held both values: the parent left `working` and counted it.
    EXPECT_EQ(state(), "plain");
    EXPECT_EQ(machine.completed(), 1u);
}

TEST_F(AStaticChildIsHandedItsParams, AChildHandedNothingKeepsTheValuesItsDataGaveIt) {
    settle();
    // `control` still waits for 7 and true, so `done.invoke.control` never counted.
    EXPECT_EQ(state(), "plain");
    EXPECT_EQ(machine.completed(), 1u);
}

// A string an `<invoke type="scxml">` hands its child is held to the bound the child
// declared for that variable, in UTF-8 bytes, as an `<assign>` to it would be: a value
// past it is left out and reported (§scxml-5.7.1), and the child still starts, holding the
// one its `<data>` gave it. `fits` is handed the four bytes its `title` holds and ends on
// them; `over` is handed eight bytes and `wide` two characters of five bytes, so each
// starts with the 'ab' it ends on: 1 + 10 + 100, and two errors.
TEST(AStaticChildSessionTest, AStringHandedToAChildIsHeldToTheChildsBound) {
    G::static_invoke_string::static_invoke_string machine;
    machine.initialize();
    for (int i = 0; i < 5; ++i) {
        machine.tick();
    }
    EXPECT_EQ(machine.completed(), 111u);
    EXPECT_EQ(machine.errors(), 2u);
}

// An `<invoke srcexpr>` that declares `sce:candidates` starts the document its value names
// (§scxml-6.4), by the document's stem, and hands it the invoke's arguments, each to the
// variable of the same name that candidate declares (§scxml-6.4.3). The fixture runs four
// phases: `first` (a `file:` value), `second` (an absolute path), `lossy` (an `extra` no
// 32-bit field can hold: reported, left out, the child still ends) and `missing` (a
// document the invoke did not declare: reported, nothing starts). Each `done.invoke` adds
// a power of ten of its own, so 111 says which candidates ended, and the two errors are
// the `lossy` argument and the `missing` document. A candidate handed what the OTHER
// declares would never end.
class AStaticHybridInvokeStartsTheCandidateItsValueNames : public ::testing::Test {
protected:
    using Machine = G::static_invoke_hybrid::static_invoke_hybrid;

    void SetUp() override {
        machine.initialize();
        for (int i = 0; i < 40; ++i) {
            machine.tick();
        }
    }

    Machine machine;
};

TEST_F(AStaticHybridInvokeStartsTheCandidateItsValueNames, EachPhaseStartsTheCandidateItsValueNamesAndEnds) {
    EXPECT_EQ(machine.completed(), 111u);
    EXPECT_TRUE(machine.isInFinalState()) << "the last phase named no declared candidate, so the run is over";
}

TEST_F(AStaticHybridInvokeStartsTheCandidateItsValueNames, AnArgumentIsEvaluatedWhateverTheCandidateKeeps) {
    EXPECT_EQ(machine.errors(), 2u);
}

TEST_F(AStaticHybridInvokeStartsTheCandidateItsValueNames, AValueNamingNoDeclaredCandidateStartsNothing) {
    // `done.invoke.missing_run` would add 1000: nothing started to send it.
    EXPECT_LT(machine.completed(), 1000u);
}

// A hybrid `<invoke>` of a document that never mentions `error.execution` has no such
// event in its enum, and what the invoke does where it would raise one (a value naming no
// declared candidate, an argument that cannot be computed) is to start nothing and say
// nothing, not to name a member the document never declared. `static_invoke_hybrid_saved`
// is such a document: the watcher it starts is handed 7 and waits for an 8 nothing gives
// it.
TEST(AStaticChildSessionTest, AHybridInvokeOfADocumentWithNoErrorEventStartsItsCandidate) {
    G::static_invoke_hybrid_saved::static_invoke_hybrid_saved machine;
    machine.initialize();
    for (int i = 0; i < 5; ++i) {
        machine.tick();
    }
    EXPECT_EQ(machine.completed(), 0u);
    EXPECT_FALSE(machine.isInFinalState());
}

}  // namespace
}  // namespace SCE::Tests
