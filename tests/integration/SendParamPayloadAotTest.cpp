// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 `<send>` `<param>` payload delivery — C++ AOT.
//
// Two send paths that were fixed at the template layer with no runtime
// witness, because no committed fixture had a machine of the required
// shape. The suites could only show that nothing regressed; that same
// absence is why the defects survived as long as they did.
//
//   engine-less child -> parent   param emission was gated on the
//     *machine* needing a script engine rather than on the send needing
//     one, so a `datamodel="null"` child shipped its `<send>` with the
//     params dropped. C++ made the per-send judgement all along, which is
//     what the other backends were brought up to; this pins that it keeps
//     doing so.
//
//   #_internal                    the internal raise took no event data,
//     so params were built and then discarded.
//
// The two reach distinct final states, so a failure names the path.
//
// Fixture: integration_resources/send_param_payload/send_param_payload.scxml
// (canonical, shared with the Rust / Go / Kotlin / Python / C11 channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(send_param_payload ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "scripting/ScriptEngineProvider.h"
#include "send_param_payload_sm.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

TEST(SendParamPayloadAotTest, SendParamsReachEventDataFromChildAndInternalQueue) {
    using SM = SCE::Generated::send_param_payload::send_param_payload;

    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        // Aliasing constructor + no-op deleter — engine lifetime is owned by
        // the ScriptEngineProvider singleton; the shared_ptr is a non-owning
        // view. Mirrors SimpleAotTest's W3C-AOT pattern.
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    sm.initialize();
    const bool reachedFinal = sm.runUntilCompletion(std::chrono::seconds(3));

    EXPECT_TRUE(reachedFinal) << "parent did not reach a final state within timeout — it never saw "
                                 "`fromChild` or never saw its own `loopback`";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailChildPayload)
        << "`fromChild` arrived without `_event.data.value`: a `datamodel=\"null\"` child needs no "
           "script engine, but its `<send>` still has to carry the params it declares. The gate is "
           "whether this send folds to literals, not whether the machine needs an engine.";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailInternalPayload)
        << "`loopback` arrived without `_event.data.carried`: a `<send target=\"#_internal\">` must "
           "raise its params as event data, not build them and drop them at the internal-raise "
           "boundary.";
    EXPECT_EQ(sm.getCurrentState(), SM::State::Pass);
}

}  // namespace SCE::Tests
