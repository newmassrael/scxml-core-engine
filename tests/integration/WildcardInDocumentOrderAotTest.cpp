// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A wildcard keeps its own guard and its own type — C++ AOT path.
//
// W3C SCXML 3.12.1 lets a `*` descriptor match every event, and that is all
// it changes: W3C SCXML 3.13 still asks the transition's `cond` whether it is
// enabled, and a `type="internal"` transition whose target is a proper
// descendant of its compound source still does not exit that source.
//
// No W3C document writes a wildcard with a `cond` or with `type="internal"`, so
// a generator that moved the wildcard into a hand-written fallback dropped both
// without a suite noticing — the Kotlin one did until 2026-09-13.
//
// Sibling of `WildcardInDocumentOrderTest.cpp` (Interpreter channel). Both
// engines ship in production, so each is held to the clause independently
// against one canonical fixture.
//
// Fixture: integration_resources/wildcard_in_document_order/wildcard_in_document_order.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(wildcard_in_document_order ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "scripting/ScriptEngineProvider.h"
#include "wildcard_in_document_order_sm.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

TEST(WildcardInDocumentOrderAotTest, AWildcardKeepsItsGuardAndItsType) {
    using SM = SCE::Generated::wildcard_in_document_order::wildcard_in_document_order;

    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    sm.initialize();
    const bool completed = sm.runUntilCompletion(std::chrono::seconds(3));

    EXPECT_TRUE(completed) << "the machine never reached a final state; resting in GuardedFrom or SealedFrom "
                              "means an internal wildcard was not taken at all";

    // Each failure final names the case, so this one assertion says which
    // property of the wildcard was lost:
    //   FailGuardIgnored              a wildcard fired with its guard false
    //   FailGuardNeverFired           a wildcard did not fire with its guard true
    //   FailGuardedInternalReentered  a guarded internal wildcard exited its source
    //   FailSealedInternalReentered   an unguarded internal wildcard exited its source
    EXPECT_EQ(sm.terminalState(), SM::State::Pass)
        << "the machine did not reach pass: a wildcard is enabled only when its guard is true, and an "
           "internal wildcard targeting a descendant of its compound source must not exit and re-enter "
           "that source";
}

}  // namespace SCE::Tests
