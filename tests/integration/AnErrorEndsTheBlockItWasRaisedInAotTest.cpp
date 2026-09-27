// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 4.9: an error ends the block it was raised in — whichever element
// raised it — and no other block — C++ AOT path.
//
// Sibling of `AnErrorEndsTheBlockItWasRaisedInTest.cpp` (Interpreter channel).
//
// Fixture:
// integration_resources/an_error_ends_the_block_it_was_raised_in/an_error_ends_the_block_it_was_raised_in.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(an_error_ends_the_block_it_was_raised_in ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "an_error_ends_the_block_it_was_raised_in_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::an_error_ends_the_block_it_was_raised_in::an_error_ends_the_block_it_was_raised_in;

std::string show(const std::optional<int64_t> &value) {
    return value ? std::to_string(*value) : std::string("<unreadable>");
}

}  // namespace

TEST(AnErrorEndsTheBlockItWasRaisedInAotTest, EachErrorEndsOnlyItsOwnBlock) {
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    sm.initialize();
    sm.processEvent(SM::Event::T);
    sm.processEvent(SM::Event::Finish);

    EXPECT_EQ(sm.terminalState(), SM::State::Done) << "`finish` must carry the run to `done`";
    EXPECT_EQ(sm.errors(), std::optional<int64_t>(7)) << "errors = " << show(sm.errors());
    EXPECT_EQ(sm.afterAssign(), std::optional<int64_t>(0)) << "afterAssign = " << show(sm.afterAssign());
    EXPECT_EQ(sm.afterScript(), std::optional<int64_t>(0)) << "afterScript = " << show(sm.afterScript());
    EXPECT_EQ(sm.afterLog(), std::optional<int64_t>(0)) << "afterLog = " << show(sm.afterLog());
    EXPECT_EQ(sm.afterCancel(), std::optional<int64_t>(0)) << "afterCancel = " << show(sm.afterCancel());
    EXPECT_EQ(sm.afterIfInner(), std::optional<int64_t>(0)) << "afterIfInner = " << show(sm.afterIfInner());
    EXPECT_EQ(sm.afterIf(), std::optional<int64_t>(0)) << "afterIf = " << show(sm.afterIf());
    EXPECT_EQ(sm.afterSingle(), std::optional<int64_t>(0)) << "afterSingle = " << show(sm.afterSingle());
    EXPECT_EQ(sm.afterTrans(), std::optional<int64_t>(0)) << "afterTrans = " << show(sm.afterTrans());
    EXPECT_EQ(sm.initRan(), std::optional<int64_t>(1)) << "initRan = " << show(sm.initRan());
    EXPECT_EQ(sm.pairs(), std::optional<int64_t>(4)) << "pairs = " << show(sm.pairs());
    EXPECT_EQ(sm.sum(), std::optional<int64_t>(90)) << "sum = " << show(sm.sum());
}

}  // namespace SCE::Tests
