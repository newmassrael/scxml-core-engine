// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 6.4 + C.1: a <send> reaches what its target names, carries
// its payload there, and a target that names nothing reachable is reported —
// C++ AOT.
//
// Fixture:
// integration_resources/a_send_reaches_only_what_its_target_names/a_send_reaches_only_what_its_target_names.scxml
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(a_send_reaches_only_what_its_target_names ...)`.

#include "a_send_reaches_only_what_its_target_names_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>

namespace SCE::Tests {

namespace {

// The document sends to `#_parent`, so the class takes its parent's type; the
// test starts it as a host does, with none (W3C SCXML C.1).
using SM = SCE::Generated::a_send_reaches_only_what_its_target_names::a_send_reaches_only_what_its_target_names<>;

std::string show(const std::optional<int64_t> &value) {
    return value ? std::to_string(*value) : std::string("<unreadable>");
}

}  // namespace

// W3C SCXML 6.2.4: this document sends to `#_parent`, so a host that runs
// machines as roots and asks for the refusal gets it — and nothing starts. The
// plain `initialize` below runs the same machine; the refusal is opt-in.
TEST(ASendReachesOnlyWhatItsTargetNamesAotTest, ARootStartOfAMachineThatNeedsAParentIsRefused) {
    EXPECT_EQ(SM::rootStartRefusal(), ::SCE::Common::RootStartRefusal::NeedsParent);
    SM sm;
    EXPECT_EQ(sm.initializeAsRoot(), ::SCE::Common::RootStartRefusal::NeedsParent);
    EXPECT_FALSE(sm.isRunning()) << "a refused root start must start nothing";
    EXPECT_STREQ(::SCE::Common::reason(::SCE::Common::RootStartRefusal::NeedsParent),
                 "the machine sends to #_parent and was started with no parent session");
}

TEST(ASendReachesOnlyWhatItsTargetNamesAotTest, ASendReachesOnlyWhatItsTargetNames) {
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    sm.initialize();
    EXPECT_TRUE(sm.runUntilCompletion(std::chrono::seconds(3))) << "the machine never completed";

    EXPECT_EQ(sm.terminalState(), SM::State::Done) << "the run must end in `done`";
    EXPECT_EQ(sm.execErrors(), std::optional<int64_t>(1)) << "execErrors = " << show(sm.execErrors());
    EXPECT_EQ(sm.commErrors(), std::optional<int64_t>(3)) << "commErrors = " << show(sm.commErrors());
    EXPECT_EQ(sm.afterRefused(), std::optional<int64_t>(0)) << "afterRefused = " << show(sm.afterRefused());
    EXPECT_EQ(sm.afterNobody(), std::optional<int64_t>(0)) << "afterNobody = " << show(sm.afterNobody());
    EXPECT_EQ(sm.afterStranger(), std::optional<int64_t>(0)) << "afterStranger = " << show(sm.afterStranger());
    EXPECT_EQ(sm.afterOrphan(), std::optional<int64_t>(0)) << "afterOrphan = " << show(sm.afterOrphan());
    EXPECT_EQ(sm.bareArrived(), std::optional<int64_t>(1)) << "bareArrived = " << show(sm.bareArrived());
    EXPECT_EQ(sm.pongOk(), std::optional<int64_t>(1)) << "pongOk = " << show(sm.pongOk());
}

}  // namespace SCE::Tests
