// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.7.1 + 6.4: what each argument of an <invoke> costs when it
// cannot be read, and what a readable one delivers — C++ AOT path.
//
// Sibling of `ABadInvokeArgumentIsReportedOnceTest.cpp` (Interpreter channel).
//
// Fixture:
// integration_resources/a_bad_invoke_argument_is_reported_once/a_bad_invoke_argument_is_reported_once.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(a_bad_invoke_argument_is_reported_once ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "a_bad_invoke_argument_is_reported_once_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::a_bad_invoke_argument_is_reported_once::a_bad_invoke_argument_is_reported_once;

std::string show(const std::optional<int64_t> &value) {
    return value ? std::to_string(*value) : std::string("<unreadable>");
}

}  // namespace

TEST(ABadInvokeArgumentIsReportedOnceAotTest, EachArgumentCostsWhatItsClauseSays) {
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    sm.initialize();
    sm.processEvent(SM::Event::Go);
    sm.processEvent(SM::Event::Finish);

    EXPECT_EQ(sm.terminalState(), SM::State::Done) << "`finish` must carry the run to `done`";
    EXPECT_EQ(sm.errors(), std::optional<int64_t>(5)) << "errors = " << show(sm.errors());
    EXPECT_EQ(sm.started(), std::optional<int64_t>(1)) << "started = " << show(sm.started());
    EXPECT_EQ(sm.fromLocOk(), std::optional<int64_t>(1)) << "fromLocOk = " << show(sm.fromLocOk());
    EXPECT_EQ(sm.emptyLocLeftOut(), std::optional<int64_t>(1)) << "emptyLocLeftOut = " << show(sm.emptyLocLeftOut());
    EXPECT_EQ(sm.brokenLeftOut(), std::optional<int64_t>(1)) << "brokenLeftOut = " << show(sm.brokenLeftOut());
}

}  // namespace SCE::Tests
