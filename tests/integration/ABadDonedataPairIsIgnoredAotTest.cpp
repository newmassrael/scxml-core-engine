// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.7: a <donedata> pair that cannot be evaluated raises
// error.execution and is ignored; the done events are raised all the same —
// C++ AOT path.
//
// Measured 2026-09-27, this channel evaluated a `location=""` pair as the
// identifier `none` (the template's fallback rendered an absent `expr`), so
// the answer depended on whether the data model happened to hold `none`.
//
// Sibling of `ABadDonedataPairIsIgnoredTest.cpp` (Interpreter channel).
//
// Fixture:
// integration_resources/a_bad_donedata_pair_is_ignored/a_bad_donedata_pair_is_ignored.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(a_bad_donedata_pair_is_ignored ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "a_bad_donedata_pair_is_ignored_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::a_bad_donedata_pair_is_ignored::a_bad_donedata_pair_is_ignored;

std::string describe(SM &sm) {
    const auto read = [](const std::optional<int64_t> &value) {
        return value ? std::to_string(*value) : std::string("<unreadable>");
    };
    return "errors=" + read(sm.errors()) + " shape=" + read(sm.shape()) + " (wanted 2 / 1)";
}

}  // namespace

TEST(ABadDonedataPairIsIgnoredAotTest, TheBadPairsAreDroppedAndTheDoneEventsStillArrive) {
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    // The run needs nothing from the host.
    sm.initialize();

    EXPECT_EQ(sm.terminalState(), SM::State::Done)
        << "done.state.p must still arrive and carry the run to `done`. " << describe(sm);
    EXPECT_EQ(sm.errors(), std::optional<int64_t>(2))
        << "each ignored pair raises its own error.execution. " << describe(sm);
    EXPECT_EQ(sm.shape(), std::optional<int64_t>(1))
        << "done.state.r1 must carry the surviving pair and neither ignored one. " << describe(sm);
}

}  // namespace SCE::Tests
