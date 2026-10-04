// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A computed event name is matched like any other (§scxml-3.12.1, §scxml-5.10) —
// C++ AOT channel.
//
// Sibling of `AComputedEventNameIsMatchedLikeAnyOtherTest.cpp` (Interpreter
// channel). An AOT machine carries an event as the enum member of a name its
// document writes, so a `<send eventexpr>` whose name it does not write has to be
// resolved to the member of the longest prefix it does (or its wildcard), and has
// to carry the whole name beside that member for `_event.name` to read it, over
// the external and the internal queue, now and after a delay.
//
// Fixture:
// integration_resources/a_computed_event_name_is_matched_like_any_other/a_computed_event_name_is_matched_like_any_other.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(a_computed_event_name_is_matched_like_any_other ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "a_computed_event_name_is_matched_like_any_other_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

TEST(AComputedEventNameIsMatchedLikeAnyOtherAotTest, AComputedNameIsMatchedByTokenAndToldWhole) {
    using SM = SCE::Generated::a_computed_event_name_is_matched_like_any_other::
        a_computed_event_name_is_matched_like_any_other;

    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        // Aliasing constructor + no-op deleter — engine lifetime is owned by the
        // ScriptEngineProvider singleton; the shared_ptr is a non-owning view.
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    sm.initialize();
    const bool reachedFinal = sm.runUntilCompletion(std::chrono::seconds(3));

    EXPECT_TRUE(reachedFinal) << "the machine did not reach a final state within timeout — a computed name was "
                                 "dropped (`request.new`, `other.thing`, `request.again`, `last.one`), or matched "
                                 "but told shorter than the name it was sent under";
    EXPECT_EQ(sm.terminalState(), SM::State::Pass);
}

}  // namespace SCE::Tests
