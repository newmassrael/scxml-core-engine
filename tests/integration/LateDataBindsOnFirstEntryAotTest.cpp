// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.3 / Appendix D enterStates, late binding: a state's <data> is
// bound on that state's FIRST entry and never again — C++ AOT path.
//
// `s` is entered, its `v` changed to 5, `s` left and entered again; its
// <onentry> records the `v` it sees each time. Measured 2026-09-26, this
// channel bound late data on every entry, so the second entry saw 1 again.
//
// Sibling of `LateDataBindsOnFirstEntryTest.cpp` (Interpreter channel).
//
// Fixture:
// integration_resources/late_data_binds_on_first_entry/late_data_binds_on_first_entry.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(late_data_binds_on_first_entry ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "late_data_binds_on_first_entry_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::late_data_binds_on_first_entry::late_data_binds_on_first_entry;

/// What the handlers recorded (W3C SCXML 5.3 readers), for the failure message.
std::string describe(SM &sm) {
    const auto read = [](const std::optional<int64_t> &value) {
        return value ? std::to_string(*value) : std::string("<unreadable>");
    };
    return "entries=" + read(sm.entries()) + " seen=" + read(sm.seen()) + " contentSeen=" + read(sm.contentSeen()) +
           " (wanted 2 / 15 / 7)";
}

}  // namespace

TEST(LateDataBindsOnFirstEntryAotTest, AStateBindsItsDataOnlyOnItsFirstEntry) {
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    sm.initialize();
    ASSERT_EQ(sm.getCurrentState(), SM::State::Idle) << "the run has to start in `idle`";

    // Enter `s`, change its `v`, leave it, enter it again.
    sm.processEvent(SM::Event::Go);
    sm.processEvent(SM::Event::Bump);
    sm.processEvent(SM::Event::Back);
    sm.processEvent(SM::Event::Go);
    ASSERT_EQ(sm.getCurrentState(), SM::State::S)
        << "the second `go` has to leave the machine in `s`. " << describe(sm);

    EXPECT_EQ(sm.entries(), std::optional<int64_t>(2)) << "both entries of `s` must have run. " << describe(sm);
    EXPECT_EQ(sm.seen(), std::optional<int64_t>(15))
        << "`s` saw v=1 on its first entry and must see the 5 it was changed to on its second: 11 is a "
           "processor that binds late data on every entry, and no value at all one that never binds it. "
        << describe(sm);
    EXPECT_EQ(sm.contentSeen(), std::optional<int64_t>(7))
        << "`c` is bound from inline content, not an expr, and must be bound too. " << describe(sm);
}

}  // namespace SCE::Tests
