// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Unit-test family for the C++ PositionMap primitive.
// Mirrors the test coverage in `sce-build/src/position_map.rs::tests`:
// identity mapping, multi-line lookup, Unicode-scalar column counting,
// splice composition via `append_mapped_substring`, predicate shape
// for `is_identity()`. Every public method must be exercised by this
// file so the header carries no dead code; production consumers are
// `processXInclude` / `processSceTemplate`.

#include "parsing/PositionMap.h"

#include <gtest/gtest.h>

#include <filesystem>
#include <string>
#include <variant>

using SCE::parsing::CallSiteOrigin;
using SCE::parsing::FileOrigin;
using SCE::parsing::Origin;
using SCE::parsing::PositionMap;
using SCE::parsing::rowcol_to_offset;
using SCE::parsing::SourcePos;

// ── Test 1: Identity map round-trip + EOF clamp ────────────────────
// Mirrors Rust `identity_single_line_ascii` +
// `identity_eof_is_clamped_not_panicking`. An identity-constructed map
// over `<root/>` must translate byte offset 0 to (row 1, col 1) and
// byte offset text.size() + 1 to (row 1, col text.size() + 1) without
// asserting — the clamp choice matches roxmltree's `text_pos_at`
// behaviour at EOF.
TEST(PositionMap, IdentityRoundtripLookupAtZeroAndEof) {
    const std::string text = "<root/>";
    auto map = PositionMap::identity("main.scxml", text);
    EXPECT_TRUE(map.is_identity());

    SourcePos pos = map.lookup(0);
    EXPECT_EQ(pos.file, std::filesystem::path("main.scxml"));
    EXPECT_EQ(pos.row, 1u);
    EXPECT_EQ(pos.col, 1u);

    // Mid-line offset (byte 3 is the 'o' of "<root/>") — 1 + 3 = col 4.
    pos = map.lookup(3);
    EXPECT_EQ(pos.row, 1u);
    EXPECT_EQ(pos.col, 4u);

    // One past end: same semantics as Rust test — clamp to last
    // entry via `entry_for`, then `offset_to_rowcol`'s own clamp.
    pos = map.lookup(text.size() + 1);
    EXPECT_EQ(pos.row, 1u);
    EXPECT_EQ(pos.col, static_cast<uint32_t>(text.size()) + 1u);
}

// ── Test 2: Multi-line lookup ──────────────────────────────────────
// Mirrors Rust `identity_multi_line_ascii`. The lookup must walk the
// line-starts table to find the correct row, then count Unicode
// scalars on the matched line to find the column.
TEST(PositionMap, MultiLineFileLookup) {
    const std::string text = "<root>\n  <state id=\"s1\"/>\n</root>";
    auto map = PositionMap::identity("main.scxml", text);

    // `<state` sits after "<root>\n  " — row 2, col 3.
    const size_t state_offset = text.find("<state");
    ASSERT_NE(state_offset, std::string::npos);
    SourcePos pos = map.lookup(state_offset);
    EXPECT_EQ(pos.row, 2u);
    EXPECT_EQ(pos.col, 3u);

    // `</root>` sits at the start of line 3.
    const size_t close_offset = text.find("</root>");
    ASSERT_NE(close_offset, std::string::npos);
    pos = map.lookup(close_offset);
    EXPECT_EQ(pos.row, 3u);
    EXPECT_EQ(pos.col, 1u);
}

// ── Test 3: Unicode scalar column counting ─────────────────────────
// Mirrors Rust `column_counts_unicode_scalars_not_bytes`. "한" is 3
// UTF-8 bytes but a single Unicode scalar — column 2 must point at
// the byte *after* "한", not at byte 3. Byte counting would report
// col 4 and silently skew every multibyte diagnostic.
TEST(PositionMap, UnicodeScalarColumn) {
    const std::string text = "\xED\x95\x9C" "abc";  // "한abc"
    auto map = PositionMap::identity("main.scxml", text);

    const size_t byte_offset = 3;  // Length of "한" in UTF-8.
    SourcePos pos = map.lookup(byte_offset);
    EXPECT_EQ(pos.row, 1u);
    EXPECT_EQ(pos.col, 2u);

    // And rowcol_to_offset round-trips: (1, 2) → byte 3.
    EXPECT_EQ(rowcol_to_offset(text, 1, 2), 3u);
    EXPECT_EQ(rowcol_to_offset(text, 1, 3), 4u);
}

