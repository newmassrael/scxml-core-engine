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
                                 "`fromChild`, never saw its own `loopback`, or discarded a whole "
                                 "`<send>` because one `<param>` would not evaluate (W3C SCXML "
                                 "5.7.1 drops the pair, not the message)";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailChildPayload)
        << "`fromChild` arrived without `_event.data.value`: a `datamodel=\"null\"` child needs no "
           "script engine, but its `<send>` still has to carry the params it declares. The gate is "
           "whether this send folds to literals, not whether the machine needs an engine.";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailInternalPayload)
        << "`loopback` arrived without `_event.data.carried`: a `<send target=\"#_internal\">` must "
           "raise its params as event data, not build them and drop them at the internal-raise "
           "boundary.";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailNumberType)
        << "`typed` arrived with `_event.data.n` unequal to 7: `expr=\"7\"` is the Number 7, and a "
           "backend that stringifies on the way through delivers \"7\", which `===` finds unequal.";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailStringType)
        << "`typed` arrived with `_event.data.s` unequal to 'kept': a param that has to be "
           "EVALUATED reaches the runtime serialiser, whose string arm must emit the value rather "
           "than an engine spelling of it.";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailDuplicateParams)
        << "`typed` did not carry both values of the repeated name `d` with their types: W3C SCXML "
           "6.2 lets a name repeat and every value must be delivered.";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailNoParamError)
        << "`withBadParam` arrived with no `error.execution` before it: W3C SCXML 5.7.1 puts that "
           "error on the internal queue while the `<send>` is being evaluated, so it is dequeued "
           "first.";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailBrokenParamDelivered)
        << "`_event.data.broken` arrived as the empty string: W3C SCXML 5.7.1 says ignore the name "
           "AND the value, so a receiver must find no field at all rather than a placeholder under "
           "the name.";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailSiblingParamLost)
        << "`_event.data.kept` did not survive alongside the failed param: one `<param>` that will "
           "not evaluate costs its own pair and nothing else.";
    EXPECT_EQ(sm.getCurrentState(), SM::State::Pass);
}

}  // namespace SCE::Tests
