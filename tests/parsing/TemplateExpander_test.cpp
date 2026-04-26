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

#include "parsing/PugiXMLParser.h"
#include "parsing/TemplateExpander.h"
#include "parsing/TemplateError.h"

#include <pugixml.hpp>

#include <gtest/gtest.h>

#include <filesystem>
#include <fstream>
#include <string>
#include <string_view>
#include <unordered_map>
#include <variant>

using SCE::parsing::CallSiteOrigin;
using SCE::parsing::expandString;
using SCE::parsing::FileOrigin;
using SCE::parsing::TemplateMalformed;
using SCE::parsing::detail::applySubstitutionWithTracking;
using SCE::parsing::detail::ByteRange;
using SCE::parsing::detail::collectTopLevelSceUseRanges;
using SCE::parsing::detail::collectUseBindings;
using SCE::parsing::detail::extractTemplateBodyRanges;
using SCE::parsing::detail::findElementEnd;
using SCE::parsing::detail::ParamDecl;
using SCE::parsing::detail::parseParamDecl;
using SCE::parsing::detail::SubstitutionResult;

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
    const auto upstream =
        SCE::parsing::PositionMap::identity("main.scxml", source);
    const auto result = expandString(source, "main.scxml", ".", upstream);
    EXPECT_EQ(result.expanded_text, source);
    EXPECT_TRUE(result.positions.is_identity());
}

// ── applySubstitutionWithTracking: basic File+CallSite entries ──────
// "before{$x}after" with `x=VAL` emits three entries: File prefix,
// CallSite substitution, File suffix. Mirrors Rust's tuple output for
// the same input.
TEST(TemplateExpander, ApplySubstitutionBasicOrigins) {
    const std::string body = "before{$x}after";
    std::unordered_map<std::string, std::string> params{{"x", "VAL"}};
    const auto result = applySubstitutionWithTracking(
        body, /*bodySourceOffset=*/100,
        std::filesystem::path("tmpl.scxml"), params,
        std::filesystem::path("caller.scxml"),
        /*callerRow=*/7, /*callerCol=*/3);
    EXPECT_EQ(result.substituted, "beforeVALafter");
    ASSERT_EQ(result.entries.size(), 3u);

    // Entry 0: "before" — FileOrigin(tmpl, offset 100).
    EXPECT_EQ(result.entries[0].out_start, 0u);
    EXPECT_EQ(result.entries[0].out_end, 6u);
    const auto *fileOrigin0 =
        std::get_if<FileOrigin>(&result.entries[0].origin);
    ASSERT_NE(fileOrigin0, nullptr);
    EXPECT_EQ(fileOrigin0->path, std::filesystem::path("tmpl.scxml"));
    EXPECT_EQ(fileOrigin0->source_offset, 100u);

    // Entry 1: "VAL" — CallSiteOrigin(caller, 7, 3).
    EXPECT_EQ(result.entries[1].out_start, 6u);
    EXPECT_EQ(result.entries[1].out_end, 9u);
    const auto *callSite =
        std::get_if<CallSiteOrigin>(&result.entries[1].origin);
    ASSERT_NE(callSite, nullptr);
    EXPECT_EQ(callSite->path, std::filesystem::path("caller.scxml"));
    EXPECT_EQ(callSite->row, 7u);
    EXPECT_EQ(callSite->col, 3u);

    // Entry 2: "after" — FileOrigin(tmpl, offset 100+10=110). The
    // body-local offset of "after" is 10 (after the closing `}`).
    const auto *fileOrigin2 =
        std::get_if<FileOrigin>(&result.entries[2].origin);
    ASSERT_NE(fileOrigin2, nullptr);
    EXPECT_EQ(fileOrigin2->source_offset, 110u);
}

