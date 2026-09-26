// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: an invoke whose state leaves within its macrostep is never
// attempted, so the §6.4.1 error.execution for a type no processor runs is
// never raised — C++ AOT path.
//
// Measured 2026-09-26, this channel already raised nothing; the fixture pins
// it.
//
// Sibling of `AnInvokeLeftBeforeItStartsRaisesNothingTest.cpp` (Interpreter
// channel).
//
// Fixture:
// integration_resources/an_invoke_left_before_it_starts_raises_nothing/an_invoke_left_before_it_starts_raises_nothing.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(an_invoke_left_before_it_starts_raises_nothing ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "an_invoke_left_before_it_starts_raises_nothing_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>

namespace SCE::Tests {

namespace {

using SM =
    SCE::Generated::an_invoke_left_before_it_starts_raises_nothing::an_invoke_left_before_it_starts_raises_nothing;

}  // namespace

TEST(AnInvokeLeftBeforeItStartsRaisesNothingAotTest, TheLeftStatesInvokeIsNeverAttempted) {
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    sm.initialize();
    ASSERT_EQ(sm.getCurrentState(), SM::State::S1)
        << "`s0` leaves on an eventless transition within the first macrostep";

    sm.processEvent(SM::Event::Finish);

    EXPECT_EQ(sm.terminalState(), SM::State::Done) << "`finish` must carry the run to `done`";
    EXPECT_EQ(sm.errors(), std::optional<int64_t>(0))
        << "`s0` left before its macrostep ended, yet its invoke was attempted and raised error.execution; "
           "W3C SCXML 6.4 cancels an invoke whose state has left";
}

}  // namespace SCE::Tests
