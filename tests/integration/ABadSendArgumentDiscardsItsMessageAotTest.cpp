// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 4.9: a <send> whose own argument cannot be evaluated raises
// error.execution, discards the message, and ends its block — C++ AOT path.
//
// Sibling of `ABadSendArgumentDiscardsItsMessageTest.cpp` (Interpreter channel).
//
// Fixture:
// integration_resources/a_bad_send_argument_discards_its_message/a_bad_send_argument_discards_its_message.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(a_bad_send_argument_discards_its_message ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "a_bad_send_argument_discards_its_message_sm.h"
#include "common/SceClock.h"
#include "scripting/ScriptEngineProvider.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::a_bad_send_argument_discards_its_message::a_bad_send_argument_discards_its_message;

std::string show(const std::optional<int64_t> &value) {
    return value ? std::to_string(*value) : std::string("<unreadable>");
}

}  // namespace

// On a manual clock advanced a full minute before `finish`: a channel that
// scheduled the message whose `delayexpr` failed, under some default delay,
// delivers it within that minute and moves `sent`.
TEST(ABadSendArgumentDiscardsItsMessageAotTest, EachBadArgumentDiscardsItsMessage) {
    SM sm;
    sm.setClock(std::make_shared<SCE::ManualClock>(0));
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    sm.initialize();
    sm.advanceTimeMs(60000);
    sm.processEvent(SM::Event::Finish);

    EXPECT_EQ(sm.terminalState(), SM::State::Done) << "`finish` must carry the run to `done`";
    EXPECT_EQ(sm.errors(), std::optional<int64_t>(6)) << "errors = " << show(sm.errors());
    EXPECT_EQ(sm.sent(), std::optional<int64_t>(0)) << "sent = " << show(sm.sent());
    EXPECT_EQ(sm.after(), std::optional<int64_t>(0)) << "after = " << show(sm.after());
}

}  // namespace SCE::Tests