// ── applySubstitutionWithTracking: empty value omits CallSite entry ──
// Rust path omits the `CallSite` entry when the param's value is
// empty; the surrounding File entries remain untouched. The emitted
// string is the concatenation of the prefix + suffix alone.
TEST(TemplateExpander, ApplySubstitutionEmptyValueSkipsEntry) {
    const std::string body = "x{$p}y";
    std::unordered_map<std::string, std::string> params{{"p", ""}};
    const auto result = applySubstitutionWithTracking(
        body, 0, std::filesystem::path("t"), params,
        std::filesystem::path("c"), 1, 1);
    EXPECT_EQ(result.substituted, "xy");
    ASSERT_EQ(result.entries.size(), 2u);
    EXPECT_TRUE(std::holds_alternative<FileOrigin>(result.entries[0].origin));
    EXPECT_TRUE(std::holds_alternative<FileOrigin>(result.entries[1].origin));
}

// ── applySubstitutionWithTracking: undeclared token → literal `{$` ──
// Mirrors Rust's literal-emission path when the token name is not in
// `params`. Only `{$` is emitted as template bytes; the remaining
// characters continue the scan (they are tail bytes, not part of
// any token).
TEST(TemplateExpander, ApplySubstitutionUndeclaredEmitsLiteralBrace) {
    const std::string body = "before{$unknown}after";
    std::unordered_map<std::string, std::string> params;  // no bindings
    const auto result = applySubstitutionWithTracking(
        body, 0, std::filesystem::path("t"), params,
        std::filesystem::path("c"), 1, 1);
    EXPECT_EQ(result.substituted, body);
    for (const auto &entry : result.entries) {
        EXPECT_TRUE(std::holds_alternative<FileOrigin>(entry.origin));
    }
}

// ── parseParamDecl: happy path ──────────────────────────────────────
TEST(TemplateExpander, ParseParamDeclBasic) {
    pugi::xml_document doc;
    const char *xml = R"(<sce:param name="port" required="true"/>)";
    ASSERT_TRUE(doc.load_string(xml));
    const ParamDecl decl =
        parseParamDecl(doc.document_element(), "tmpl");
    EXPECT_EQ(decl.name, "port");
    EXPECT_TRUE(decl.required);
    EXPECT_FALSE(decl.hasDefault);
}

// ── parseParamDecl: required + default mutual exclusion ─────────────
TEST(TemplateExpander, ParseParamDeclRequiredAndDefaultRejected) {
    pugi::xml_document doc;
    const char *xml =
        R"(<sce:param name="p" required="true" default="x"/>)";
    ASSERT_TRUE(doc.load_string(xml));
    EXPECT_THROW(parseParamDecl(doc.document_element(), "tmpl"),
                 TemplateMalformed);
}

// ── parseParamDecl: invalid name pattern ────────────────────────────
TEST(TemplateExpander, ParseParamDeclInvalidNameRejected) {
    pugi::xml_document doc;
    const char *xml = R"(<sce:param name="9bad"/>)";
    ASSERT_TRUE(doc.load_string(xml));
    EXPECT_THROW(parseParamDecl(doc.document_element(), "tmpl"),
                 TemplateMalformed);
}

// ── collectUseBindings: skips template + xmlns attributes ───────────
TEST(TemplateExpander, CollectUseBindingsFiltersReserved) {
    pugi::xml_document doc;
    const char *xml =
        R"(<sce:use xmlns:sce="http://sce.dev/ext" template="t" port="80" host="h"/>)";
    ASSERT_TRUE(doc.load_string(xml));
    const auto bindings = collectUseBindings(doc.document_element());
    EXPECT_EQ(bindings.size(), 2u);
    EXPECT_EQ(bindings.at("port"), "80");
    EXPECT_EQ(bindings.at("host"), "h");
    EXPECT_EQ(bindings.count("template"), 0u);
    EXPECT_EQ(bindings.count("xmlns:sce"), 0u);
}

// ── extractTemplateBodyRanges: element + sibling, param skipped ─────
TEST(TemplateExpander, ExtractTemplateBodyRangesBasic) {
    const std::string expanded =
        R"(<sce:template xmlns:sce="http://sce.dev/ext"><sce:param name="p"/><a/><b/></sce:template>)";
    const auto ranges = extractTemplateBodyRanges(expanded, "tmpl");
    ASSERT_EQ(ranges.size(), 2u);
    EXPECT_EQ(expanded.substr(ranges[0].start, ranges[0].end - ranges[0].start),
              "<a/>");
    EXPECT_EQ(expanded.substr(ranges[1].start, ranges[1].end - ranges[1].start),
              "<b/>");
}

