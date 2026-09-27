// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2: a <send> delay is read as one CSS2 time — C++ AOT path.
//
// Sibling of `ADelayIsACss2TimeTest.cpp` (Interpreter channel).
//
// Fixture: integration_resources/a_delay_is_a_css2_time/a_delay_is_a_css2_time.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(a_delay_is_a_css2_time ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "a_delay_is_a_css2_time_sm.h"
#include "common/SceClock.h"
#include "scripting/ScriptEngineProvider.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::a_delay_is_a_css2_time::a_delay_is_a_css2_time;

std::string show(const std::optional<int64_t> &value) {
    return value ? std::to_string(*value) : std::string("<unreadable>");
}

}  // namespace

// On a manual clock advanced a full minute: both valid delays fire in the order
// their milliseconds give, and a refused message, had it been scheduled under
// some default wait, would have arrived and moved `bad`.
TEST(ADelayIsACss2TimeAotTest, EachDelayIsReadAsOneCss2Time) {
    SM sm;
    sm.setClock(std::make_shared<SCE::ManualClock>(0));
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    sm.initialize();
    sm.advanceTimeMs(60000);

    EXPECT_EQ(sm.terminalState(), SM::State::Done) << "`a` must carry the run to `done`";
    EXPECT_EQ(sm.errors(), std::optional<int64_t>(2)) << "errors = " << show(sm.errors());
    EXPECT_EQ(sm.after(), std::optional<int64_t>(0)) << "after = " << show(sm.after());
    EXPECT_EQ(sm.bad(), std::optional<int64_t>(0)) << "bad = " << show(sm.bad());
    EXPECT_EQ(sm.aAfterB(), std::optional<int64_t>(1)) << "aAfterB = " << show(sm.aAfterB());
}

}  // namespace SCE::Tests
