// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D exitInterpreter: a run ends by exiting every state it
// is still in, the way exitStates exits one — C++ AOT path.
//
// Reached two ways, and both are driven here: the run enters a top-level
// `<final>` after a step, or the host stops it. Measured 2026-09-26, this
// channel ran the final's `<onexit>` only when the run ended during
// `initialize()`, kept the final in the configuration afterwards, and ran no
// `<onexit>` at all on stop().
//
// Sibling of `TheRunEndsByExitingEveryStateTest.cpp` (Interpreter channel).
//
// Fixture:
// integration_resources/the_run_ends_by_exiting_every_state/the_run_ends_by_exiting_every_state.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(the_run_ends_by_exiting_every_state ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "scripting/ScriptEngineProvider.h"
#include "the_run_ends_by_exiting_every_state_sm.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::the_run_ends_by_exiting_every_state::the_run_ends_by_exiting_every_state;

std::unique_ptr<SM> started() {
    auto sm = std::make_unique<SM>();
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm->setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                  [](::SCE::IScriptEngine *) {}));
    }
    sm->initialize();
    return sm;
}

/// What the run left behind (W3C SCXML 5.3 readers), printed together so a
/// failure says what happened and not only which clause broke.
std::string describe(SM &sm) {
    const auto read = [](const std::optional<int64_t> &value) {
        return value ? std::to_string(*value) : std::string("<unreadable>");
    };
    return "active states: " + std::to_string(sm.getActiveStates().size()) + ", ended in " +
           (sm.terminalState() ? std::string(sm.getPolicy().getStateName(*sm.terminalState())) : "<none>") +
           ", running=" + (sm.isRunning() ? "true" : "false") + ", order=" + read(sm.order()) +
           " finalExits=" + read(sm.finalExits()) + " selfInFinal=" + read(sm.selfInFinal());
}

}  // namespace

/// The run ends in a top-level `<final>` after a step: the final is exited too.
TEST(TheRunEndsByExitingEveryStateAotTest, ARunThatReachesItsFinalExitsTheFinal) {
    auto sm = started();
    ASSERT_EQ(sm->getCurrentState(), SM::State::Inner) << "the run has to start inside `inner`";

    sm->processEvent(SM::Event::Finish);

    EXPECT_EQ(sm->terminalState(), SM::State::Done) << describe(*sm);
    EXPECT_FALSE(sm->isRunning()) << describe(*sm);
    EXPECT_TRUE(sm->getActiveStates().empty())
        << "exitInterpreter deletes every state it exits, the final included. " << describe(*sm);
    EXPECT_EQ(sm->finalExits(), std::optional<int64_t>(1))
        << "the final's own <onexit> must run exactly once as the run ends. " << describe(*sm);
    EXPECT_EQ(sm->selfInFinal(), std::optional<int64_t>(1))
        << "`done` must still be in the configuration during its own <onexit>. " << describe(*sm);
    EXPECT_EQ(sm->order(), std::optional<int64_t>(12)) << "`finish` exits `inner` then `outer`. " << describe(*sm);
}

/// The host stops a run that has not ended: every state is exited, innermost first.
TEST(TheRunEndsByExitingEveryStateAotTest, AStoppedRunExitsEveryStateInnermostFirst) {
    auto sm = started();
    ASSERT_EQ(sm->getCurrentState(), SM::State::Inner) << "the run has to start inside `inner`";

    sm->stop();

    EXPECT_FALSE(sm->terminalState().has_value()) << "a stopped run did not end in a final. " << describe(*sm);
    EXPECT_FALSE(sm->isRunning()) << describe(*sm);
    EXPECT_TRUE(sm->getActiveStates().empty()) << describe(*sm);
    EXPECT_EQ(sm->order(), std::optional<int64_t>(12))
        << "stop() must run `inner`'s <onexit> and then `outer`'s: 0 is a stop that exited nothing, "
           "21 one that exited in document order instead of exit order. "
        << describe(*sm);
    EXPECT_EQ(sm->finalExits(), std::optional<int64_t>(0)) << "the run never entered `done`. " << describe(*sm);
}

}  // namespace SCE::Tests