// ── extractTemplateBodyRanges: wrong-root throws ────────────────────
TEST(TemplateExpander, ExtractTemplateBodyRangesWrongRootThrows) {
    const std::string expanded = R"(<root><a/></root>)";
    EXPECT_THROW(extractTemplateBodyRanges(expanded, "tmpl"),
                 TemplateMalformed);
}

// ── expandString: end-to-end simple substitution ────────────────────
// Writes a template file with one param + one body element, invokes
// it from an in-memory caller, verifies the expanded text contains
// the substituted value and the PositionMap lookups resolve as
// expected (body bytes → template file; substituted bytes → caller).
TEST(TemplateExpander, ExpandStringSimpleSubstitution) {
    const auto tmpDir = std::filesystem::temp_directory_path() /
                        "sce_template_expander_test_simple";
    std::filesystem::create_directories(tmpDir);
    const auto tmplPath = tmpDir / "t.scxml";
    const std::string tmplBody =
        R"(<sce:template xmlns:sce="http://sce.dev/ext"><sce:param name="x"/><state id="{$x}"/></sce:template>)";
    {
        std::ofstream out(tmplPath, std::ios::binary);
        out << tmplBody;
    }
    const std::string caller =
        R"(<root xmlns:sce="http://sce.dev/ext"><sce:use template="t.scxml" x="S1"/></root>)";
    const auto upstream = SCE::parsing::PositionMap::identity(
        (tmpDir / "caller.scxml").string(), caller);
    const auto result =
        expandString(caller, (tmpDir / "caller.scxml").string(),
                     tmpDir.string(), upstream);
    EXPECT_FALSE(result.positions.is_identity());
    EXPECT_NE(result.expanded_text.find("<state id=\"S1\"/>"),
              std::string::npos);

    // Byte inside `S1` should resolve back to the caller file (the
    // CallSiteOrigin depth-1 collapse).
    const std::size_t s1Offset = result.expanded_text.find("S1");
    ASSERT_NE(s1Offset, std::string::npos);
    const auto substPos = result.positions.lookup(s1Offset);
    EXPECT_EQ(substPos.file, tmpDir / "caller.scxml");

    // Byte inside the spliced `<state` (template-file bytes) should
    // resolve to the template file.
    const std::size_t stateOffset = result.expanded_text.find("<state");
    ASSERT_NE(stateOffset, std::string::npos);
    const auto bodyPos = result.positions.lookup(stateOffset + 2);  // inside `ta`
    EXPECT_EQ(bodyPos.file, tmplPath);

    std::filesystem::remove_all(tmpDir);
}

// ── expandString: missing required param throws ────────────────────
TEST(TemplateExpander, ExpandStringMissingRequiredParamThrows) {
    const auto tmpDir = std::filesystem::temp_directory_path() /
                        "sce_template_expander_test_missing";
    std::filesystem::create_directories(tmpDir);
    const auto tmplPath = tmpDir / "t.scxml";
    {
        std::ofstream out(tmplPath, std::ios::binary);
        out << R"(<sce:template xmlns:sce="http://sce.dev/ext"><sce:param name="x" required="true"/><a/></sce:template>)";
    }
    const std::string caller =
        R"(<root xmlns:sce="http://sce.dev/ext"><sce:use template="t.scxml"/></root>)";
    const auto upstream = SCE::parsing::PositionMap::identity(
        (tmpDir / "caller.scxml").string(), caller);
    EXPECT_THROW(
        expandString(caller, (tmpDir / "caller.scxml").string(),
                     tmpDir.string(), upstream),
        SCE::parsing::TemplateMissingParam);
    std::filesystem::remove_all(tmpDir);
}

