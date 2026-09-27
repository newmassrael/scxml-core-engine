// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A <send> delay is read as one CSS2 time on every engine (ARCHITECTURE.md,
// "Durations (Single Source of Truth)"). The cases live in
// tests/durations/css2_time.json. The Interpreter and the AOT engine share
// SendSchedulingHelper::parseDelayString, so one reader is held to the table
// here for both.

#include "common/SendSchedulingHelper.h"

#include <fstream>
#include <gtest/gtest.h>
#include <nlohmann/json.hpp>
#include <optional>
#include <string>

namespace {

// A floor, not an equality: adding a case must not have to touch it, but a
// table that stopped being read must not pass either.
constexpr size_t kTableFloor = 20;

TEST(Durations, ADelayIsReadAsTheOneCss2TimeEveryEngineReads) {
    std::ifstream file(SCE_DURATION_TABLE);
    ASSERT_TRUE(file.is_open()) << "cannot read " << SCE_DURATION_TABLE;
    const auto table = nlohmann::json::parse(file);
    const auto &cases = table.at("cases");
    ASSERT_GE(cases.size(), kTableFloor) << "the table lost cases";
    for (const auto &row : cases) {
        const auto name = row.at("name").get<std::string>();
        const auto text = row.at("text").get<std::string>();
        const auto got = SCE::SendSchedulingHelper::parseDelayString(text);
        if (row.at("ms").is_null()) {
            EXPECT_FALSE(got.has_value())
                << name << ": \"" << text << "\" read as " << got->count() << " ms, the table says it is not a time";
        } else {
            const auto want = row.at("ms").get<int64_t>();
            ASSERT_TRUE(got.has_value()) << name << ": \"" << text << "\" refused, the table says " << want;
            EXPECT_EQ(got->count(), want) << name << ": \"" << text << "\"";
        }
    }
}

}  // namespace
