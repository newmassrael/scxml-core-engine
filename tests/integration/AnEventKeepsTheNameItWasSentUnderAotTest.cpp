// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The name an event arrives under (§scxml-5.10, §scxml-3.12.1) — C++ AOT
// channel.
//
// Sibling of `AnEventKeepsTheNameItWasSentUnderTest.cpp` (Interpreter channel).
// An AOT machine carries an event as the enum member of a name its document
// writes, so a name it does not write reaches it as the member of the longest
// prefix it does, and has to carry the whole name beside that member for
// `_event.name` to read it.
//
// Fixture:
// integration_resources/an_event_keeps_the_name_it_was_sent_under/an_event_keeps_the_name_it_was_sent_under.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(an_event_keeps_the_name_it_was_sent_under ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "an_event_keeps_the_name_it_was_sent_under_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

TEST(AnEventKeepsTheNameItWasSentUnderAotTest, ANameTheDocumentDoesNotWriteIsToldWhole) {
    using SM = SCE::Generated::an_event_keeps_the_name_it_was_sent_under::an_event_keeps_the_name_it_was_sent_under;

    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        // Aliasing constructor + no-op deleter — engine lifetime is owned by the
        // ScriptEngineProvider singleton; the shared_ptr is a non-owning view.
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    sm.initialize();
    const bool reachedFinal = sm.runUntilCompletion(std::chrono::seconds(3));

    EXPECT_TRUE(reachedFinal) << "parent did not reach a final state within timeout — the child never "
                                 "heard `request.new`, so it never answered";
    EXPECT_EQ(sm.terminalState(), SM::State::Pass)
        << "the child reported `arrivedShortened`: `request.new` took the transition on `request` but "
           "`_event.name` told the child `request`. W3C §5.10 makes the name the one the event was sent "
           "under; the arrival name must travel beside the enum member the document's own names give it.";
}

}  // namespace SCE::Tests