// ── expandString: unknown caller binding throws ────────────────────
TEST(TemplateExpander, ExpandStringUnknownBindingThrows) {
    const auto tmpDir = std::filesystem::temp_directory_path() /
                        "sce_template_expander_test_unknown";
    std::filesystem::create_directories(tmpDir);
    const auto tmplPath = tmpDir / "t.scxml";
    {
        std::ofstream out(tmplPath, std::ios::binary);
        out << R"(<sce:template xmlns:sce="http://sce.dev/ext"><sce:param name="x"/><a/></sce:template>)";
    }
    const std::string caller =
        R"(<root xmlns:sce="http://sce.dev/ext"><sce:use template="t.scxml" x="v" bogus="b"/></root>)";
    const auto upstream = SCE::parsing::PositionMap::identity(
        (tmpDir / "caller.scxml").string(), caller);
    EXPECT_THROW(
        expandString(caller, (tmpDir / "caller.scxml").string(),
                     tmpDir.string(), upstream),
        SCE::parsing::TemplateUnknownParam);
    std::filesystem::remove_all(tmpDir);
}

// ── expandString: template not found throws ────────────────────────
TEST(TemplateExpander, ExpandStringNotFoundThrows) {
    const std::string caller =
        R"(<root xmlns:sce="http://sce.dev/ext"><sce:use template="does_not_exist.scxml"/></root>)";
    const auto upstream =
        SCE::parsing::PositionMap::identity("/tmp/caller.scxml", caller);
    EXPECT_THROW(
        expandString(caller, "/tmp/caller.scxml", "/tmp", upstream),
        SCE::parsing::TemplateNotFound);
}

// ── PugiXMLDocument::processSceTemplate: roundtrip through parser ──
// End-to-end path: write caller + template to disk, parse through
// the real PugiXMLParser so sourceText_ / sourcePath_ / basePath_
// are populated, run processSceTemplate, and assert the returned
// PositionMap looks up as expected. Closes the RFC §3 P2 deliverable
// #1 contract (processSceTemplate returns PositionMap) with a
// consumer exercising the threaded member.
TEST(ProcessSceTemplate, RoundtripResolvesBodyAndCallsite) {
    const auto tmpDir = std::filesystem::temp_directory_path() /
                        "sce_process_sce_template_test_roundtrip";
    std::filesystem::create_directories(tmpDir);
    const auto tmplPath = tmpDir / "t.scxml";
    const auto callerPath = tmpDir / "caller.scxml";
    {
        std::ofstream out(tmplPath, std::ios::binary);
        out << R"(<sce:template xmlns:sce="http://sce.dev/ext"><sce:param name="id"/><state id="{$id}"/></sce:template>)";
    }
    {
        std::ofstream out(callerPath, std::ios::binary);
        out << R"(<root xmlns:sce="http://sce.dev/ext"><sce:use template="t.scxml" id="S1"/></root>)";
    }

    SCE::PugiXMLParser parser;
    auto doc = parser.parseFile(callerPath.string());
    ASSERT_NE(doc, nullptr);
    // RFC §W4.5 D1: process*() return PositionMap directly; failures
    // throw and would surface as gtest unhandled-exception failures.
    const auto xPositions = doc->processXInclude();
    const auto positions = doc->processSceTemplate(xPositions);
    EXPECT_FALSE(positions.is_identity());

    std::filesystem::remove_all(tmpDir);
}

// ── TemplateError::location() populated at throw sites ──────────────
// Round-trip verification that the depth-1 author-source location
// is stamped on thrown subtypes so diagnostic consumers can present
// `(file, row, col)` directly. A caller with a `<sce:use>` on its
// second line pointing at a missing template should produce a
// TemplateNotFound whose location() names the caller file at row 2.
TEST(TemplateExpander, ErrorLocationPointsAtCallerSceUse) {
    const std::string callerSource =
        "<root xmlns:sce=\"http://sce.dev/ext\">\n"
        "  <sce:use template=\"does_not_exist.scxml\"/>\n"
        "</root>";
    const std::string callerPath = "/tmp/p2_location_test_caller.scxml";
    const auto upstream =
        SCE::parsing::PositionMap::identity(callerPath, callerSource);
    try {
        expandString(callerSource, callerPath, "/tmp", upstream);
        FAIL() << "expected TemplateNotFound";
    } catch (const SCE::parsing::TemplateNotFound &e) {
        const auto &loc = e.location();
        ASSERT_TRUE(loc.has_value());
        EXPECT_EQ(loc->file, std::filesystem::path(callerPath));
        EXPECT_EQ(loc->row, 2u);
        // The `<sce:use>` element starts at column 3 on line 2
        // (two leading spaces + `<`).
        EXPECT_EQ(loc->col, 3u);
    }
}

