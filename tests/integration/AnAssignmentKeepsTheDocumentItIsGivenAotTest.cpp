// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.4 and B.2: `<assign location="v" expr="_event.data"/>` leaves
// `v` holding the DOM a `<send>`'s XML `<content>` arrived as — C++ AOT.
//
// The generated machine's `<assign>` reaches the same
// `AssignmentExecutionHelper` the Interpreter's does, so it lost the document
// at a bare-name location the same way (read 2026-11-06), and the engine now
// runs every assignment itself for both.
//
// Fixture:
// integration_resources/an_assignment_keeps_the_document_it_is_given/an_assignment_keeps_the_document_it_is_given.scxml
// (canonical, shared with every channel).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(an_assignment_keeps_the_document_it_is_given ...)`.

#include "an_assignment_keeps_the_document_it_is_given_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

TEST(AnAssignmentKeepsTheDocumentItIsGivenAotTest, BothLocationsHoldTheDocumentTheEventCarried) {
    using SM =
        SCE::Generated::an_assignment_keeps_the_document_it_is_given::an_assignment_keeps_the_document_it_is_given;

    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        // Non-owning view of the ScriptEngineProvider singleton, as the
        // sibling AOT drivers hold it.
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    EXPECT_EQ(sm.initialize(), std::nullopt);
    const bool reachedFinal = sm.runUntilCompletion(std::chrono::seconds(3));

    EXPECT_TRUE(reachedFinal) << "the machine did not reach a final state within the timeout: its own `doc` "
                                 "event never arrived";
    EXPECT_NE(sm.terminalState(), SM::State::NameLost)
        << "`<assign location=\"v\" expr=\"_event.data\"/>` did not leave the document in `v`: the value was "
           "read out of the data model and written back.";
    EXPECT_NE(sm.terminalState(), SM::State::MemberLost)
        << "`<assign location=\"holder.doc\" expr=\"_event.data\"/>` did not leave the document in `holder.doc`.";
    EXPECT_EQ(sm.terminalState(), SM::State::Kept);
}

}  // namespace SCE::Tests
