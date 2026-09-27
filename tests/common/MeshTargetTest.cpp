// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Which <send target> values name a Mesh peer: tests/mesh/mesh_target_cases.json,
// the table the build's host_processor_analyzer::mesh_peer and the Rust and
// Kotlin routers read too. SendHelper::isMeshTarget is the C++ core's copy,
// and the one that decides whether a send takes the generated TransportRouter.

#include "common/SendHelper.h"

#include <fstream>
#include <gtest/gtest.h>
#include <nlohmann/json.hpp>
#include <string>

TEST(MeshTarget, IsMeshTargetAgreesWithTheSharedTable) {
    std::ifstream file(SCE_MESH_TARGET_TABLE);
    const auto table = nlohmann::json::parse(file);
    const auto &cases = table.at("cases");
    // A floor, not an equality: adding a case must not have to touch it, but
    // a table that stopped being read must not pass either.
    ASSERT_GE(cases.size(), 10u) << "the table was not read from " << SCE_MESH_TARGET_TABLE;
    for (const auto &row : cases) {
        const auto target = row.at("target").get<std::string>();
        EXPECT_EQ(SCE::SendHelper::isMeshTarget(target), !row.at("peer").is_null()) << target;
    }
}
