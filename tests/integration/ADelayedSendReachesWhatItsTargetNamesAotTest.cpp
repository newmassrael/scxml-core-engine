// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 6.4 + C.1: a delay postpones a <send>, it does not change
// where the send goes — C++ AOT.
//
// Fixture:
// integration_resources/a_delayed_send_reaches_what_its_target_names/a_delayed_send_reaches_what_its_target_names.scxml
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(a_delayed_send_reaches_what_its_target_names ...)`.

#include "a_delayed_send_reaches_what_its_target_names_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::a_delayed_send_reaches_what_its_target_names::a_delayed_send_reaches_what_its_target_names;

std::string show(const std::optional<int64_t> &value) {
    return value ? std::to_string(*value) : std::string("<unreadable>");
}

}  // namespace

TEST(ADelayedSendReachesWhatItsTargetNamesAotTest, ADelayedSendReachesWhatItsTargetNames) {
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    sm.initialize();
    EXPECT_TRUE(sm.runUntilCompletion(std::chrono::seconds(3))) << "the machine never completed";

    EXPECT_EQ(sm.terminalState(), SM::State::Done) << "the run must end in `done`";
    EXPECT_EQ(sm.order(), std::optional<int64_t>(31)) << "order = " << show(sm.order());
    EXPECT_EQ(sm.innerInternal(), std::optional<int64_t>(1)) << "innerInternal = " << show(sm.innerInternal());
    EXPECT_EQ(sm.lateOk(), std::optional<int64_t>(1)) << "lateOk = " << show(sm.lateOk());
    EXPECT_EQ(sm.lateCount(), std::optional<int64_t>(1)) << "lateCount = " << show(sm.lateCount());
    EXPECT_EQ(sm.pongOk(), std::optional<int64_t>(1)) << "pongOk = " << show(sm.pongOk());
    EXPECT_EQ(sm.commErrors(), std::optional<int64_t>(2)) << "commErrors = " << show(sm.commErrors());
    EXPECT_EQ(sm.lostArrived(), std::optional<int64_t>(0)) << "lostArrived = " << show(sm.lostArrived());
    EXPECT_EQ(sm.afterStranger(), std::optional<int64_t>(0)) << "afterStranger = " << show(sm.afterStranger());
}

}  // namespace SCE::Tests
