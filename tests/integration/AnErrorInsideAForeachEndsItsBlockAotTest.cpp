// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 4.9 + 4.6: an error raised inside a <foreach> body ends the
// block that contains the <foreach>, and it is the only error raised —
// C++ AOT path.
//
// Sibling of `AnErrorInsideAForeachEndsItsBlockTest.cpp` (Interpreter channel).
//
// Fixture:
// integration_resources/an_error_inside_a_foreach_ends_its_block/an_error_inside_a_foreach_ends_its_block.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(an_error_inside_a_foreach_ends_its_block ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "an_error_inside_a_foreach_ends_its_block_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::an_error_inside_a_foreach_ends_its_block::an_error_inside_a_foreach_ends_its_block;

std::string show(const std::optional<int64_t> &value) {
    return value ? std::to_string(*value) : std::string("<unreadable>");
}

}  // namespace

TEST(AnErrorInsideAForeachEndsItsBlockAotTest, TheErrorEndsTheBlockAndIsTheOnlyOne) {
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    sm.initialize();
    sm.processEvent(SM::Event::Go);
    sm.processEvent(SM::Event::T);
    sm.processEvent(SM::Event::Finish);

    EXPECT_EQ(sm.terminalState(), SM::State::Done) << "`finish` must carry the run to `done`";
    EXPECT_EQ(sm.iters1(), std::optional<int64_t>(1)) << "iters1 = " << show(sm.iters1());
    EXPECT_EQ(sm.after1(), std::optional<int64_t>(0)) << "after1 = " << show(sm.after1());
    EXPECT_EQ(sm.iters2(), std::optional<int64_t>(1)) << "iters2 = " << show(sm.iters2());
    EXPECT_EQ(sm.after2(), std::optional<int64_t>(0)) << "after2 = " << show(sm.after2());
    EXPECT_EQ(sm.iters3(), std::optional<int64_t>(1)) << "iters3 = " << show(sm.iters3());
    EXPECT_EQ(sm.after3(), std::optional<int64_t>(0)) << "after3 = " << show(sm.after3());
    EXPECT_EQ(sm.errors(), std::optional<int64_t>(3)) << "errors = " << show(sm.errors());
    EXPECT_EQ(sm.sent(), std::optional<int64_t>(2)) << "sent = " << show(sm.sent());
}

}  // namespace SCE::Tests
