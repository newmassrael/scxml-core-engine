// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.7.1 + 4.9: a <send> whose <param> cannot be read still sends
// its message without that pair, and the error ends its block; a valid
// location param is sent — C++ AOT path.
//
// Measured 2026-09-27, this channel let the rest of the block run, never
// read a `location`, and evaluated an empty one as undefined.
//
// Sibling of `ABadSendParamEndsItsBlockTest.cpp` (Interpreter channel).
//
// Fixture:
// integration_resources/a_bad_send_param_ends_its_block/a_bad_send_param_ends_its_block.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(a_bad_send_param_ends_its_block ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "a_bad_send_param_ends_its_block_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::a_bad_send_param_ends_its_block::a_bad_send_param_ends_its_block;

std::string describe(SM &sm) {
    const auto read = [](const std::optional<int64_t> &value) {
        return value ? std::to_string(*value) : std::string("<unreadable>");
    };
    return "errors=" + read(sm.errors()) + " partials=" + read(sm.partials()) + " bares=" + read(sm.bares()) +
           " after=" + read(sm.after()) + " carried=" + read(sm.carried()) + " (wanted 3 / 2 / 1 / 0 / 1)";
}

}  // namespace

TEST(ABadSendParamEndsItsBlockAotTest, TheMessageGoesAndTheBlockStops) {
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    sm.initialize();
    sm.processEvent(SM::Event::Finish);

    EXPECT_EQ(sm.terminalState(), SM::State::Done) << "`finish` must carry the run to `done`. " << describe(sm);
    EXPECT_EQ(sm.errors(), std::optional<int64_t>(3))
        << "each unreadable <param> raises one error.execution. " << describe(sm);
    EXPECT_EQ(sm.partials(), std::optional<int64_t>(2))
        << "both internal sends go, carrying the good pair without the bad one. " << describe(sm);
    EXPECT_EQ(sm.bares(), std::optional<int64_t>(1))
        << "the external send goes with its empty pair left out. " << describe(sm);
    EXPECT_EQ(sm.after(), std::optional<int64_t>(0))
        << "the <param> error ends the block, so nothing after the <send> runs. " << describe(sm);
    EXPECT_EQ(sm.carried(), std::optional<int64_t>(1))
        << "a valid location param is sent with its value. " << describe(sm);
}

}  // namespace SCE::Tests
