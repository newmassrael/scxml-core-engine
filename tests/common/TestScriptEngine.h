// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include "scripting/IScriptEngine.h"
#include "scripting/ScriptEngineProvider.h"

#include <cstdio>
#include <cstdlib>
#include <memory>

namespace SCE::Test {

// Start a generated AOT state machine a test program expects to run. A start
// the machine refuses — its script session could not be established
// (docs/adr/0096) — ends the program, naming the step that failed: a program
// that ran on would drive a machine that never started and report whatever
// that machine did not do instead. For the test programs that have no test
// framework to report through; a gtest test asks
// `EXPECT_EQ(sm.initialize(), std::nullopt)`.
template <typename SM> inline void start_or_abort(SM &sm) {
    if (const auto refused = sm.initialize()) {
        std::fprintf(stderr, "FAIL: the machine did not start: %s\n", refused->describe().c_str());
        std::fflush(stderr);
        std::abort();
    }
}

// Inject the build-configured script engine into a generated AOT state
// machine before initialize(). Compiles to no-op when the state machine
// does not need a script engine (the generated Policy advertises this
// via NEEDS_SCRIPT_ENGINE). Lifetime: aliasing shared_ptr with a no-op
// deleter — the engine is owned by ScriptEngineProvider, this view is
// just the handle the generated emit sites require.
template <typename SM> inline void inject_build_engine(SM &sm) {
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<SCE::IScriptEngine>(&SCE::ScriptEngineProvider::getScriptEngine(),
                                                               [](SCE::IScriptEngine *) {}));
    }
}

}  // namespace SCE::Test
