// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: a reply an invoked child has already sent is on the
// parent's external queue, so a host that only hands the machine events
// still sees it ahead of its own later events — C++ AOT path.
//
// Sibling of `AChildReplyArrivesWithoutATickTest.cpp` (Interpreter channel).
//
// Fixture:
// integration_resources/a_child_reply_arrives_without_a_tick/a_child_reply_arrives_without_a_tick.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(a_child_reply_arrives_without_a_tick ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "a_child_reply_arrives_without_a_tick_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::a_child_reply_arrives_without_a_tick::a_child_reply_arrives_without_a_tick;

std::string describe(SM &sm) {
    const auto read = [](const std::optional<int64_t> &value) {
        return value ? std::to_string(*value) : std::string("<unreadable>");
    };
    return "hellos=" + read(sm.hellos()) + " (wanted 1)";
}

}  // namespace

TEST(AChildReplyArrivesWithoutATickAotTest, TheChildsReplyArrivesBeforeTheHostsNextEvent) {
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    sm.initialize();
    sm.processEvent(SM::Event::Finish);

    EXPECT_EQ(sm.terminalState(), SM::State::Done) << "`finish` must carry the run to `done`. " << describe(sm);
    EXPECT_EQ(sm.hellos(), std::optional<int64_t>(1))
        << "the child's start-time reply arrives before `finish`. " << describe(sm);
}

}  // namespace SCE::Tests
