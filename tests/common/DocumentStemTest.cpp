// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The value a hybrid <invoke srcexpr> computes is reduced to the stem of the
// document it names (docs/SCE_ACCEPTED_SUBSET.md §2.13), and every engine
// reduces it the same way. The cases live in
// tests/document_stem/document_stem.json. The Interpreter and the AOT engine
// share SCE::documentStem, so one reader is held to the table here for both.

#include "common/DocumentStem.h"

#include <fstream>
#include <gtest/gtest.h>
#include <nlohmann/json.hpp>
#include <string>

namespace {

// A floor, not an equality: adding a case must not have to touch it, but a
// table that stopped being read must not pass either.
constexpr size_t kTableFloor = 15;

TEST(DocumentStem, AValueIsReducedToTheOneStemEveryEngineReducesItTo) {
    std::ifstream file(SCE_DOCUMENT_STEM_TABLE);
    ASSERT_TRUE(file.is_open()) << "cannot read " << SCE_DOCUMENT_STEM_TABLE;
    const auto table = nlohmann::json::parse(file);
    const auto &cases = table.at("cases");
    ASSERT_GE(cases.size(), kTableFloor) << "the table lost cases";
    for (const auto &row : cases) {
        const auto name = row.at("name").get<std::string>();
        const auto value = row.at("value").get<std::string>();
        const auto stem = row.at("stem").get<std::string>();
        EXPECT_EQ(SCE::documentStem(value), stem) << name << ": \"" << value << "\"";
    }
}

}  // namespace
