// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2.4 + C.1: a `targetexpr` is routed as the same value written in
// `target` is, at once or after a delay — C++ AOT.
//
// Fixture:
// integration_resources/a_target_expression_is_routed_as_its_literal_is/a_target_expression_is_routed_as_its_literal_is.scxml
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(a_target_expression_is_routed_as_its_literal_is ...)`.

#include "a_target_expression_is_routed_as_its_literal_is_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>

namespace SCE::Tests {

namespace {

using SM =
    SCE::Generated::a_target_expression_is_routed_as_its_literal_is::a_target_expression_is_routed_as_its_literal_is;

std::string show(const std::optional<int64_t> &value) {
    return value ? std::to_string(*value) : std::string("<unreadable>");
}

}  // namespace

TEST(ATargetExpressionIsRoutedAsItsLiteralIsAotTest, ATargetExpressionIsRoutedAsItsLiteralIs) {
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    sm.initialize();
    EXPECT_TRUE(sm.runUntilCompletion(std::chrono::seconds(3))) << "the machine never completed";

    EXPECT_EQ(sm.terminalState(), SM::State::Done) << "the run must end in `done`";
    EXPECT_EQ(sm.internalNow(), std::optional<int64_t>(1)) << "internalNow = " << show(sm.internalNow());
    EXPECT_EQ(sm.internalLater(), std::optional<int64_t>(1)) << "internalLater = " << show(sm.internalLater());
    EXPECT_EQ(sm.kidNow(), std::optional<int64_t>(1)) << "kidNow = " << show(sm.kidNow());
    EXPECT_EQ(sm.kidLater(), std::optional<int64_t>(1)) << "kidLater = " << show(sm.kidLater());
    EXPECT_EQ(sm.sessNow(), std::optional<int64_t>(1)) << "sessNow = " << show(sm.sessNow());
    EXPECT_EQ(sm.sessLater(), std::optional<int64_t>(1)) << "sessLater = " << show(sm.sessLater());
    EXPECT_EQ(sm.commErrors(), std::optional<int64_t>(4)) << "commErrors = " << show(sm.commErrors());
    EXPECT_EQ(sm.execErrors(), std::optional<int64_t>(2)) << "execErrors = " << show(sm.execErrors());
    EXPECT_EQ(sm.afterStranger(), std::optional<int64_t>(0)) << "afterStranger = " << show(sm.afterStranger());
    EXPECT_EQ(sm.afterStrangerLater(), std::optional<int64_t>(0))
        << "afterStrangerLater = " << show(sm.afterStrangerLater());
    EXPECT_EQ(sm.afterOrphan(), std::optional<int64_t>(0)) << "afterOrphan = " << show(sm.afterOrphan());
    EXPECT_EQ(sm.afterOrphanLater(), std::optional<int64_t>(0)) << "afterOrphanLater = " << show(sm.afterOrphanLater());
    EXPECT_EQ(sm.afterBogus(), std::optional<int64_t>(0)) << "afterBogus = " << show(sm.afterBogus());
    EXPECT_EQ(sm.afterBogusLater(), std::optional<int64_t>(0)) << "afterBogusLater = " << show(sm.afterBogusLater());
}

}  // namespace SCE::Tests
