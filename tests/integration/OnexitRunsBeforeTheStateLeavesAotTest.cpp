// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D exitStates: a state leaves the configuration AFTER its
// own `<onexit>` has run — C++ AOT path.
//
// The procedure is onexit, then cancelInvoke, then configuration.delete(s),
// for each state in exitOrder. Measured 2026-09-26, this channel did the
// reverse: its generated exit removed the state from `activeStates_` first,
// then cancelled the state's invocations, and ran the `<onexit>` last, so
// `In(s)` inside s's own handler answered false.
//
// Sibling of `OnexitRunsBeforeTheStateLeavesTest.cpp` (Interpreter channel).
//
// Fixture:
// integration_resources/onexit_runs_before_the_state_leaves/onexit_runs_before_the_state_leaves.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(onexit_runs_before_the_state_leaves ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "onexit_runs_before_the_state_leaves_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::onexit_runs_before_the_state_leaves::onexit_runs_before_the_state_leaves;

/// The final the verdict landed in, and what the handlers recorded (W3C SCXML
/// 5.3 readers). The document has no `<parallel>`, so the current state is the
/// configuration's only leaf; the records are what say which observation was
/// wrong, the same pair the Interpreter and C11 channels print.
std::string describe(SM &sm) {
    const auto read = [](const std::optional<int64_t> &value) {
        return value ? std::to_string(*value) : std::string("<unreadable>");
    };
    return std::string(sm.getPolicy().getStateName(sm.getCurrentState())) + "  (selfInInner=" + read(sm.selfInInner()) +
           " parentInInner=" + read(sm.parentInInner()) + " selfInOuter=" + read(sm.selfInOuter()) +
           " childInOuter=" + read(sm.childInOuter()) + " exits=" + read(sm.exits()) + "; wanted 1 / 1 / 1 / 0 / 2)";
}

}  // namespace

TEST(OnexitRunsBeforeTheStateLeavesAotTest, AStateIsStillActiveWhileItsOwnOnexitRuns) {
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    sm.initialize();
    ASSERT_EQ(sm.getCurrentState(), SM::State::Inner)
        << "the run has to start inside `inner`. active: " << describe(sm);

    sm.processEvent(SM::Event::Leave);

    // The document checks its clauses in document order and lands each in a
    // `<final>` of its own, so the final reached names which one broke.
    EXPECT_EQ(sm.getCurrentState(), SM::State::Settled)
        << "`leave` did not carry the machine to `settled`: `failExits` (a handler did not "
           "run), `failSelfInInner` / `failSelfInOuter` (a state was already out of the "
           "configuration during its own `<onexit>`), `failParentInInner` (the parent left "
           "before its child's `<onexit>`), `failChildInOuter` (the child was still active "
           "during its parent's `<onexit>`). active: "
        << describe(sm);
}

}  // namespace SCE::Tests
