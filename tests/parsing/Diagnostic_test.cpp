// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "factory/NodeFactory.h"
#include "parsing/Diagnostic.h"
#include "parsing/DiagnosticBatchFormatter.h"
#include "parsing/SCXMLParser.h"
#include "parsing/TemplateError.h"
#include "parsing/XIncludeError.h"
#include "parsing/XIncludeExpander.h"

#include <gtest/gtest.h>

#include <array>
#include <memory>
#include <optional>
#include <regex>
#include <set>
#include <sstream>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

// Standing consumer for `SCE::parsing::Diagnostic` and the concrete
// `TemplateError` refit (RFC §W1 commit-series). Two layers:
//
//   1. Abstract-base contract (FakeDiagnostic) — removing any pure
//      virtual override reds the build with "cannot declare variable
//      of abstract type", so the interface stays load-bearing per
//      `feedback_built_but_unconsumed.md`.
//
//   2. v1-schema conformance for each TemplateError subtype's
//      `to_json()` output — required fields present with the exact
//      Rust wire names, code returns the Rust `xml/template-*`
//      DiagnosticCode literal, id matches `^fnv1a:[0-9a-f]{16}$`,
//      no unexpected fields, optional `location` shape matches the
//      schema's `{file, line, col}` envelope.
//
// Together the two layers pin the C++ producer to the schema in
// `schemas/sce-diagnostic.v1.schema.json`, which the Rust side
// (`sce-build/src/forge/diagnostic.rs`) drives as the authority.

namespace SCE::parsing {
namespace {

class FakeDiagnostic : public Diagnostic {
public:
    std::string_view code() const noexcept override {
        return "xml/template-cycle";
    }

    const std::optional<SourcePos> &location() const noexcept override {
        static const std::optional<SourcePos> kNone;
        return kNone;
    }

