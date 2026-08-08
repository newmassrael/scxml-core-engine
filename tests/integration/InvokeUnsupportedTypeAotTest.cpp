// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.4.1: an `<invoke>` naming a processor the platform does not
// support places `error.execution` on the internal event queue — C++ AOT path.
//
// The AOT failure mode differs from the Interpreter's. The Interpreter
// substituted the SCXML handler for an unknown type and started a child
// session the author never asked for; AOT dropped the `<invoke>` from the
// model outright, so the machine had no observable at all where the
// Interpreter had the wrong one. Codegen therefore carries the unsupported
// invoke as its own model variant and lowers the raise, deferred to
// macrostep end so §scxml-6.4 ordering holds and an early state exit cancels
// it like any other pending invoke.
//
// A backend that renders this fixture without the raise reproduces the drop
// one layer down, and the machine rests in `probe` instead of reaching
// `pass` — which is what this test measures.
//
// Sibling of `InvokeUnsupportedTypeTest.cpp` (Interpreter channel).
//
// Fixture: integration_resources/invoke_unsupported_type/invoke_unsupported_type.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(invoke_unsupported_type ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "invoke_unsupported_type_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

TEST(InvokeUnsupportedTypeAotTest, UnsupportedTypeRaisesErrorExecutionOnTheInternalQueue) {
    using SM = SCE::Generated::invoke_unsupported_type::invoke_unsupported_type;

    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    sm.initialize();
    const bool completed = sm.runUntilCompletion(std::chrono::seconds(3));

    EXPECT_TRUE(completed) << "the machine never completed. §scxml-6.4.1 requires an `<invoke>` whose `type` "
                              "names no supported processor to place `error.execution` on the internal queue; "
                              "parking in `probe` means the `<invoke>` was dropped rather than lowered.";
    EXPECT_EQ(sm.getCurrentState(), SM::State::Pass)
        << "the machine completed somewhere other than the `error.execution` target";
}

}  // namespace SCE::Tests
