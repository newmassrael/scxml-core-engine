// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10 + 6.2: a <send>'s payload is the data of the event it sends,
// and a data-less event dequeued before it carries none — C++ AOT path.
//
// Sibling of `APayloadRidesOnItsOwnEventTest.cpp` (Interpreter channel).
//
// Fixture:
// integration_resources/a_payload_rides_on_its_own_event/a_payload_rides_on_its_own_event.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(a_payload_rides_on_its_own_event ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "a_payload_rides_on_its_own_event_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::a_payload_rides_on_its_own_event::a_payload_rides_on_its_own_event;

std::string describe(SM &sm) {
    const auto read = [](const std::optional<int64_t> &value) {
        return value ? std::to_string(*value) : std::string("<unreadable>");
    };
    return "got=" + read(sm.got()) + " stolen=" + read(sm.stolen()) + " plains=" + read(sm.plains()) +
           " (wanted 4 / 0 / 4)";
}

}  // namespace

TEST(APayloadRidesOnItsOwnEventAotTest, EachPayloadArrivesOnItsOwnEvent) {
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    sm.initialize();
    sm.processEvent(SM::Event::Finish);

    EXPECT_EQ(sm.terminalState(), SM::State::Done) << "`finish` must carry the run to `done`. " << describe(sm);
    EXPECT_EQ(sm.got(), std::optional<int64_t>(4))
        << "each payload event arrives carrying its own payload. " << describe(sm);
    EXPECT_EQ(sm.stolen(), std::optional<int64_t>(0))
        << "no data-less event arrives carrying a payload. " << describe(sm);
    EXPECT_EQ(sm.plains(), std::optional<int64_t>(4))
        << "every data-less event arrives, and arrives empty. " << describe(sm);
}

}  // namespace SCE::Tests