// ── PugiXMLDocument::processSceTemplate threads upstream map ────────
// Phase X RFC §1 Q2 contract: when a document has an `<xi:include>`
// but no `<sce:use>`, the upstream PositionMap produced by
// `processXInclude` must thread through `processSceTemplate`'s
// fast-path unchanged, so a byte originating in the spliced fragment
// still resolves to the fragment file after both preprocessor stages
// have run. Without this passthrough, fragment-byte diagnostics
// would be wrongly attributed to the host file.
//
// Standing-consumer pattern from RFC §1 Q4 condition #1: the test
// drives the production entry points (`processXInclude` →
// `processSceTemplate`) end-to-end, not the expander helpers in
// isolation. Bite: replacing `result.positions = upstream;` in the
// no-`<sce:use>` fast path with
// `PositionMap::identity(sourcePath_, sourceText_)` (the pre-Phase-X
// shape) reds the assertion below — the fragment byte then resolves
// to `main.xml` instead of `frag.xml`.
TEST(ProcessSceTemplate, ThreadsUpstreamMapForNoSceUseDoc) {
    const auto tmpDir = std::filesystem::temp_directory_path() /
                        "sce_process_sce_template_threads_upstream";
    std::filesystem::create_directories(tmpDir);
    const auto fragPath = tmpDir / "frag.xml";
    const auto mainPath = tmpDir / "main.xml";
    {
        std::ofstream out(fragPath, std::ios::binary);
        out << R"(<fragment><state id="leaf"/></fragment>)";
    }
    const std::string mainSrc =
        R"(<root><xi:include xmlns:xi="http://www.w3.org/2001/XInclude" href="frag.xml"/></root>)";
    {
        std::ofstream out(mainPath, std::ios::binary);
        out << mainSrc;
    }

    auto rawDoc = std::make_shared<pugi::xml_document>();
    ASSERT_TRUE(rawDoc->load_buffer(mainSrc.data(), mainSrc.size()));

    SCE::PugiXMLDocument doc(rawDoc);
    doc.setSourcePath(mainPath.string());
    doc.setSourceText(mainSrc);
    doc.setBasePath(tmpDir.string());

    // RFC §W4.5 D1: process*() return PositionMap directly.
    const auto xPositions = doc.processXInclude();

    // Upstream attributes some byte to frag.xml (the spliced
    // fragment region). Probe to find one — the harness does not
    // depend on the exact post-XInclude byte layout.
    std::size_t fragByte = 0;
    bool foundFragByte = false;
    for (std::size_t i = 0; i < 4096; ++i) {
        const auto pos = xPositions.lookup(i);
        if (pos.file.filename() == "frag.xml") {
            fragByte = i;
            foundFragByte = true;
            break;
        }
    }
    ASSERT_TRUE(foundFragByte)
        << "upstream PositionMap from processXInclude must attribute "
           "at least one byte to frag.xml";

    const auto tPositions = doc.processSceTemplate(xPositions);

    const auto upstreamPos = xPositions.lookup(fragByte);
    const auto downstreamPos = tPositions.lookup(fragByte);
    EXPECT_EQ(upstreamPos.file, downstreamPos.file)
        << "fragment-byte origin must survive processSceTemplate's "
           "no-`<sce:use>` fast path — Phase X RFC §1 Q2 upstream "
           "passthrough";
    EXPECT_EQ(upstreamPos.row, downstreamPos.row);
    EXPECT_EQ(upstreamPos.col, downstreamPos.col);

    std::filesystem::remove_all(tmpDir);
}
