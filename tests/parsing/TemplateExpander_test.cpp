// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Phase C P2 scanner-primitive unit tests for
// `SCE::parsing::detail::findElementEnd` +
// `SCE::parsing::detail::collectTopLevelSceUseRanges`. Standing
// consumer for the `TemplateExpander.h` / `.cpp` infrastructure
// until a later Phase C P2 commit wires the full recursive
// expansion and `processSceTemplate` routes through it — per
// `feedback_built_but_unconsumed.md`, every new helper here must
// be exercised so the header is not dead-code.

#include "parsing/TemplateExpander.h"
#include "parsing/TemplateError.h"

#include <gtest/gtest.h>

#include <string>
#include <string_view>

using SCE::parsing::expandString;
using SCE::parsing::TemplateMalformed;
using SCE::parsing::detail::ByteRange;
using SCE::parsing::detail::collectTopLevelSceUseRanges;
using SCE::parsing::detail::findElementEnd;

// ── findElementEnd: self-closing ────────────────────────────────────
TEST(TemplateExpander, FindElementEndSelfClosing) {
    const std::string source = R"(<root><sce:use template="t"/></root>)";
    const std::size_t start = source.find("<sce:use");
    ASSERT_NE(start, std::string::npos);
    const std::size_t end = findElementEnd(source, start, "sce:use");
    EXPECT_EQ(source.substr(start, end - start),
              R"(<sce:use template="t"/>)");
}

// ── findElementEnd: open-close pair ─────────────────────────────────
TEST(TemplateExpander, FindElementEndOpenClose) {
    const std::string source =
        R"(<root><sce:use template="t">body</sce:use></root>)";
    const std::size_t start = source.find("<sce:use");
    const std::size_t end = findElementEnd(source, start, "sce:use");
    EXPECT_EQ(source.substr(start, end - start),
              R"(<sce:use template="t">body</sce:use>)");
}

// ── findElementEnd: nested same-tag depth tracking ──────────────────
// Two `<sce:use>` tags — outer and nested — must close in the right
// order; the scanner's depth counter has to pair the nested open
// with the first `</sce:use>` before falling through to the outer
// close.
TEST(TemplateExpander, FindElementEndNestedSameTag) {
    const std::string source =
        R"(<r><sce:use><sce:use template="t"/></sce:use></r>)";
    const std::size_t start = source.find("<sce:use");
    const std::size_t end = findElementEnd(source, start, "sce:use");
    EXPECT_EQ(source.substr(start, end - start),
              R"(<sce:use><sce:use template="t"/></sce:use>)");
}

// ── findElementEnd: nested open-close pair requires depth counter ───
// Load-bearing for the depth-counter branch inside `findElementEnd`:
// an open-close-pair inner `<sce:use>` must deepen the scan so the
// first `</sce:use>` (the *inner* one) does not terminate the outer
// walk prematurely. Neutralising the `++depth` branch must red this
// test — self-closing inner forms do not exercise that code path, so
// a dedicated open-close-pair fixture is necessary.
TEST(TemplateExpander, FindElementEndNestedOpenCloseRequiresDepth) {
    const std::string source =
        R"(<r><sce:use template="a"><sce:use template="b"></sce:use></sce:use></r>)";
    const std::size_t start = source.find("<sce:use");
    const std::size_t end = findElementEnd(source, start, "sce:use");
    EXPECT_EQ(
        source.substr(start, end - start),
        R"(<sce:use template="a"><sce:use template="b"></sce:use></sce:use>)");
}

// ── findElementEnd: `>` inside quoted attribute must not terminate ──
TEST(TemplateExpander, FindElementEndQuotedAngleInAttr) {
    const std::string source =
        R"(<root><sce:use template="t" label="a>b"/></root>)";
    const std::size_t start = source.find("<sce:use");
    const std::size_t end = findElementEnd(source, start, "sce:use");
    EXPECT_EQ(source.substr(start, end - start),
              R"(<sce:use template="t" label="a>b"/>)");
}

// ── findElementEnd: comment body containing `</sce:use>` ignored ────
TEST(TemplateExpander, FindElementEndCommentShieldsCloseTag) {
    const std::string source =
        R"(<r><sce:use template="t"><!-- </sce:use> -->x</sce:use></r>)";
    const std::size_t start = source.find("<sce:use");
    const std::size_t end = findElementEnd(source, start, "sce:use");
    EXPECT_EQ(
        source.substr(start, end - start),
        R"(<sce:use template="t"><!-- </sce:use> -->x</sce:use>)");
}

// ── collectTopLevelSceUseRanges: single element ─────────────────────
TEST(TemplateExpander, CollectTopLevelSceUseRangesSingle) {
    const std::string source = R"(<root><sce:use template="t"/></root>)";
    const auto ranges = collectTopLevelSceUseRanges(source);
    ASSERT_EQ(ranges.size(), 1u);
    EXPECT_EQ(source.substr(ranges[0].start, ranges[0].end - ranges[0].start),
              R"(<sce:use template="t"/>)");
}

// ── collectTopLevelSceUseRanges: skips nested (top-level only) ──────
// Mirrors Rust `collect_uses_into`: if a `<sce:use>` already sits on
// the path, its children are expanded by the recursive call, not by
// the outer walker. The caller expects one range here, not two.
TEST(TemplateExpander, CollectTopLevelSceUseRangesSkipsNested) {
    const std::string source =
        R"(<r><sce:use template="t"><sce:use template="inner"/></sce:use></r>)";
    const auto ranges = collectTopLevelSceUseRanges(source);
    ASSERT_EQ(ranges.size(), 1u);
    EXPECT_EQ(source.substr(ranges[0].start, ranges[0].end - ranges[0].start),
              R"(<sce:use template="t"><sce:use template="inner"/></sce:use>)");
}

// ── collectTopLevelSceUseRanges: multiple siblings in order ─────────
TEST(TemplateExpander, CollectTopLevelSceUseRangesSiblings) {
    const std::string source =
        R"(<r><sce:use template="a"/><x/><sce:use template="b"/></r>)";
    const auto ranges = collectTopLevelSceUseRanges(source);
    ASSERT_EQ(ranges.size(), 2u);
    EXPECT_EQ(source.substr(ranges[0].start, ranges[0].end - ranges[0].start),
              R"(<sce:use template="a"/>)");
    EXPECT_EQ(source.substr(ranges[1].start, ranges[1].end - ranges[1].start),
              R"(<sce:use template="b"/>)");
}

// ── collectTopLevelSceUseRanges: no `<sce:use>` → empty ─────────────
TEST(TemplateExpander, CollectTopLevelSceUseRangesNoneReturnsEmpty) {
    const std::string source = R"(<root><state id="s1"/></root>)";
    const auto ranges = collectTopLevelSceUseRanges(source);
    EXPECT_TRUE(ranges.empty());
}

// ── collectTopLevelSceUseRanges: malformed source throws ────────────
TEST(TemplateExpander, CollectTopLevelSceUseRangesMalformedThrows) {
    const std::string source = "<root><sce:use";  // truncated
    EXPECT_THROW(collectTopLevelSceUseRanges(source), TemplateMalformed);
}

// ── expandString: fast path returns identity on no `<sce:use>` ──────
TEST(TemplateExpander, ExpandStringNoSceUseReturnsIdentity) {
    const std::string source = R"(<root><state id="s1"/></root>)";
    const auto result = expandString(source, "main.scxml", ".");
    EXPECT_EQ(result.expanded_text, source);
    EXPECT_TRUE(result.positions.is_identity());
}
