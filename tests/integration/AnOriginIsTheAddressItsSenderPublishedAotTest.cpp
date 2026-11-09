// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1 + C.1 (docs/adr/0090, test 1): the origin of an event the
// SCXML Event I/O Processor delivers is the address its sender published, on a
// self-send, a child's send to its parent and a parent's send to its child, and
// a reply to it arrives — C++ AOT.
//
// The generated engine writes the pair where it builds the event
// (`StaticExecutionEngine::sentByASession`, `forwardedByASession`, the policy's
// `deliverToParent`), the build resolves `type="scxml"` to the processor's URI,
// and a child reaches its parent at the parent's published location.
//
// Fixture:
// integration_resources/an_origin_is_the_address_its_sender_published/an_origin_is_the_address_its_sender_published.scxml
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(an_origin_is_the_address_its_sender_published ...)`.

#include "an_origin_is_the_address_its_sender_published_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::an_origin_is_the_address_its_sender_published::an_origin_is_the_address_its_sender_published;

std::string show(const std::optional<int64_t> &value) {
    return value ? std::to_string(*value) : std::string("<unreadable>");
}

}  // namespace

TEST(AnOriginIsTheAddressItsSenderPublishedAotTest, AnOriginIsTheAddressItsSenderPublished) {
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    EXPECT_EQ(sm.initialize(), std::nullopt);
    const bool reachedFinal = sm.runUntilCompletion(std::chrono::seconds(3));

    EXPECT_TRUE(reachedFinal) << "the run did not reach a final state; it waits where a reply reached nobody";
    EXPECT_EQ(sm.terminalState(), SM::State::Done)
        << "the run must end in `done`; a reply sent to an origin that reached nobody leaves it waiting";
    EXPECT_EQ(sm.selfOk(), std::optional<int64_t>(1)) << "selfOk = " << show(sm.selfOk());
    EXPECT_EQ(sm.selfReplied(), std::optional<int64_t>(1)) << "selfReplied = " << show(sm.selfReplied());
    EXPECT_EQ(sm.childOk(), std::optional<int64_t>(1)) << "childOk = " << show(sm.childOk());
    EXPECT_EQ(sm.childReplied(), std::optional<int64_t>(1)) << "childReplied = " << show(sm.childReplied());
    EXPECT_EQ(sm.parentOk(), std::optional<int64_t>(1)) << "parentOk = " << show(sm.parentOk());
    EXPECT_EQ(sm.parentReplied(), std::optional<int64_t>(1)) << "parentReplied = " << show(sm.parentReplied());
}

}  // namespace SCE::Tests
