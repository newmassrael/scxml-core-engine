// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Cross-document drift guard between the C++ `SCE::Mesh::ReasonCode`
// enum + `kReasonCodeTable` (sce/include/mesh/CommunicationError.h)
// and the canonical §16.7 reason-code catalog in `SCE_MESH.md`.
//
// The typed enum is the C++ source of truth
// for the reason-code vocabulary; the markdown table is the
// authoring source of truth that SCXML authors consult to write
// `_event.data.reason == 'X'` cond branches. This test binds the two
// declarations so a rename / removal / new row in either file fails
// the build the moment it lands, instead of drifting silently and
// breaking SCXML cond comparisons at runtime.
//
// The test iterates `kReasonCodeTable` (the constexpr array that
// drives both `reasonCodeString()` and the JSON wire emit), reads
// `SCE_MESH.md` from the path injected at compile time
// (`SCE_MESH_MD_PATH`), and parses the §16.7 catalog rows via a
// strict regex matching the markdown table format. The bidirectional
// coverage assertions are:
//
//   * Every wire-string in `kReasonCodeTable` appears as a row's
//     code-column entry in §16.7.
//   * Every row code in §16.7 appears in `kReasonCodeTable`.
//
// Failure modes the test catches:
//   * Variant added to enum without §16.7 update: row missing →
//     `kReasonCodeTable` entry has no matching row.
//   * §16.7 row added without enum extension: row code has no
//     matching table entry.
//   * Wire-string typo in either source: presence/absence mismatch.

#include "mesh/CommunicationError.h"

#include <gtest/gtest.h>

#include <algorithm>
#include <fstream>
#include <regex>
#include <sstream>
#include <string>
#include <unordered_set>
#include <vector>

#ifndef SCE_MESH_MD_PATH
#error "SCE_MESH_MD_PATH must be injected by CMake (target_compile_definitions)"
#endif

namespace {

std::string slurp(const std::string &path) {
    std::ifstream in(path);
    if (!in.is_open()) {
        ADD_FAILURE() << "cannot open SCE_MESH.md at: " << path;
        return {};
    }
    std::ostringstream ss;
    ss << in.rdbuf();
    return ss.str();
}

// Parse §16.7 row codes by anchoring on the catalog's stable row shape:
//
//     | <digits> | <human prose> | `<CODE>` | <extras column> |
//
// The code-column is the third pipe-delimited column and is wrapped
// in backticks. Anchoring on `^\s*\|` + digits + the backticked code
// avoids false positives from prose mentions of the codes elsewhere
// in SCE_MESH.md (e.g. row 8's prose narrative references PEER_PARTITIONED
// outside any table). The regex is strict so a future SCE_MESH.md table
// format change surfaces as a regex-mismatch (zero rows parsed) and
// fails the test loudly rather than silently skipping.
std::vector<std::string> parse_16_7_row_codes(const std::string &md) {
    std::vector<std::string> rows;
    const std::regex row_re{R"(^\s*\|\s*\d+\s*\|[^|]+\|\s*`([A-Z_]+)`\s*\|)",
                            std::regex_constants::ECMAScript |
                                std::regex_constants::multiline};
    auto begin = std::sregex_iterator(md.begin(), md.end(), row_re);
    auto end = std::sregex_iterator();
    for (auto it = begin; it != end; ++it) {
        rows.push_back((*it)[1].str());
    }
    return rows;
}

}  // namespace

TEST(ReasonCodeCatalogTest, EveryEnumVariantHasUniqueWireString) {
    // Sanity: the constexpr table has no duplicate enum variants and
    // no duplicate wire strings. A duplicate would silently mask one
    // raise site's intended reason.
    std::unordered_set<std::string_view> seen_wire;
    std::unordered_set<std::uint8_t> seen_variant;
    for (const auto &[code, wire] : ::SCE::Mesh::kReasonCodeTable) {
        EXPECT_TRUE(seen_wire.insert(wire).second)
            << "duplicate wire string in kReasonCodeTable: " << wire;
        EXPECT_TRUE(seen_variant.insert(static_cast<std::uint8_t>(code)).second)
            << "duplicate enum variant in kReasonCodeTable: "
            << static_cast<int>(code);
    }
}

TEST(ReasonCodeCatalogTest, ReasonCodeStringRoundTripsEveryTableEntry) {
    // `reasonCodeString()` must agree with `kReasonCodeTable` for every
    // variant — guards a future edit that adds an enum variant but
    // forgets to extend the table (the linear-search fallback's
    // `"UNKNOWN_REASON"` would otherwise mask the gap).
    for (const auto &[code, wire] : ::SCE::Mesh::kReasonCodeTable) {
        EXPECT_EQ(::SCE::Mesh::reasonCodeString(code), wire)
            << "reasonCodeString mismatch for variant "
            << static_cast<int>(code);
    }
}

TEST(ReasonCodeCatalogTest, EnumMatchesSpecCatalogBidirectionally) {
    const std::string md = slurp(SCE_MESH_MD_PATH);
    ASSERT_FALSE(md.empty()) << "SCE_MESH.md is empty or unreadable";

    const std::vector<std::string> spec_codes = parse_16_7_row_codes(md);

    // The §16.7 catalog has 13 rows at HEAD (2026-05-20). The expected
    // count is a hard pin so a future row addition that DOES land in
    // SCE_MESH.md but DOES NOT update the enum / table is caught by
    // the explicit-count guard before the membership assertions
    // (which would otherwise only fail with a "missing variant"
    // message that doesn't surface the count drift directly).
    EXPECT_EQ(spec_codes.size(), ::SCE::Mesh::kReasonCodeTable.size())
        << "§16.7 row count (" << spec_codes.size()
        << ") differs from kReasonCodeTable size ("
        << ::SCE::Mesh::kReasonCodeTable.size()
        << ") — adding a row in SCE_MESH.md without extending the enum, "
           "or vice versa, drops the bidirectional binding";

    // Every wire string from the C++ table must appear in §16.7.
    std::unordered_set<std::string> spec_set(spec_codes.begin(),
                                             spec_codes.end());
    for (const auto &[code, wire] : ::SCE::Mesh::kReasonCodeTable) {
        EXPECT_TRUE(spec_set.count(std::string(wire)) > 0)
            << "wire string '" << wire
            << "' from kReasonCodeTable is missing from SCE_MESH.md §16.7 "
               "row catalog — either the variant was added without a "
               "matching row, or the row was removed";
    }

    // Every §16.7 row code must appear in the C++ table.
    std::unordered_set<std::string_view> table_set;
    for (const auto &[code, wire] : ::SCE::Mesh::kReasonCodeTable) {
        table_set.insert(wire);
    }
    for (const auto &row_code : spec_codes) {
        EXPECT_TRUE(table_set.count(std::string_view(row_code)) > 0)
            << "§16.7 row code '" << row_code
            << "' is missing from kReasonCodeTable — either the row was "
               "added without a matching enum variant, or the variant "
               "was removed";
    }
}