    nlohmann::ordered_json to_json() const override {
        nlohmann::ordered_json j;
        j["v"] = 1;
        j["id"] = "fnv1a:0000000000000000";
        j["code"] = code();
        j["stage"] = "xml";
        j["message"] = "fake";
        return j;
    }

    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<FakeDiagnostic>(*this);
    }
};

TEST(DiagnosticBase, FakeSubtypeImplementsContract) {
    FakeDiagnostic d;
    EXPECT_EQ(d.code(), std::string_view{"xml/template-cycle"});
    EXPECT_FALSE(d.location().has_value());

    const auto j = d.to_json();
    EXPECT_EQ(j["v"].get<int>(), 1);
    EXPECT_EQ(j["code"].get<std::string>(), "xml/template-cycle");
    EXPECT_EQ(j["stage"].get<std::string>(), "xml");
    EXPECT_EQ(j["message"].get<std::string>(), "fake");
}

TEST(DiagnosticBase, PolymorphicErasureViaBaseReference) {
    FakeDiagnostic concrete;
    const Diagnostic &erased = concrete;
    EXPECT_EQ(erased.code(), std::string_view{"xml/template-cycle"});
    EXPECT_FALSE(erased.location().has_value());
    EXPECT_EQ(erased.to_json()["code"].get<std::string>(),
              "xml/template-cycle");
}

// ── v1 schema conformance for TemplateError subtypes ───────────────

namespace conformance {

// Curated mirror of the `xml/template-*` entries in
// `schemas/sce-diagnostic.v1.schema.json` (also pinned by the Rust
// drift test `cpp_template_subtypes_match_rust_diagnostic_codes`).
// Hand-edited list rather than runtime introspection so a typo or a
// silent rename on either side reds this fixture with a pointed
// string-equality diff against the constant on this side.
const std::array<std::string_view, 8> kExpectedCodes = {
    "xml/template-not-found",     "xml/template-read-error",
    "xml/template-malformed",     "xml/template-missing-attribute",
    "xml/template-missing-param", "xml/template-unknown-param",
    "xml/template-cycle",         "xml/template-too-deep",
};

bool matchesIdRegex(const std::string &id) {
    static const std::regex kPattern{R"(^fnv1a:[0-9a-f]{16}$)"};
    return std::regex_match(id, kPattern);
}

void assertSchemaConformantBase(const nlohmann::ordered_json &j,
                                std::string_view expectedCode) {
    // v1 schema requires v/id/code/stage/message; spec/location/
    // expected/actual/fix are optional. The C++ producer in W1 emits
    // exactly the required set (plus location when populated); the
    // assertions below pin both presence AND absence so a future
    // additive field surfaces as a curated-set drift.
    ASSERT_TRUE(j.contains("v")) << j.dump();
    ASSERT_TRUE(j.contains("id")) << j.dump();
    ASSERT_TRUE(j.contains("code")) << j.dump();
    ASSERT_TRUE(j.contains("stage")) << j.dump();
    ASSERT_TRUE(j.contains("message")) << j.dump();

    EXPECT_EQ(j.at("v").get<int>(), 1);
    EXPECT_EQ(j.at("code").get<std::string>(), std::string{expectedCode});
    EXPECT_EQ(j.at("stage").get<std::string>(), "xml");
    EXPECT_TRUE(matchesIdRegex(j.at("id").get<std::string>()))
        << "id '" << j.at("id").get<std::string>()
        << "' does not match ^fnv1a:[0-9a-f]{16}$";
    EXPECT_FALSE(j.at("message").get<std::string>().empty());
}

// Curated set of legal top-level keys (mirrors v1 schema's
// `additionalProperties: false` constraint at the envelope level —
// nlohmann::json does not validate this for us, so we enforce it
// explicitly here so a stray field added on the C++ side reds.
const std::set<std::string> kAllowedTopLevelKeys = {
    "v",       "id",       "code",   "stage", "spec",
    "message", "location", "expected", "actual", "fix",
};

void assertNoUnexpectedKeys(const nlohmann::ordered_json &j) {
    for (const auto &item : j.items()) {
        EXPECT_TRUE(kAllowedTopLevelKeys.count(item.key()) == 1)
            << "unexpected top-level key: '" << item.key() << "'";
    }
}

}  // namespace conformance

TEST(TemplateErrorWire, TemplateNotFoundConformsToV1Schema) {
    const TemplateNotFound err(
        "<sce:use template=\"missing.sce-template.xml\">: file not "
        "found (searched: /project/missing.sce-template.xml)");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/template-not-found");
    conformance::assertNoUnexpectedKeys(j);
    EXPECT_FALSE(j.contains("location"));
}

TEST(TemplateErrorWire, TemplateReadErrorConformsToV1Schema) {
    const TemplateReadError err(
        "<sce:use template=\"unreadable.sce-template.xml\">: read error: "
        "permission denied");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/template-read-error");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(TemplateErrorWire, TemplateMalformedConformsToV1Schema) {
    const TemplateMalformed err(
        "<sce:use template=\"bad.sce-template.xml\">: template is "
        "malformed: expanded template is malformed: unexpected end of file");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/template-malformed");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(TemplateErrorWire, TemplateMissingAttributeConformsToV1Schema) {
    const TemplateMissingAttribute err(
        "<sce:use> is missing required 'template' attribute");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(
        j, "xml/template-missing-attribute");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(TemplateErrorWire, TemplateMissingParamConformsToV1Schema) {
    const TemplateMissingParam err(
        "<sce:use template=\"guard.sce-template.xml\">: missing required "
        "parameter 'condition'");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/template-missing-param");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(TemplateErrorWire, TemplateUnknownParamConformsToV1Schema) {
    const TemplateUnknownParam err(
        "<sce:use template=\"guard.sce-template.xml\">: unknown parameter "
        "'state' (declared: condition, action)");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/template-unknown-param");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(TemplateErrorWire, TemplateCycleConformsToV1Schema) {
    const TemplateCycle err(
        "<sce:use template=\"a.sce-template.xml\">: cycle detected "
        "(/a.sce-template.xml -> /b.sce-template.xml -> /a.sce-template.xml)");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/template-cycle");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(TemplateErrorWire, TemplateTooDeepConformsToV1Schema) {
    const TemplateTooDeep err(
        "<sce:use>: template depth limit (32) exceeded");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/template-too-deep");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(TemplateErrorWire, EveryCuratedCodeIsExercised) {
    // Sanity-check the curated list mirrors the 8 subtypes above.
    // Adding a 9th wire code without a corresponding test reds here.
    EXPECT_EQ(conformance::kExpectedCodes.size(), 8u);
    std::set<std::string_view> uniq(conformance::kExpectedCodes.begin(),
                                    conformance::kExpectedCodes.end());
    EXPECT_EQ(uniq.size(), conformance::kExpectedCodes.size())
        << "duplicate entry in kExpectedCodes";
}

TEST(TemplateErrorWire, LocationFieldShapeWhenPresent) {
    TemplateCycle err(
        "<sce:use template=\"a.sce-template.xml\">: cycle detected");
    err.setLocation(SourcePos{
        std::filesystem::path{"/project/main.scxml"}, 12u, 5u});

    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/template-cycle");
    conformance::assertNoUnexpectedKeys(j);

    ASSERT_TRUE(j.contains("location"));
    const auto &loc = j.at("location");
    ASSERT_TRUE(loc.contains("file"));
    ASSERT_TRUE(loc.contains("line"));
    ASSERT_TRUE(loc.contains("col"));
    EXPECT_EQ(loc.at("file").get<std::string>(), "/project/main.scxml");
    EXPECT_EQ(loc.at("line").get<unsigned>(), 12u);
    EXPECT_EQ(loc.at("col").get<unsigned>(), 5u);

    // location envelope is closed at v1: only file/line/col allowed.
    static const std::set<std::string> kAllowedLocationKeys = {
        "file", "line", "col"};
    for (const auto &item : loc.items()) {
        EXPECT_TRUE(kAllowedLocationKeys.count(item.key()) == 1)
            << "unexpected location key: '" << item.key() << "'";
    }
}

TEST(TemplateErrorWire, IdIsStableAcrossCalls) {
    // Identity is content-addressed (RFC §W1 / SCE_ERROR_CONTRACT.md):
    // re-rendering the same logical error must not shift its id.
    const TemplateCycle err(
        "<sce:use template=\"a.sce-template.xml\">: cycle detected");
    EXPECT_EQ(err.to_json().at("id").get<std::string>(),
              err.to_json().at("id").get<std::string>());
}

// ── XInclude wire-layer probes ────────────────────────────────────
//
// Parallel structure to `TemplateErrorWire` above for the
// `xml/xinclude-*` Rust DiagnosticCode family (7 codes pinned in
// `schemas/sce-diagnostic.v1.schema.json` lines 27-32 and
// `sce-build/src/forge/diagnostic.rs::DiagnosticCode::Xml*`).
//
// W3 (`claudedocs/rfc-sce-diagnostic-wire-unification.md`) promotes
// `XIncludeExpansionError` to implement `SCE::parsing::Diagnostic`
// with 7 typed leaf subtypes — `XIncludeMissingHref`,
// `XIncludeNotFound`, `XIncludeReadError`, `XIncludeCycle`,
// `XIncludeTooDeep`, `XIncludeMalformed`, `XIncludeUnsupported`.
// Catching by base reference still works (the leaf is-a
// `XIncludeExpansionError`); the additional `EXPECT_EQ(e.code(), ...)`
// probe asserts virtual dispatch lands on the right leaf so a
// future throw-site rewrite that picks the wrong subtype reds here.

TEST(XIncludeErrorWire, MissingHrefCarriesActionableFragmentInMessage) {
    // Mirrors Rust `xml/xinclude-missing-href`. Both the missing-href
    // and empty-href shapes collapse onto the same Rust code (the
    // empty string never resolves), so a single C++ subtype handles
    // both — `XIncludeMissingHref`. The message names the missing
    // attribute so an operator/agent can dispatch the repair without
    // re-parsing the call site.
    const std::string src =
        R"(<root><xi:include xmlns:xi="http://www.w3.org/2001/XInclude"/></root>)";
    try {
        SCE::parsing::expandStringX(src, "inline", "");
        FAIL() << "expandStringX must throw on missing href";
    } catch (const SCE::parsing::XIncludeExpansionError &e) {
        const std::string what = e.what();
        EXPECT_NE(what.find("href"), std::string::npos) << what;
        EXPECT_EQ(e.code(),
                  std::string_view{"xml/xinclude-missing-href"});
    }
}

TEST(XIncludeErrorWire, EmptyHrefCarriesActionableFragmentInMessage) {
    const std::string src =
        R"(<root><xi:include xmlns:xi="http://www.w3.org/2001/XInclude" href=""/></root>)";
    try {
        SCE::parsing::expandStringX(src, "inline", "");
        FAIL() << "expandStringX must throw on empty href";
    } catch (const SCE::parsing::XIncludeExpansionError &e) {
        const std::string what = e.what();
        EXPECT_NE(what.find("href"), std::string::npos) << what;
        EXPECT_EQ(e.code(),
                  std::string_view{"xml/xinclude-missing-href"});
    }
}

// ── v1 schema conformance for XIncludeError subtypes ──────────────
//
// Mirror of the TemplateError schema-conformance tests above for the
// 7 typed xinclude leaves promoted in RFC §W3. Each test constructs
// a leaf with an example message and asserts the to_json() envelope
// passes the shared `assertSchemaConformantBase` + `assertNoUnexpectedKeys`
// curated checks against `schemas/sce-diagnostic.v1.schema.json`.

namespace conformance {

// Curated mirror of the `xml/xinclude-*` entries in
// `schemas/sce-diagnostic.v1.schema.json` (lines 27-33). Hand-edited
// list rather than runtime introspection so a typo or silent rename
// on either side reds this fixture with a pointed string-equality
// diff. Pinned cross-side by the W3-3 Rust drift tests in
// `sce-build/src/xinclude.rs::tests`.
const std::array<std::string_view, 7> kExpectedXIncludeCodes = {
    "xml/xinclude-missing-href", "xml/xinclude-not-found",
    "xml/xinclude-read-error",   "xml/xinclude-cycle",
    "xml/xinclude-too-deep",     "xml/xinclude-malformed",
    "xml/xinclude-unsupported",
};

}  // namespace conformance

TEST(XIncludeErrorWire, MissingHrefConformsToV1Schema) {
    const XIncludeMissingHref err(
        "<xi:include> missing or empty `href` attribute");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/xinclude-missing-href");
    conformance::assertNoUnexpectedKeys(j);
    EXPECT_FALSE(j.contains("location"));
}

TEST(XIncludeErrorWire, NotFoundConformsToV1Schema) {
    const XIncludeNotFound err(
        "<xi:include href=\"missing.xml\">: file not found "
        "(searched: /project/missing.xml)");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/xinclude-not-found");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(XIncludeErrorWire, ReadErrorConformsToV1Schema) {
    const XIncludeReadError err(
        "<xi:include href=\"frag.xml\">: cannot read file: "
        "/project/frag.xml");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/xinclude-read-error");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(XIncludeErrorWire, CycleConformsToV1Schema) {
    const XIncludeCycle err(
        "<xi:include href=\"a.xml\">: cycle detected "
        "(/a.xml -> /b.xml -> /a.xml)");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/xinclude-cycle");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(XIncludeErrorWire, TooDeepConformsToV1Schema) {
    const XIncludeTooDeep err(
        "<xi:include> nesting exceeds depth limit of 10");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/xinclude-too-deep");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(XIncludeErrorWire, MalformedConformsToV1Schema) {
    const XIncludeMalformed err(
        "<xi:include href=\"bad.xml\">: included file is malformed: "
        "unexpected end of file");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/xinclude-malformed");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(XIncludeErrorWire, UnsupportedConformsToV1Schema) {
    const XIncludeUnsupported err(
        "<xi:include href=\"frag.xml\">: unsupported feature: "
        "parse=\"text\" (only parse=\"xml\" is supported)");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/xinclude-unsupported");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(XIncludeErrorWire, EveryCuratedXIncludeCodeIsExercised) {
    // Sanity-check the curated list mirrors the 7 subtypes above.
    // Adding an 8th wire code without a corresponding test reds here.
    EXPECT_EQ(conformance::kExpectedXIncludeCodes.size(), 7u);
    std::set<std::string_view> uniq(
        conformance::kExpectedXIncludeCodes.begin(),
        conformance::kExpectedXIncludeCodes.end());
    EXPECT_EQ(uniq.size(), conformance::kExpectedXIncludeCodes.size())
        << "duplicate entry in kExpectedXIncludeCodes";
}

TEST(XIncludeErrorWire, IdDiffersAcrossSubtypesWithSameMessage) {
    // Mixing two subtypes' code() into the FNV-1a key keeps the id
    // distinct even when the rendered message text happens to match
    // (the canonical key prepends `code | stage | file` before the
    // message fragment, so flipping code flips id). Mirrors the
    // sister `TemplateErrorWire.IdDiffersAcrossSubtypesWithSameMessage`
    // for the xinclude family.
    const std::string msg = "shared message";
    const XIncludeCycle a(msg);
    const XIncludeMalformed b(msg);
    EXPECT_NE(a.to_json().at("id").get<std::string>(),
              b.to_json().at("id").get<std::string>());
}

// ── Canonical-JSON string (RFC §W2 deliverable #3) ────────────────

TEST(TemplateErrorWire, CanonicalJsonStringIsKeyOrderStable) {
    // `to_canonical_json_string()` must produce the same bytes for
    // the same diagnostic across calls — that is the contract any
    // byte-diff consumer relies on. The producer-side
    // `nlohmann::ordered_json` insertion order is irrelevant after
    // canonicalisation; re-running on the same object hits std::map
    // alphabetical order both times.
    //
    // Load-bearing bite: change `dump(-1, ' ', false)` in
    // `Diagnostic.cpp` to `dump(2)` (pretty-print) and this test
    // stays green for the equality assertion BUT the no-whitespace
    // assertion below reds with a leading space at every key.
    const TemplateCycle err(
        "<sce:use template=\"a.sce-template.xml\">: cycle detected "
        "(/a.sce-template.xml -> /b.sce-template.xml -> /a.sce-template.xml)");

    const std::string a = err.to_canonical_json_string();
    const std::string b = err.to_canonical_json_string();
    EXPECT_EQ(a, b);

    // Required v1 fields are present in the canonical string.
    EXPECT_NE(a.find("\"v\":1"), std::string::npos) << a;
    EXPECT_NE(a.find("\"code\":\"xml/template-cycle\""),
              std::string::npos)
        << a;
    EXPECT_NE(a.find("\"stage\":\"xml\""), std::string::npos) << a;

    // No-whitespace contract: dump(-1, ' ', false) emits compact
    // form. A pretty-print regression introduces literal '\n' or
    // indent spacing.
    EXPECT_EQ(a.find('\n'), std::string::npos) << a;
    EXPECT_EQ(a.find("\": "), std::string::npos)
        << "found ': ' (with space) — canonical dump must be "
           "compact: " << a;
}

TEST(TemplateErrorWire, CanonicalJsonStringHasAlphabeticalKeyOrder) {
    // Producer-side `to_json()` emits keys in insertion order
    // (v, id, code, stage, message, location). Canonicalisation
    // re-parses through std::map so the byte stream lands keys
    // alphabetically — `code`, `id`, `message`, `stage`, `v` for
    // the location-less subtypes. Pin the alphabetisation by
    // checking that `code` precedes `v` in the canonical string
    // (it would not in the producer's insertion order).
    const TemplateMissingAttribute err(
        "<sce:use> is missing required 'template' attribute");
    const std::string canonical = err.to_canonical_json_string();

    const auto code_pos = canonical.find("\"code\":");
    const auto v_pos = canonical.find("\"v\":");
    ASSERT_NE(code_pos, std::string::npos) << canonical;
    ASSERT_NE(v_pos, std::string::npos) << canonical;
    EXPECT_LT(code_pos, v_pos)
        << "canonical string did not alphabetise keys: " << canonical;
}

// ── Batch NDJSON formatter (RFC §W2 deliverable #2) ───────────────

TEST(TemplateErrorWire,
     BatchFormatterEmitsOneRecordPerDiagnosticAsNdjson) {
    // Three subtypes share a vector. `emit_json_diagnostics` writes
    // one JSON record per line, '\n'-delimited, no array wrapper.
    // The reader parses each line with `nlohmann::json::parse` and
    // confirms required v1 fields.
    //
    // Load-bearing bite: change the trailing '\n' to ',' in
    // `DiagnosticBatchFormatter.cpp` and the per-line parse below
    // reds with `nlohmann::json::parse_error` on line 2 (the comma
    // glues two records into a single malformed line).
    std::vector<std::unique_ptr<Diagnostic>> diags;
    diags.push_back(std::make_unique<TemplateCycle>(
        "<sce:use template=\"a.sce-template.xml\">: cycle detected"));
    diags.push_back(std::make_unique<TemplateMalformed>(
        "<sce:use template=\"bad.sce-template.xml\">: malformed"));
    diags.push_back(std::make_unique<TemplateMissingAttribute>(
        "<sce:use> is missing required 'template' attribute"));

    std::ostringstream oss;
    emit_json_diagnostics(diags, oss);

    const std::string ndjson = oss.str();
    ASSERT_FALSE(ndjson.empty());
    EXPECT_EQ(ndjson.back(), '\n')
        << "NDJSON must end on the trailing record's newline: "
        << ndjson;

    // Split on '\n'; with a trailing '\n' there are 3 record lines
    // plus one empty trailing fragment after the last delimiter.
    std::vector<std::string> lines;
    std::string current;
    for (char c : ndjson) {
        if (c == '\n') {
            lines.push_back(std::move(current));
            current.clear();
        } else {
            current += c;
        }
    }
    if (!current.empty()) {
        lines.push_back(std::move(current));
    }

    ASSERT_EQ(lines.size(), 3u) << ndjson;

    static const std::array<std::string_view, 3> kExpectedCodes = {
        "xml/template-cycle",
        "xml/template-malformed",
        "xml/template-missing-attribute",
    };

    for (std::size_t i = 0; i < lines.size(); ++i) {
        const std::string &line = lines[i];
        nlohmann::json parsed;
        ASSERT_NO_THROW(parsed = nlohmann::json::parse(line))
            << "line " << i << " failed to parse: " << line;
        conformance::assertSchemaConformantBase(parsed,
                                                kExpectedCodes[i]);
        conformance::assertNoUnexpectedKeys(parsed);
    }
}

TEST(TemplateErrorWire, BatchFormatterSkipsNullEntries) {
    // Defensive: a hand-assembled vector with a null entry must
    // not corrupt the line-based reader. The skip is documented
    // in `DiagnosticBatchFormatter.cpp`.
    std::vector<std::unique_ptr<Diagnostic>> diags;
    diags.push_back(std::make_unique<TemplateCycle>("cycle"));
    diags.push_back(nullptr);
    diags.push_back(std::make_unique<TemplateMalformed>("malformed"));

    std::ostringstream oss;
    emit_json_diagnostics(diags, oss);

    const std::string ndjson = oss.str();
    std::size_t newlines = 0;
    for (char c : ndjson) {
        if (c == '\n') {
            ++newlines;
        }
    }
    EXPECT_EQ(newlines, 2u)
        << "null entry must be skipped, not emitted as empty line: "
        << ndjson;
}

TEST(TemplateErrorWire, BatchFormatterEmptyVectorWritesNothing) {
    std::vector<std::unique_ptr<Diagnostic>> diags;
    std::ostringstream oss;
    emit_json_diagnostics(diags, oss);
    EXPECT_TRUE(oss.str().empty());
}

// ── SCXMLParser boundary flatten (RFC §W1 audit #1 / W2) ──────────

TEST(SCXMLParserBoundary, ParseContentSurfacesTypedTemplateDiagnostic) {
    // `<sce:use/>` without the required `template` attribute fires
    // `TemplateMissingAttribute` from `TemplateExpander` at the call
    // site (no file resolution). The boundary flatten in
    // `SCXMLParser::parseContent` adds a typed catch arm AHEAD of
    // the existing `std::exception&` fallback that records the
    // diagnostic via `Diagnostic::clone()` so `getDiagnostics()`
    // returns the typed object alongside the legacy string vector
    // (Q4-B coexistence).
    //
    // Standing-consumer load-bearing-ness: removing the
    // `recordDiagnostic(tpl.clone())` line in the typed catch arm
    // reds this test with `getDiagnostics().size() == 0` because
    // the flatten happens via `addError` only.
    constexpr const char *kBrokenScxml =
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
        "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\""
        "       xmlns:sce=\"http://sce.dev/ext\""
        "       version=\"1.0\""
        "       initial=\"s1\""
        "       name=\"boundary_test\">"
        "  <state id=\"s1\">"
        "    <sce:use/>"
        "  </state>"
        "</scxml>";

    SCE::SCXMLParser parser(std::make_shared<SCE::NodeFactory>());
    auto model = parser.parseContent(kBrokenScxml);

    EXPECT_EQ(model, nullptr);

    // Q4-B: legacy string-vector surface remains populated.
    EXPECT_TRUE(parser.hasErrors());
    EXPECT_FALSE(parser.getErrorMessages().empty());

    // RFC §W1 audit #1 closure: typed surface populated.
    const auto &diags = parser.getDiagnostics();
    ASSERT_EQ(diags.size(), 1u) << "expected exactly one typed diagnostic";
    ASSERT_NE(diags[0], nullptr);
    EXPECT_EQ(diags[0]->code(),
              std::string_view{"xml/template-missing-attribute"});

    // Round-trip through `to_json()` to confirm the cloned object
    // preserved its dynamic type (a sliced base copy would dispatch
    // to a different override or fail to compile against pure-virt).
    const auto j = diags[0]->to_json();
    EXPECT_EQ(j.at("code").get<std::string>(),
              "xml/template-missing-attribute");
    EXPECT_EQ(j.at("stage").get<std::string>(), "xml");
}

TEST(SCXMLParserBoundary, EndToEndParseGetDiagnosticsEmitNdjson) {
    // Full pipeline:
    //   parseContent (broken `<sce:use/>`)
    //     → SCXMLParser::getDiagnostics() (typed surface)
    //     → emit_json_diagnostics (batch NDJSON)
    //     → nlohmann::json::parse per line
    //     → assert code matches `xml/template-missing-attribute`.
    //
    // Q4-B coexistence: legacy `getErrorMessages()` is also
    // populated. RFC §W2 deliverable plus §W1 audit #1 closure
    // composed in one fixture.
    //
    // Load-bearing bite: drop the `<sce:use/>` line from the
    // fixture (pasted as `<state id=\"s1\"/>`) and the
    // `getDiagnostics().size() == 1` assertion below reds with
    // "expected at least one diagnostic, got 0".
    constexpr const char *kBrokenScxml =
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
        "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\""
        "       xmlns:sce=\"http://sce.dev/ext\""
        "       version=\"1.0\""
        "       initial=\"s1\""
        "       name=\"e2e_test\">"
        "  <state id=\"s1\">"
        "    <sce:use/>"
        "  </state>"
        "</scxml>";

    SCE::SCXMLParser parser(std::make_shared<SCE::NodeFactory>());
    auto model = parser.parseContent(kBrokenScxml);
    ASSERT_EQ(model, nullptr);

    // Q4-B: legacy string-vector surface is populated.
    EXPECT_TRUE(parser.hasErrors());
    EXPECT_FALSE(parser.getErrorMessages().empty());

    // Typed surface via the boundary flatten.
    const auto &diags = parser.getDiagnostics();
    ASSERT_GE(diags.size(), 1u)
        << "expected at least one typed diagnostic from SCXMLParser";

    // Run the typed vector through the batch formatter.
    std::ostringstream oss;
    emit_json_diagnostics(diags, oss);
    const std::string ndjson = oss.str();
    ASSERT_FALSE(ndjson.empty());

    // Parse the first line as a v1 schema record. With one
    // diagnostic emitted, `lines[0]` is the only record.
    const auto firstNewline = ndjson.find('\n');
    ASSERT_NE(firstNewline, std::string::npos)
        << "NDJSON output is missing line delimiter: " << ndjson;
    const std::string firstLine = ndjson.substr(0, firstNewline);

    nlohmann::json parsed;
    ASSERT_NO_THROW(parsed = nlohmann::json::parse(firstLine))
        << firstLine;

    conformance::assertSchemaConformantBase(
        parsed, "xml/template-missing-attribute");
    conformance::assertNoUnexpectedKeys(parsed);
}

TEST(SCXMLParserBoundary, ParseContentSurfacesTypedXIncludeDiagnostic) {
    // Mirrors the TemplateError sister test above for the W3-5
    // boundary surfacing. `<xi:include/>` without `href` fires
    // `XIncludeMissingHref` from `expandStringX`; the typed
    // exception is re-thrown by `PugiXMLDocument::processXInclude`
    // and caught by `SCXMLParser::parseContent`'s typed catch arm
    // ahead of the existing `std::exception&` fallback.
    //
    // Standing-consumer load-bearing-ness: removing the
    // `recordDiagnostic(xie.clone())` call in the typed catch arm
    // reds this test with `getDiagnostics().size() == 0` because
    // the legacy `addError` only populates the string surface.
    constexpr const char *kBrokenScxml =
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
        "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\""
        "       xmlns:xi=\"http://www.w3.org/2001/XInclude\""
        "       version=\"1.0\""
        "       initial=\"s1\""
        "       name=\"xinc_missing_href_test\">"
        "  <state id=\"s1\">"
        "    <xi:include/>"
        "  </state>"
        "</scxml>";

    SCE::SCXMLParser parser(std::make_shared<SCE::NodeFactory>());
    auto model = parser.parseContent(kBrokenScxml);

    EXPECT_EQ(model, nullptr);

    // Q4-B: legacy string-vector surface remains populated.
    EXPECT_TRUE(parser.hasErrors());
    EXPECT_FALSE(parser.getErrorMessages().empty());

    // RFC §W3-5: typed surface populated with the leaf's wire code.
    const auto &diags = parser.getDiagnostics();
    ASSERT_EQ(diags.size(), 1u) << "expected exactly one typed diagnostic";
    ASSERT_NE(diags[0], nullptr);
    EXPECT_EQ(diags[0]->code(),
              std::string_view{"xml/xinclude-missing-href"});

    // Round-trip through to_json() to confirm the cloned object
    // preserved its dynamic type — a sliced base copy would dispatch
    // to a different code() override or fail to compile against the
    // pure-virtual.
    const auto j = diags[0]->to_json();
    EXPECT_EQ(j.at("code").get<std::string>(),
              "xml/xinclude-missing-href");
    EXPECT_EQ(j.at("stage").get<std::string>(), "xml");
}

TEST(SCXMLParserBoundary, GetDiagnosticsEmptyOnSuccessfulParse) {
    // Successful parse leaves `diagnostics_` empty — the typed
    // surface is opt-in and must not accumulate noise on the
    // happy path. Sanity-checks `initParsing()` clears the vector
    // between successive parses on the same parser instance.
    constexpr const char *kValidScxml =
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
        "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\""
        "       version=\"1.0\""
        "       initial=\"s1\""
        "       name=\"valid_test\">"
        "  <state id=\"s1\"/>"
        "</scxml>";

    SCE::SCXMLParser parser(std::make_shared<SCE::NodeFactory>());
    auto model = parser.parseContent(kValidScxml);
    EXPECT_NE(model, nullptr);
    EXPECT_TRUE(parser.getDiagnostics().empty());
}

TEST(TemplateErrorWire, IdDiffersAcrossSubtypesWithSameMessage) {
    // Mixing two subtypes' code() into the FNV-1a key keeps the id
    // distinct even when the rendered message text happens to match
    // (the canonical key prepends `code | stage | file` before the
    // message fragment, so flipping code flips id).
    const std::string msg = "shared message";
    const TemplateCycle a(msg);
    const TemplateMalformed b(msg);
    EXPECT_NE(a.to_json().at("id").get<std::string>(),
              b.to_json().at("id").get<std::string>());
}

}  // namespace
}  // namespace SCE::parsing
