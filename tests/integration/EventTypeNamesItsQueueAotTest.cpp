// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1: `_event.type` names the queue an event was taken from —
// C++ AOT path.
//
// The document queues an external event first and two internal ones after
// it. Measured 2026-09-26, this channel set an "external" flag when an event
// was ENQUEUED and consumed it on whichever event was bound next, so `int`
// was typed "external" and `ext` "internal".
//
// Sibling of `EventTypeNamesItsQueueTest.cpp` (Interpreter channel).
//
// Fixture:
// integration_resources/event_type_names_its_queue/event_type_names_its_queue.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(event_type_names_its_queue ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "event_type_names_its_queue_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::event_type_names_its_queue::event_type_names_its_queue;

/// What the handlers recorded (W3C SCXML 5.3 readers), for the failure message.
std::string describe(SM &sm) {
    const auto read = [](const std::optional<int64_t> &value) {
        return value ? std::to_string(*value) : std::string("<unreadable>");
    };
    return "intCode=" + read(sm.intCode()) + " sendCode=" + read(sm.sendCode()) + " extCode=" + read(sm.extCode()) +
           " (1 internal, 2 external, 3 other; wanted 1 / 1 / 2)";
}

}  // namespace

TEST(EventTypeNamesItsQueueAotTest, EachEventIsTypedByTheQueueItWasTakenFrom) {
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    // The document queues its own events; the run needs nothing from the host.
    sm.initialize();

    EXPECT_EQ(sm.terminalState(), SM::State::Done) << "`ext` must carry the run to `done`. " << describe(sm);
    EXPECT_EQ(sm.intCode(), std::optional<int64_t>(1))
        << "`int` came off the internal queue while `ext` waited on the external one. " << describe(sm);
    EXPECT_EQ(sm.sendCode(), std::optional<int64_t>(1))
        << "a `<send target=\"#_internal\">` with a payload rides the internal queue too. " << describe(sm);
    EXPECT_EQ(sm.extCode(), std::optional<int64_t>(2)) << "`ext` came off the external queue. " << describe(sm);
}

}  // namespace SCE::Tests
