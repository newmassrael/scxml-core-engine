// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.4.3: the value a `srcexpr` computes is the child that runs — C++
// AOT path, over the candidate set `sce:candidates` declares.
//
// The sibling fixture asks whether the expression is evaluated at all. This
// asks whether its value means anything, which on the AOT path it did not
// until candidates existed: the child was fixed at build time and one stub
// answered whatever was computed (docs/SCE_ACCEPTED_SUBSET.md §2.13).
//
// The candidates announce themselves, so the outcomes are distinct: `chosen`
// reaches `pass`, `other` reaches `wrongChild`, a failure to load reaches
// `noChild`, and a stub reaches none of them and parks in `probe`.
//
// Sibling of `InvokeCandidateSelectsTheChildTest.cpp` (Interpreter channel),
// which reaches the same answer by loading the named document at run time —
// the behaviour this path now matches rather than approximates.
//
// Fixture: integration_resources/invoke_candidate_selects_the_child/invoke_candidate_selects_the_child.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(invoke_candidate_selects_the_child ...)`.

#include "invoke_candidate_selects_the_child_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

TEST(InvokeCandidateSelectsTheChildAotTest, TheEvaluatedValueSelectsWhichCandidateRuns) {
    using SM = SCE::Generated::invoke_candidate_selects_the_child::invoke_candidate_selects_the_child;

    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    sm.initialize();
    const bool completed = sm.runUntilCompletion(std::chrono::seconds(3));

    EXPECT_TRUE(completed) << "the machine never completed. Parking means no child spoke: a stub "
                              "ran, or nothing did.";
    EXPECT_EQ(sm.getCurrentState(), SM::State::Pass)
        << "`WrongChild` means the value selected the other candidate; `NoChild` means "
           "nothing was loaded at all";
}

}  // namespace SCE::Tests
