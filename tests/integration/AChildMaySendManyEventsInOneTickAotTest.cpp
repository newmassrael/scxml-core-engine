// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: every event an invoked child sends to `#_parent` reaches
// the parent, however many it sends in one tick — C++ AOT path.
//
// Sibling of `AChildMaySendManyEventsInOneTickTest.cpp` (Interpreter channel).
//
// Fixture:
// integration_resources/a_child_may_send_many_events_in_one_tick/a_child_may_send_many_events_in_one_tick.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(a_child_may_send_many_events_in_one_tick ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "a_child_may_send_many_events_in_one_tick_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::a_child_may_send_many_events_in_one_tick::a_child_may_send_many_events_in_one_tick;

std::string describe(SM &sm) {
    const auto read = [](const std::optional<int64_t> &value) {
        return value ? std::to_string(*value) : std::string("<unreadable>");
    };
    return "ticks=" + read(sm.ticks()) + " (wanted 110)";
}

}  // namespace

TEST(AChildMaySendManyEventsInOneTickAotTest, EveryEventTheChildSentArrives) {
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    sm.initialize();
    sm.processEvent(SM::Event::Finish);

    EXPECT_EQ(sm.terminalState(), SM::State::Done) << "`finish` must carry the run to `done`. " << describe(sm);
    EXPECT_EQ(sm.ticks(), std::optional<int64_t>(110))
        << "every event the child sent arrives before `finish`. " << describe(sm);
}

}  // namespace SCE::Tests