// ── Test 4: Single append_mapped_substring (File origin) ───────────
// Mirrors Rust `append_mapped_substring_identity_sub_composes_into_outer`.
// Splices inner.xml bytes [2, 5) into the outer at offset 3, so
// expanded byte 3 resolves to inner.xml (row 1, col 3).
TEST(PositionMap, SingleAppendMappedSubstringFileOrigin) {
    const std::string inner_text = "abcdef";
    auto inner = PositionMap::identity(std::filesystem::path("inner.xml"), inner_text);

    PositionMap outer;
    outer.register_file(std::filesystem::path("outer.xml"), "out");
    outer.push_entry(0, 3, FileOrigin{std::filesystem::path("outer.xml"), 0});
    outer.append_mapped_substring(inner, 2, 5, 3);

    // Expanded byte 0 → outer.xml (1, 1).
    SourcePos pos = outer.lookup(0);
    EXPECT_EQ(pos.file, std::filesystem::path("outer.xml"));
    EXPECT_EQ(pos.row, 1u);
    EXPECT_EQ(pos.col, 1u);

    // Expanded byte 3 → inner.xml[2] = 'c' → (1, 3).
    pos = outer.lookup(3);
    EXPECT_EQ(pos.file, std::filesystem::path("inner.xml"));
    EXPECT_EQ(pos.row, 1u);
    EXPECT_EQ(pos.col, 3u);

    // Expanded byte 5 → inner.xml[4] = 'e' → (1, 5).
    pos = outer.lookup(5);
    EXPECT_EQ(pos.file, std::filesystem::path("inner.xml"));
    EXPECT_EQ(pos.row, 1u);
    EXPECT_EQ(pos.col, 5u);
}

// ── Test 5: Composition with File + CallSite origins ───────────────
// A three-entry outer map mixing a FileOrigin prefix, a
// CallSite-origin substitution region (the depth-1 collapse used by
// `sce:template`'s `{$param}` substitution), and a FileOrigin suffix.
// The CallSite region must resolve to the caller's stored (row, col)
// regardless of the offset inside the region.
TEST(PositionMap, CompositionFileAndCallSite) {
    PositionMap outer;
    outer.register_file(std::filesystem::path("caller.scxml"), "<a/>");
    outer.register_file(std::filesystem::path("template.scxml"),
                        "body-before\nbody-after");

    // [0, 6): body-before bytes from template.scxml[0..6]
    //         = "body-b" → template row 1.
    outer.push_entry(0, 6, FileOrigin{std::filesystem::path("template.scxml"), 0});
    // [6, 16): 10 bytes of substituted value from caller row 5 col 9.
    outer.push_entry(6, 16,
                     CallSiteOrigin{std::filesystem::path("caller.scxml"), 5, 9});
    // [16, 22): 6 bytes from template.scxml[16..22] = "-after".
    outer.push_entry(16, 22,
                     FileOrigin{std::filesystem::path("template.scxml"), 16});

    // File-prefix: offset 0 → template.scxml (1, 1).
    SourcePos pos = outer.lookup(0);
    EXPECT_EQ(pos.file, std::filesystem::path("template.scxml"));
    EXPECT_EQ(pos.row, 1u);
    EXPECT_EQ(pos.col, 1u);

    // CallSite region: every offset inside [6, 16) resolves to the
    // same caller (row, col), regardless of position — depth-1 rule.
    for (const size_t off : {size_t{6}, size_t{10}, size_t{15}}) {
        pos = outer.lookup(off);
        EXPECT_EQ(pos.file, std::filesystem::path("caller.scxml"));
        EXPECT_EQ(pos.row, 5u);
        EXPECT_EQ(pos.col, 9u);
    }

    // File-suffix: offset 16 → template.scxml[16] = '-' on line 2,
    // which starts at byte 12 ("body-after"), so col 5.
    pos = outer.lookup(16);
    EXPECT_EQ(pos.file, std::filesystem::path("template.scxml"));
    EXPECT_EQ(pos.row, 2u);
    EXPECT_EQ(pos.col, 5u);

    // Composed map is not identity.
    EXPECT_FALSE(outer.is_identity());
}

// ── Test 6: is_identity() predicate shape ──────────────────────────
// Mirrors Rust `is_identity_rejects_multi_entry_maps` +
// `is_identity_rejects_offset_file_entry`. Single-entry File-origin
// at source_offset 0 is identity; anything else is not.
TEST(PositionMap, IsIdentityPredicate) {
    // Happy path: identity factory.
    auto identity_map =
        PositionMap::identity(std::filesystem::path("x.scxml"), "<a/>");
    EXPECT_TRUE(identity_map.is_identity());

    // Multi-entry map, even with both FileOrigin: not identity.
    PositionMap multi;
    multi.register_file(std::filesystem::path("x.scxml"), "<a/>");
    multi.push_entry(0, 2, FileOrigin{std::filesystem::path("x.scxml"), 0});
    multi.push_entry(2, 4, FileOrigin{std::filesystem::path("x.scxml"), 2});
    EXPECT_FALSE(multi.is_identity());

    // Single FileOrigin entry with source_offset != 0: not identity.
    PositionMap offset_file;
    offset_file.register_file(std::filesystem::path("x.scxml"), "zzz<a/>");
    offset_file.push_entry(0, 4, FileOrigin{std::filesystem::path("x.scxml"), 3});
    EXPECT_FALSE(offset_file.is_identity());

    // Single CallSite entry: not identity (wrong variant).
    PositionMap callsite_only;
    callsite_only.register_file(std::filesystem::path("x.scxml"), "<a/>");
    callsite_only.push_entry(
        0, 4, CallSiteOrigin{std::filesystem::path("x.scxml"), 1, 1});
    EXPECT_FALSE(callsite_only.is_identity());
}
