// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML C.1: an event a session sends to itself names its origin, and a
// target expression that evaluates to nothing reaches no one — C++ AOT.
//
// The delayed phase is the one this engine used to fail: a scheduled event
// lost its origin and its sendid on the way to the queue.
//
// Fixture: integration_resources/a_self_sent_event_names_its_origin/a_self_sent_event_names_its_origin.scxml
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(a_self_sent_event_names_its_origin ...)`.

#include "a_self_sent_event_names_its_origin_sm.h"
#include "common/SceClock.h"
#include "scripting/ScriptEngineProvider.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::a_self_sent_event_names_its_origin::a_self_sent_event_names_its_origin;

std::string show(const std::optional<int64_t> &value) {
    return value ? std::to_string(*value) : std::string("<unreadable>");
}

}  // namespace

TEST(ASelfSentEventNamesItsOriginAotTest, ASelfSentEventNamesItsOrigin) {
    SM sm;
    sm.setClock(std::make_shared<SCE::ManualClock>(0));
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    sm.initialize();
    sm.advanceTimeMs(1000);

    EXPECT_EQ(sm.terminalState(), SM::State::Done) << "the run must end in `done`";
    EXPECT_EQ(sm.immediateOk(), std::optional<int64_t>(1)) << "immediateOk = " << show(sm.immediateOk());
    EXPECT_EQ(sm.replied(), std::optional<int64_t>(1)) << "replied = " << show(sm.replied());
    EXPECT_EQ(sm.delayedOk(), std::optional<int64_t>(1)) << "delayedOk = " << show(sm.delayedOk());
    EXPECT_EQ(sm.unreachable(), std::optional<int64_t>(1)) << "unreachable = " << show(sm.unreachable());
    EXPECT_EQ(sm.strayed(), std::optional<int64_t>(0)) << "strayed = " << show(sm.strayed());
}

}  // namespace SCE::Tests
