// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.5.2: what an EMPTY `<finalize>` does, and what an absent one does
// not — C++ AOT path.
//
// With no executable content the Processor "MUST update the data model each
// time an event is received from the child process ... as if by `<assign>`
// with any return value that has a name that matches", and: "Note that the
// automatic update does not take place if the `<finalize>` element is absent
// as opposed to empty."
//
// Two defects met here. The clause was unrepresentable — `finalize_content`
// is one string, so an empty element and a missing one were the same value —
// and the body this channel did run was never lowered: the parser hands the
// finalize over as JavaScript and this template passed it to a Lua engine
// unescaped by the frontend, so a `<finalize>` worked only when its JavaScript
// was valid Lua as well. W3C test233's body is one bare assignment, which is,
// and it is the only `<finalize>` body in the corpus.
//
// Sibling of `EmptyFinalizeUpdatesTheLocationTest.cpp` (Interpreter), which
// never had the lowering gap: it keeps the finalize as XML and re-parses it as
// executable content.
//
// Fixture: integration_resources/empty_finalize_updates_the_location/empty_finalize_updates_the_location.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(empty_finalize_updates_the_location ...)`.

#include "empty_finalize_updates_the_location_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

TEST(EmptyFinalizeUpdatesTheLocationAotTest, AnEmptyFinalizeUpdatesTheLocationAndAnAbsentOneDoesNot) {
    using SM = SCE::Generated::empty_finalize_updates_the_location::empty_finalize_updates_the_location;

    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    sm.initialize();
    // Two 3s timeouts back to back; the budget outlasts both so a silent child
    // reaches its own verdict state instead of reading as a hang.
    const bool reachedFinal = sm.runUntilCompletion(std::chrono::seconds(20));

    EXPECT_TRUE(reachedFinal) << "parent did not reach a final state within timeout — neither child "
                                 "answered and neither delayed timeout fired";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailNotUpdated)
        << "the empty `<finalize/>` left `tally` at its old value: §scxml-6.5.2 makes an empty "
           "element mean the automatic update — for each `namelist` item the Processor updates the "
           "location as if by `<assign>` with the matching return value.";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailUpdatedWithoutFinalize)
        << "`guard` moved with no `<finalize>` element at all: the clause's note is a prohibition — "
           "\"the automatic update does not take place if the <finalize> element is absent as "
           "opposed to empty\".";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailUnmatchedNameWrote)
        << "an event carrying no matching name still wrote `keeper`: §scxml-6.5.2 says \"with ANY "
           "return value that has a name that matches\", so an unconditional write blanks the "
           "parent's data model on every unrelated answer the child sends.";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailUnmatchedChildSilent)
        << "the third child never answered, so the guarded-write half was never exercised.";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailEmptyChildSilent)
        << "the first child never answered, so the empty-`<finalize>` half was never exercised.";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailAbsentChildSilent)
        << "the second child never answered, so the absent-`<finalize>` half was never exercised.";
    EXPECT_EQ(sm.getCurrentState(), SM::State::Pass);
}

}  // namespace SCE::Tests
