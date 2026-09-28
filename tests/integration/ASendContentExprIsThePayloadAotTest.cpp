// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.6.2 + 6.2: a <send>'s <content expr> is evaluated when the send
// is, and its value is the event's data — C++ AOT.
//
// Fixture: integration_resources/a_send_content_expr_is_the_payload/a_send_content_expr_is_the_payload.scxml
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(a_send_content_expr_is_the_payload ...)`.

#include "a_send_content_expr_is_the_payload_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::a_send_content_expr_is_the_payload::a_send_content_expr_is_the_payload;

std::string show(const std::optional<int64_t> &value) {
    return value ? std::to_string(*value) : std::string("<unreadable>");
}

}  // namespace

TEST(ASendContentExprIsThePayloadAotTest, ASendContentExprIsThePayload) {
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    sm.initialize();
    EXPECT_TRUE(sm.runUntilCompletion(std::chrono::seconds(3))) << "the machine never completed";

    EXPECT_EQ(sm.terminalState(), SM::State::Done) << "the run must end in `done`";
    EXPECT_EQ(sm.numberOk(), std::optional<int64_t>(1)) << "numberOk = " << show(sm.numberOk());
    EXPECT_EQ(sm.objectOk(), std::optional<int64_t>(1)) << "objectOk = " << show(sm.objectOk());
    EXPECT_EQ(sm.textOk(), std::optional<int64_t>(1)) << "textOk = " << show(sm.textOk());
    EXPECT_EQ(sm.errors(), std::optional<int64_t>(1)) << "errors = " << show(sm.errors());
    EXPECT_EQ(sm.badArrived(), std::optional<int64_t>(1)) << "badArrived = " << show(sm.badArrived());
    EXPECT_EQ(sm.badEmpty(), std::optional<int64_t>(1)) << "badEmpty = " << show(sm.badEmpty());
    EXPECT_EQ(sm.afterBad(), std::optional<int64_t>(0)) << "afterBad = " << show(sm.afterBad());
}

}  // namespace SCE::Tests
