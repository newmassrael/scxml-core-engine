// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: cancelling an invocation raises no event in the invoking
// session — C++ AOT path.
//
// Measured 2026-09-26, this channel raised an internal `cancel.invoke` each
// time a still-running child was cancelled, in every document with an scxml
// `<invoke>` — the analyzer listed the event as a platform event there — and
// any transition matching it (`cancel.invoke`, `cancel.*`, `*`) took it.
//
// Sibling of `CancellingAnInvokeRaisesNothingTest.cpp` (Interpreter channel).
//
// Fixture:
// integration_resources/cancelling_an_invoke_raises_nothing/cancelling_an_invoke_raises_nothing.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(cancelling_an_invoke_raises_nothing ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "cancelling_an_invoke_raises_nothing_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::cancelling_an_invoke_raises_nothing::cancelling_an_invoke_raises_nothing;

}  // namespace

TEST(CancellingAnInvokeRaisesNothingAotTest, LeavingTheInvokingStateRaisesNoCancelEvent) {
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    sm.initialize();
    ASSERT_EQ(sm.getCurrentState(), SM::State::P) << "the run has to start in `p`, with its child invoked";

    sm.processEvent(SM::Event::Leave);
    sm.processEvent(SM::Event::Finish);

    EXPECT_EQ(sm.terminalState(), SM::State::Done) << "`finish` must carry the run to `done`";
    EXPECT_EQ(sm.spurious(), std::optional<int64_t>(0))
        << "leaving `p` cancelled its child, and a `cancel.invoke` event reached the invoking session; "
           "W3C SCXML 6.4 defines no such event";
}

}  // namespace SCE::Tests
