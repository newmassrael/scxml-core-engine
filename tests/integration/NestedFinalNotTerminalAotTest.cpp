// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.7: only a top-level `<final>` ends the session — C++ AOT path.
//
// Appendix D `enterStates` sets `running = false` for a `<final>` only when
// `isSCXMLElement(s.parent)`; otherwise it queues `done.state.<parent>` and the
// machine carries on. `StatePolicy::isFinalState` is the structural question —
// "is this state a `<final>` element" — while the engine's completion
// predicate answers "has this session ended", and only the latter may gate
// `runUntilCompletion`, the completion callback, and the `done.invoke.<id>` a
// parent emits for this machine.
//
// The fixture rests in the nested final rather than passing through it: a
// machine that continues within the same macrostep is only ever sampled at the
// end, where a right and a wrong predicate agree.
//
// Sibling of `NestedFinalNotTerminalTest.cpp` (Interpreter channel).
//
// Fixture: integration_resources/nested_final_not_terminal/nested_final_not_terminal.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(nested_final_not_terminal ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "nested_final_not_terminal_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

TEST(NestedFinalNotTerminalAotTest, ANestedFinalDoesNotEndTheSession) {
    using SM = SCE::Generated::nested_final_not_terminal::nested_final_not_terminal;

    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    sm.initialize();

    ASSERT_EQ(sm.getCurrentState(), SM::State::PhaseDone)
        << "the fixture is supposed to come to rest in the nested `<final>`; it did not, "
           "so nothing below is testing what it claims";
    EXPECT_FALSE(sm.isInFinalState())
        << "the engine reported completion while resting in `phaseDone`, a `<final>` nested "
           "inside `phase`. W3C SCXML Appendix D `enterStates` ends the session only when the "
           "final's parent is the `<scxml>` element — a nested one finishes its compound state "
           "and queues `done.state.phase`, leaving the machine live.";

    sm.raiseExternal(SM::Event::Resume);
    const bool completed = sm.runUntilCompletion(std::chrono::seconds(3));

    EXPECT_TRUE(completed) << "the machine did not complete after `resume`";
    EXPECT_EQ(sm.getCurrentState(), SM::State::Pass)
        << "`resume` did not carry the machine out of the nested final to the top-level one";
}

}  // namespace SCE::Tests
