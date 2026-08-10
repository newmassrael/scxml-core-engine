// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "factory/NodeFactory.h"
#include "parsing/Diagnostic.h"
#include "parsing/DiagnosticBatchFormatter.h"
#include "parsing/ParseError.h"
#include "parsing/SCXMLParser.h"
#include "parsing/SemanticError.h"
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
// `TemplateError` refit (§wire-W1 commit-series). Two layers:
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
    EXPECT_EQ(erased.to_json()["code"].get<std::string>(), "xml/template-cycle");
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

bool matchesGeneratorRegex(const std::string &generator) {
    static const std::regex kPattern{R"(^([0-9a-f]{7,40}|unknown)$)"};
    return std::regex_match(generator, kPattern);
}

void assertSchemaConformantBase(const nlohmann::ordered_json &j, std::string_view expectedCode) {
    // v1 schema requires v/id/generator/code/stage/message; spec/location/
    // expected/actual/fix are optional. The C++ producer in W1 emits
    // exactly the required set (plus location when populated); the
    // assertions below pin both presence AND absence so a future
    // additive field surfaces as a curated-set drift.
    ASSERT_TRUE(j.contains("v")) << j.dump();
    ASSERT_TRUE(j.contains("id")) << j.dump();
    ASSERT_TRUE(j.contains("generator")) << j.dump();
    ASSERT_TRUE(j.contains("code")) << j.dump();
    ASSERT_TRUE(j.contains("stage")) << j.dump();
    ASSERT_TRUE(j.contains("message")) << j.dump();

    EXPECT_EQ(j.at("v").get<int>(), 1);
    EXPECT_EQ(j.at("code").get<std::string>(), std::string{expectedCode});
    EXPECT_EQ(j.at("stage").get<std::string>(), "xml");
    EXPECT_TRUE(matchesIdRegex(j.at("id").get<std::string>()))
        << "id '" << j.at("id").get<std::string>() << "' does not match ^fnv1a:[0-9a-f]{16}$";
    // The header declares this side conforms to the shared schema
    // field-for-field. `generator` became required there because a
    // rejected run writes no manifest, leaving the diagnostic as the
    // only record a consumer receives — a C++ producer that omitted it
    // would emit records the schema rejects while this fixture stayed
    // green, which is exactly the drift the curated list exists to stop.
    EXPECT_TRUE(matchesGeneratorRegex(j.at("generator").get<std::string>()))
        << "generator '" << j.at("generator").get<std::string>() << "' does not match ^([0-9a-f]{7,40}|unknown)$";
    EXPECT_FALSE(j.at("message").get<std::string>().empty());
}

// Curated set of legal top-level keys (mirrors v1 schema's
// `additionalProperties: false` constraint at the envelope level —
// nlohmann::json does not validate this for us, so we enforce it
// explicitly here so a stray field added on the C++ side reds.
const std::set<std::string> kAllowedTopLevelKeys = {
    "v", "id", "generator", "code", "stage", "spec", "message", "location", "expected", "actual", "fix",
};

void assertNoUnexpectedKeys(const nlohmann::ordered_json &j) {
    for (const auto &item : j.items()) {
        EXPECT_TRUE(kAllowedTopLevelKeys.count(item.key()) == 1) << "unexpected top-level key: '" << item.key() << "'";
    }
}

}  // namespace conformance

TEST(TemplateErrorWire, TemplateNotFoundConformsToV1Schema) {
    const TemplateNotFound err("<sce:use template=\"missing.sce-template.xml\">: file not "
                               "found (searched: /project/missing.sce-template.xml)");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/template-not-found");
    conformance::assertNoUnexpectedKeys(j);
    EXPECT_FALSE(j.contains("location"));
}

TEST(TemplateErrorWire, TemplateReadErrorConformsToV1Schema) {
    const TemplateReadError err("<sce:use template=\"unreadable.sce-template.xml\">: read error: "
                                "permission denied");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/template-read-error");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(TemplateErrorWire, TemplateMalformedConformsToV1Schema) {
    const TemplateMalformed err("<sce:use template=\"bad.sce-template.xml\">: template is "
                                "malformed: expanded template is malformed: unexpected end of file");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/template-malformed");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(TemplateErrorWire, TemplateMissingAttributeConformsToV1Schema) {
    const TemplateMissingAttribute err("<sce:use> is missing required 'template' attribute");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/template-missing-attribute");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(TemplateErrorWire, TemplateMissingParamConformsToV1Schema) {
    const TemplateMissingParam err("<sce:use template=\"guard.sce-template.xml\">: missing required "
                                   "parameter 'condition'");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/template-missing-param");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(TemplateErrorWire, TemplateUnknownParamConformsToV1Schema) {
    const TemplateUnknownParam err("<sce:use template=\"guard.sce-template.xml\">: unknown parameter "
                                   "'state' (declared: condition, action)");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/template-unknown-param");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(TemplateErrorWire, TemplateCycleConformsToV1Schema) {
    const TemplateCycle err("<sce:use template=\"a.sce-template.xml\">: cycle detected "
                            "(/a.sce-template.xml -> /b.sce-template.xml -> /a.sce-template.xml)");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/template-cycle");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(TemplateErrorWire, TemplateTooDeepConformsToV1Schema) {
    const TemplateTooDeep err("<sce:use>: template depth limit (32) exceeded");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/template-too-deep");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(TemplateErrorWire, EveryCuratedCodeIsExercised) {
    // Sanity-check the curated list mirrors the 8 subtypes above.
    // Adding a 9th wire code without a corresponding test reds here.
    EXPECT_EQ(conformance::kExpectedCodes.size(), 8u);
    std::set<std::string_view> uniq(conformance::kExpectedCodes.begin(), conformance::kExpectedCodes.end());
    EXPECT_EQ(uniq.size(), conformance::kExpectedCodes.size()) << "duplicate entry in kExpectedCodes";
}

TEST(TemplateErrorWire, LocationFieldShapeWhenPresent) {
    TemplateCycle err("<sce:use template=\"a.sce-template.xml\">: cycle detected");
    err.setLocation(SourcePos{std::filesystem::path{"/project/main.scxml"}, 12u, 5u});

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
    static const std::set<std::string> kAllowedLocationKeys = {"file", "line", "col"};
    for (const auto &item : loc.items()) {
        EXPECT_TRUE(kAllowedLocationKeys.count(item.key()) == 1) << "unexpected location key: '" << item.key() << "'";
    }
}

TEST(TemplateErrorWire, IdIsStableAcrossCalls) {
    // Identity is content-addressed (§wire-W1 / SCE_ERROR_CONTRACT.md):
    // re-rendering the same logical error must not shift its id.
    const TemplateCycle err("<sce:use template=\"a.sce-template.xml\">: cycle detected");
    EXPECT_EQ(err.to_json().at("id").get<std::string>(), err.to_json().at("id").get<std::string>());
}

// ── XInclude wire-layer probes ────────────────────────────────────
//
// Parallel structure to `TemplateErrorWire` above for the
// `xml/xinclude-*` Rust DiagnosticCode family (7 codes pinned in
// `schemas/sce-diagnostic.v1.schema.json` lines 27-32 and
// `sce-build/src/forge/diagnostic.rs::DiagnosticCode::Xml*`).
//
// `XIncludeExpansionError` implements `SCE::parsing::Diagnostic`
// (§wire-W3)
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
    const std::string src = R"(<root><xi:include xmlns:xi="http://www.w3.org/2001/XInclude"/></root>)";
    try {
        SCE::parsing::expandStringX(src, "inline", "", {});
        FAIL() << "expandStringX must throw on missing href";
    } catch (const SCE::parsing::XIncludeExpansionError &e) {
        const std::string what = e.what();
        EXPECT_NE(what.find("href"), std::string::npos) << what;
        EXPECT_EQ(e.code(), std::string_view{"xml/xinclude-missing-href"});
    }
}

TEST(XIncludeErrorWire, EmptyHrefCarriesActionableFragmentInMessage) {
    const std::string src = R"(<root><xi:include xmlns:xi="http://www.w3.org/2001/XInclude" href=""/></root>)";
    try {
        SCE::parsing::expandStringX(src, "inline", "", {});
        FAIL() << "expandStringX must throw on empty href";
    } catch (const SCE::parsing::XIncludeExpansionError &e) {
        const std::string what = e.what();
        EXPECT_NE(what.find("href"), std::string::npos) << what;
        EXPECT_EQ(e.code(), std::string_view{"xml/xinclude-missing-href"});
    }
}

// ── v1 schema conformance for XIncludeError subtypes ──────────────
//
// Mirror of the TemplateError schema-conformance tests above for the
// 7 typed xinclude leaves promoted in §wire-W3. Each test constructs
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
    "xml/xinclude-missing-href", "xml/xinclude-not-found", "xml/xinclude-read-error",  "xml/xinclude-cycle",
    "xml/xinclude-too-deep",     "xml/xinclude-malformed", "xml/xinclude-unsupported",
};

}  // namespace conformance

TEST(XIncludeErrorWire, MissingHrefConformsToV1Schema) {
    const XIncludeMissingHref err("<xi:include> missing or empty `href` attribute");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/xinclude-missing-href");
    conformance::assertNoUnexpectedKeys(j);
    EXPECT_FALSE(j.contains("location"));
}

TEST(XIncludeErrorWire, NotFoundConformsToV1Schema) {
    const XIncludeNotFound err("<xi:include href=\"missing.xml\">: file not found "
                               "(searched: /project/missing.xml)");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/xinclude-not-found");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(XIncludeErrorWire, ReadErrorConformsToV1Schema) {
    const XIncludeReadError err("<xi:include href=\"frag.xml\">: cannot read file: "
                                "/project/frag.xml");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/xinclude-read-error");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(XIncludeErrorWire, CycleConformsToV1Schema) {
    const XIncludeCycle err("<xi:include href=\"a.xml\">: cycle detected "
                            "(/a.xml -> /b.xml -> /a.xml)");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/xinclude-cycle");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(XIncludeErrorWire, TooDeepConformsToV1Schema) {
    const XIncludeTooDeep err("<xi:include> nesting exceeds depth limit of 10");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/xinclude-too-deep");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(XIncludeErrorWire, MalformedConformsToV1Schema) {
    const XIncludeMalformed err("<xi:include href=\"bad.xml\">: included file is malformed: "
                                "unexpected end of file");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/xinclude-malformed");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(XIncludeErrorWire, UnsupportedConformsToV1Schema) {
    const XIncludeUnsupported err("<xi:include href=\"frag.xml\">: unsupported feature: "
                                  "parse=\"text\" (only parse=\"xml\" is supported)");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/xinclude-unsupported");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(XIncludeErrorWire, EveryCuratedXIncludeCodeIsExercised) {
    // Sanity-check the curated list mirrors the 7 subtypes above.
    // Adding an 8th wire code without a corresponding test reds here.
    EXPECT_EQ(conformance::kExpectedXIncludeCodes.size(), 7u);
    std::set<std::string_view> uniq(conformance::kExpectedXIncludeCodes.begin(),
                                    conformance::kExpectedXIncludeCodes.end());
    EXPECT_EQ(uniq.size(), conformance::kExpectedXIncludeCodes.size()) << "duplicate entry in kExpectedXIncludeCodes";
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
    EXPECT_NE(a.to_json().at("id").get<std::string>(), b.to_json().at("id").get<std::string>());
}

// ── Canonical-JSON string (§wire-W2 deliverable #3) ────────────────

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
    const TemplateCycle err("<sce:use template=\"a.sce-template.xml\">: cycle detected "
                            "(/a.sce-template.xml -> /b.sce-template.xml -> /a.sce-template.xml)");

    const std::string a = err.to_canonical_json_string();
    const std::string b = err.to_canonical_json_string();
    EXPECT_EQ(a, b);

    // Required v1 fields are present in the canonical string.
    EXPECT_NE(a.find("\"v\":1"), std::string::npos) << a;
    EXPECT_NE(a.find("\"code\":\"xml/template-cycle\""), std::string::npos) << a;
    EXPECT_NE(a.find("\"stage\":\"xml\""), std::string::npos) << a;

    // No-whitespace contract: dump(-1, ' ', false) emits compact
    // form. A pretty-print regression introduces literal '\n' or
    // indent spacing.
    EXPECT_EQ(a.find('\n'), std::string::npos) << a;
    EXPECT_EQ(a.find("\": "), std::string::npos) << "found ': ' (with space) — canonical dump must be "
                                                    "compact: "
                                                 << a;
}

TEST(TemplateErrorWire, CanonicalJsonStringHasAlphabeticalKeyOrder) {
    // Producer-side `to_json()` emits keys in insertion order
    // (v, id, code, stage, message, location). Canonicalisation
    // re-parses through std::map so the byte stream lands keys
    // alphabetically — `code`, `id`, `message`, `stage`, `v` for
    // the location-less subtypes. Pin the alphabetisation by
    // checking that `code` precedes `v` in the canonical string
    // (it would not in the producer's insertion order).
    const TemplateMissingAttribute err("<sce:use> is missing required 'template' attribute");
    const std::string canonical = err.to_canonical_json_string();

    const auto code_pos = canonical.find("\"code\":");
    const auto v_pos = canonical.find("\"v\":");
    ASSERT_NE(code_pos, std::string::npos) << canonical;
    ASSERT_NE(v_pos, std::string::npos) << canonical;
    EXPECT_LT(code_pos, v_pos) << "canonical string did not alphabetise keys: " << canonical;
}

// ── Batch NDJSON formatter (§wire-W2 deliverable #2) ───────────────

TEST(TemplateErrorWire, BatchFormatterEmitsOneRecordPerDiagnosticAsNdjson) {
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
    diags.push_back(std::make_unique<TemplateCycle>("<sce:use template=\"a.sce-template.xml\">: cycle detected"));
    diags.push_back(std::make_unique<TemplateMalformed>("<sce:use template=\"bad.sce-template.xml\">: malformed"));
    diags.push_back(std::make_unique<TemplateMissingAttribute>("<sce:use> is missing required 'template' attribute"));

    std::ostringstream oss;
    emit_json_diagnostics(diags, oss);

    const std::string ndjson = oss.str();
    ASSERT_FALSE(ndjson.empty());
    EXPECT_EQ(ndjson.back(), '\n') << "NDJSON must end on the trailing record's newline: " << ndjson;

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
        ASSERT_NO_THROW(parsed = nlohmann::json::parse(line)) << "line " << i << " failed to parse: " << line;
        conformance::assertSchemaConformantBase(parsed, kExpectedCodes[i]);
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
    EXPECT_EQ(newlines, 2u) << "null entry must be skipped, not emitted as empty line: " << ndjson;
}

TEST(TemplateErrorWire, BatchFormatterEmptyVectorWritesNothing) {
    std::vector<std::unique_ptr<Diagnostic>> diags;
    std::ostringstream oss;
    emit_json_diagnostics(diags, oss);
    EXPECT_TRUE(oss.str().empty());
}

// ── SCXMLParser boundary flatten (§wire-W1 audit #1 / W2) ──────────

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
    constexpr const char *kBrokenScxml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
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

    // §wire-W1 audit #1 closure: typed surface populated.
    const auto &diags = parser.getDiagnostics();
    ASSERT_EQ(diags.size(), 1u) << "expected exactly one typed diagnostic";
    ASSERT_NE(diags[0], nullptr);
    EXPECT_EQ(diags[0]->code(), std::string_view{"xml/template-missing-attribute"});

    // Round-trip through `to_json()` to confirm the cloned object
    // preserved its dynamic type (a sliced base copy would dispatch
    // to a different override or fail to compile against pure-virt).
    const auto j = diags[0]->to_json();
    EXPECT_EQ(j.at("code").get<std::string>(), "xml/template-missing-attribute");
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
    // populated. §wire-W2 deliverable plus §W1 audit #1 closure
    // composed in one fixture.
    //
    // Load-bearing bite: drop the `<sce:use/>` line from the
    // fixture (pasted as `<state id=\"s1\"/>`) and the
    // `getDiagnostics().size() == 1` assertion below reds with
    // "expected at least one diagnostic, got 0".
    constexpr const char *kBrokenScxml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
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
    ASSERT_GE(diags.size(), 1u) << "expected at least one typed diagnostic from SCXMLParser";

    // Run the typed vector through the batch formatter.
    std::ostringstream oss;
    emit_json_diagnostics(diags, oss);
    const std::string ndjson = oss.str();
    ASSERT_FALSE(ndjson.empty());

    // Parse the first line as a v1 schema record. With one
    // diagnostic emitted, `lines[0]` is the only record.
    const auto firstNewline = ndjson.find('\n');
    ASSERT_NE(firstNewline, std::string::npos) << "NDJSON output is missing line delimiter: " << ndjson;
    const std::string firstLine = ndjson.substr(0, firstNewline);

    nlohmann::json parsed;
    ASSERT_NO_THROW(parsed = nlohmann::json::parse(firstLine)) << firstLine;

    conformance::assertSchemaConformantBase(parsed, "xml/template-missing-attribute");
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
    constexpr const char *kBrokenScxml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
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

    // §wire-W3: typed surface populated with the leaf's wire code.
    const auto &diags = parser.getDiagnostics();
    ASSERT_EQ(diags.size(), 1u) << "expected exactly one typed diagnostic";
    ASSERT_NE(diags[0], nullptr);
    EXPECT_EQ(diags[0]->code(), std::string_view{"xml/xinclude-missing-href"});

    // Round-trip through to_json() to confirm the cloned object
    // preserved its dynamic type — a sliced base copy would dispatch
    // to a different code() override or fail to compile against the
    // pure-virtual.
    const auto j = diags[0]->to_json();
    EXPECT_EQ(j.at("code").get<std::string>(), "xml/xinclude-missing-href");
    EXPECT_EQ(j.at("stage").get<std::string>(), "xml");
}

TEST(SCXMLParserBoundary, GetDiagnosticsEmptyOnSuccessfulParse) {
    // Successful parse leaves `diagnostics_` empty — the typed
    // surface is opt-in and must not accumulate noise on the
    // happy path. Sanity-checks `initParsing()` clears the vector
    // between successive parses on the same parser instance.
    constexpr const char *kValidScxml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
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
    EXPECT_NE(a.to_json().at("id").get<std::string>(), b.to_json().at("id").get<std::string>());
}

// ── v1 schema conformance for ParseError subtypes (W4 α-strict) ──
//
// Mirror of the TemplateError + XIncludeError schema-conformance
// tests above for the 5 typed parser-entry leaves promoted in RFC
// §W4. Each test constructs a leaf with an example message and
// asserts the to_json() envelope passes the shared
// `assertSchemaConformantBase` + `assertNoUnexpectedKeys` curated
// checks against `schemas/sce-diagnostic.v1.schema.json`.
//
// Three of the five leaves (ParseXmlFailed, ParseException,
// ParseNoRootElement) reuse the existing `xml/parse` wire code
// because the Rust error model has no distinct producer for those
// scenarios — see `ParseError.h` per-leaf comments and §wire-W4 D2.
// Schema conformance still applies per-leaf (each `to_json()` must
// pass) even when the wire code is shared.

namespace conformance {

// Curated mirror of the **NEW** `xml/*` entries added in W4 to
// `schemas/sce-diagnostic.v1.schema.json`. The 3 reused-code leaves
// (ParseXmlFailed, ParseException, ParseNoRootElement) all return
// `xml/parse`, which is already on `kExpectedCodes` for the
// TemplateError/XmlError families. Listing them here would
// double-count and obscure the W4-specific surface — only the new
// codes belong in this curated set. Pinned cross-side by the W4
// Rust drift test
// `cpp_parse_subtypes_match_rust_diagnostic_codes` in
// `sce-build/src/parser.rs::tests` (§wire-W4 Stage D).
const std::array<std::string_view, 2> kExpectedNewParseCodes = {
    "xml/file-not-found",
    "xml/wrong-root-element",
};

}  // namespace conformance

TEST(ParseErrorWire, FileNotFoundConformsToV1Schema) {
    const ParseFileNotFound err("File not found: /nonexistent/path.scxml");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/file-not-found");
    conformance::assertNoUnexpectedKeys(j);
    EXPECT_FALSE(j.contains("location"));
}

TEST(ParseErrorWire, ParseXmlFailedConformsToV1Schema) {
    const ParseXmlFailed err("Parse error: unexpected end tag </scxml> at offset 42");
    const auto j = err.to_json();
    // Reuses xml/parse (no Rust producer for a distinct parser-entry
    // code; pugi err detail is embedded in the message text).
    conformance::assertSchemaConformantBase(j, "xml/parse");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(ParseErrorWire, ParseExceptionConformsToV1Schema) {
    const ParseException err("Exception while parsing file: out of memory");
    const auto j = err.to_json();
    // Reuses xml/parse (Rust has no exception model — Result-based).
    conformance::assertSchemaConformantBase(j, "xml/parse");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(ParseErrorWire, NoRootElementConformsToV1Schema) {
    const ParseNoRootElement err("No root element found");
    const auto j = err.to_json();
    // Reuses xml/parse (roxmltree rejects root-less input at parse
    // time, so no Rust producer for this specific scenario).
    conformance::assertSchemaConformantBase(j, "xml/parse");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(ParseErrorWire, WrongRootElementConformsToV1Schema) {
    const ParseWrongRootElement err("Root element is not 'scxml', found: html");
    const auto j = err.to_json();
    conformance::assertSchemaConformantBase(j, "xml/wrong-root-element");
    conformance::assertNoUnexpectedKeys(j);
}

TEST(ParseErrorWire, EveryNewCuratedParseCodeIsExercised) {
    // Sanity-check the curated list mirrors the 2 NEW W4 wire codes.
    // Adding a 3rd new wire code without a corresponding curated
    // entry reds here.
    EXPECT_EQ(conformance::kExpectedNewParseCodes.size(), 2u);
    std::set<std::string_view> uniq(conformance::kExpectedNewParseCodes.begin(),
                                    conformance::kExpectedNewParseCodes.end());
    EXPECT_EQ(uniq.size(), conformance::kExpectedNewParseCodes.size()) << "duplicate entry in kExpectedNewParseCodes";
}

TEST(ParseErrorWire, IdDiffersAcrossSubtypesWithSameMessage) {
    // Mixing two subtypes' code() into the FNV-1a key keeps the id
    // distinct even when the rendered message text happens to match.
    // Tests across leaves with DIFFERENT wire codes
    // (file-not-found vs wrong-root-element) — the 3 reused-code
    // leaves (ParseXmlFailed/Exception/NoRootElement) intentionally
    // share `xml/parse` and would yield identical ids on identical
    // messages, which is the documented α-strict tradeoff.
    const std::string msg = "shared message";
    const ParseFileNotFound a(msg);
    const ParseWrongRootElement b(msg);
    EXPECT_NE(a.to_json().at("id").get<std::string>(), b.to_json().at("id").get<std::string>());
}

// ── SCXMLParser boundary surfacing for parser-entry leaves ────────

TEST(SCXMLParserBoundary, ParseFileSurfacesTypedFileNotFoundDiagnostic) {
    // `parseFile` against a non-existent path fires
    // `PugiXMLParser::parseFile`'s `ParseFileNotFound` typed throw
    // (D1-C); SCXMLParser's `catch (const ParseError &pe)` arm
    // surfaces it on `getDiagnostics()` while populating the legacy
    // string vector for Q4-B coexistence.
    //
    // Standing-consumer load-bearing-ness: removing the
    // `recordDiagnostic(pe.clone())` line in the typed catch arm
    // reds this test with `getDiagnostics().size() == 0` (the
    // legacy `addError` only populates the string surface).
    SCE::SCXMLParser parser(std::make_shared<SCE::NodeFactory>());
    auto model = parser.parseFile("/this/path/should/not/exist/foo.scxml");

    EXPECT_EQ(model, nullptr);

    // Q4-B: legacy string-vector surface populated.
    EXPECT_TRUE(parser.hasErrors());
    EXPECT_FALSE(parser.getErrorMessages().empty());

    // §wire-W4 D1-C: typed surface populated with the leaf's wire code.
    const auto &diags = parser.getDiagnostics();
    ASSERT_EQ(diags.size(), 1u) << "expected exactly one typed diagnostic";
    ASSERT_NE(diags[0], nullptr);
    EXPECT_EQ(diags[0]->code(), std::string_view{"xml/file-not-found"});

    // Round-trip through to_json() to confirm the cloned object
    // preserved its dynamic type — a sliced base copy would dispatch
    // to a different code() override or fail to compile against the
    // pure-virtual.
    const auto j = diags[0]->to_json();
    EXPECT_EQ(j.at("code").get<std::string>(), "xml/file-not-found");
    EXPECT_EQ(j.at("stage").get<std::string>(), "xml");
}

TEST(SCXMLParserBoundary, ParseContentSurfacesTypedWrongRootElementDiagnostic) {
    // `parseContent` against a document whose root tag is not
    // `<scxml>` reaches `parseAbstractDocument`'s root-tag check,
    // which throws `ParseWrongRootElement` (D1-C). SCXMLParser's
    // typed catch arm surfaces it on `getDiagnostics()`.
    //
    // Standing-consumer load-bearing-ness: removing the root-tag
    // check in `parseAbstractDocument` would silently produce an
    // empty model rather than a typed diagnostic — exactly the
    // failure mode the W4 typed surface exists to catch
    // (`feedback_silently_broken_hooks.md`).
    constexpr const char *kWrongRootScxml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
                                            "<not-scxml/>";

    SCE::SCXMLParser parser(std::make_shared<SCE::NodeFactory>());
    auto model = parser.parseContent(kWrongRootScxml);

    EXPECT_EQ(model, nullptr);

    EXPECT_TRUE(parser.hasErrors());
    EXPECT_FALSE(parser.getErrorMessages().empty());

    const auto &diags = parser.getDiagnostics();
    ASSERT_EQ(diags.size(), 1u);
    ASSERT_NE(diags[0], nullptr);
    EXPECT_EQ(diags[0]->code(), std::string_view{"xml/wrong-root-element"});

    const auto j = diags[0]->to_json();
    EXPECT_EQ(j.at("code").get<std::string>(), "xml/wrong-root-element");
}

// ── Consumer-fragility tests (load-bearing — §wire-W4 D8) ──────────
//
// These two tests codify what the typed surface UNLOCKS: behavior
// that string-parsing cannot deliver robustly. Without them, W4 is
// `feedback_built_but_unconsumed.md` — surface exists but no caller
// distinguishes it from string parsing. The dispatch lambda in
// `TypedCodeDistinguishesFailureClassWhereStringParsingIsFragile`
// IS the consumer; the surface IS what makes it possible.

TEST(ParseErrorConsumer, TypedCodeDistinguishesFailureClassWhereStringParsingIsFragile) {
    // Two distinct parse failures — file-not-found (path retry
    // strategy) vs wrong-root-element (syntax suggestion strategy).
    // A real consumer (LSP / CI report / build tool / agent) needs
    // to dispatch on the failure CLASS, not on the message text.
    // This test proves typed code() makes that dispatch reliable;
    // the parallel string-parsing path would have to
    // startsWith("File not found:") and would silently break if a
    // future PR edited the message text.
    SCE::SCXMLParser p1(std::make_shared<SCE::NodeFactory>());
    p1.parseFile("/nonexistent/path.scxml");

    SCE::SCXMLParser p2(std::make_shared<SCE::NodeFactory>());
    constexpr const char *kWrongRoot = "<?xml version=\"1.0\"?><not-scxml/>";
    p2.parseContent(kWrongRoot);

    ASSERT_EQ(p1.getDiagnostics().size(), 1u);
    ASSERT_EQ(p2.getDiagnostics().size(), 1u);

    // THE consumer pattern — typed dispatch:
    auto retry_strategy = [](const Diagnostic &d) -> std::string {
        if (d.code() == std::string_view{"xml/file-not-found"}) {
            return "PATH_RETRY";
        }
        if (d.code() == std::string_view{"xml/wrong-root-element"}) {
            return "SYNTAX_FIX";
        }
        return "GENERIC";
    };
    EXPECT_EQ(retry_strategy(*p1.getDiagnostics()[0]), "PATH_RETRY");
    EXPECT_EQ(retry_strategy(*p2.getDiagnostics()[0]), "SYNTAX_FIX");
}

TEST(ParseErrorConsumer, TypedCodeStableUnderMessageTextEdit) {
    // Codifies that `code()` IS the wire-stable handle. Construct
    // two `ParseFileNotFound` instances with intentionally divergent
    // message-text pretexts (one as if from a future PR edit).
    // Assert `code()` is byte-identical across both, while message()
    // diverges. Bites if a future PR changes wire codes by editing
    // message text — the typed code() returns the same wire string
    // regardless of the message rendering choice.
    const ParseFileNotFound today("File not found: /tmp/foo.scxml");
    const ParseFileNotFound future_edit("ENOENT: cannot locate /tmp/foo.scxml on filesystem");

    EXPECT_EQ(today.code(), future_edit.code()) << "wire code() must be invariant under message-text edits";
    EXPECT_EQ(today.code(), std::string_view{"xml/file-not-found"});

    // Messages diverge.
    EXPECT_NE(std::string(today.what()), std::string(future_edit.what()));

    // Demonstrate the consumer's actual asymmetry: typed dispatch
    // works on both; string-parsing-on-message would only catch one.
    auto typed_dispatch = [](const ParseError &e) { return e.code() == std::string_view{"xml/file-not-found"}; };
    EXPECT_TRUE(typed_dispatch(today));
    EXPECT_TRUE(typed_dispatch(future_edit));

    // For comparison: a fragile string-parsing alternative.
    auto fragile_string_dispatch = [](const std::exception &e) {
        const std::string m = e.what();
        return m.rfind("File not found:", 0) == 0;
    };
    EXPECT_TRUE(fragile_string_dispatch(today));
    EXPECT_FALSE(fragile_string_dispatch(future_edit)) << "string dispatch silently breaks when message text changes; "
                                                          "typed code() does not. THIS is what the W4 surface unlocks.";
}

// ── §wire-W5: SCXML semantic-validation typed family ───────────────
//
// Wire-code conformance and ID-stability tests mirror the W4
// `ParseErrorWire` block. Stage assertion is inlined ("validation"
// instead of "xml") because the shared `assertSchemaConformantBase`
// helper hardcodes stage="xml" for the W3/W4 parser-entry family.
//
// Three of the four subtypes REUSE existing `validation/*` wire
// codes per the W4 D4 fold precedent (concept identity); only
// `SemanticTopLevelScriptUnloaded` introduces a NEW wire code with
// a `spec` field anchor (W3C SCXML §5.8). Each leaf test exercises
// both the schema envelope and the leaf-specific payload extras
// (`actual`, `fix.candidates`, `spec`) where they apply.

namespace semantic_conformance {

// W5 variant of the schema-conformance helper. `stage` is fixed to
// "validation" because all four SemanticError leaves share that
// stage (§wire-W5 D2).
void assertSemanticBase(const nlohmann::ordered_json &j, std::string_view expectedCode) {
    ASSERT_TRUE(j.contains("v")) << j.dump();
    ASSERT_TRUE(j.contains("id")) << j.dump();
    ASSERT_TRUE(j.contains("code")) << j.dump();
    ASSERT_TRUE(j.contains("stage")) << j.dump();
    ASSERT_TRUE(j.contains("message")) << j.dump();

    EXPECT_EQ(j.at("v").get<int>(), 1);
    EXPECT_EQ(j.at("code").get<std::string>(), std::string{expectedCode});
    EXPECT_EQ(j.at("stage").get<std::string>(), "validation");
    EXPECT_FALSE(j.at("message").get<std::string>().empty());
    // Reuse the W4 helper's id regex via the existing matchesIdRegex
    // visible from the conformance:: namespace.
    EXPECT_TRUE(conformance::matchesIdRegex(j.at("id").get<std::string>()))
        << "id '" << j.at("id").get<std::string>() << "' does not match ^fnv1a:[0-9a-f]{16}$";
}

}  // namespace semantic_conformance

TEST(SemanticErrorWire, InitialStateUnknownConformsToV1Schema) {
    // Folded onto `validation/invalid-reference` per §wire-W5 D2 — the
    // same wire code forge `ValidationError::InvalidReference` emits.
    // Test pins the fold: a consumer dispatching on
    // `validation/invalid-reference` MUST see this leaf's payload
    // identical to the forge counterpart's payload shape.
    const SemanticInitialStateUnknown err("Initial state 'nope' not found",
                                          /*state_id=*/"nope", SemanticInitialStateUnknown::Scope::DocumentRoot,
                                          /*parent_id=*/"",
                                          /*available=*/{"s1", "s2"});
    const auto j = err.to_json();
    semantic_conformance::assertSemanticBase(j, "validation/invalid-reference");
    conformance::assertNoUnexpectedKeys(j);
    EXPECT_EQ(j.at("actual").get<std::string>(), "nope");
    ASSERT_TRUE(j.contains("fix"));
    EXPECT_EQ(j.at("fix").at("kind").get<std::string>(), "replace_one_of");
    const auto candidates = j.at("fix").at("candidates");
    ASSERT_TRUE(candidates.is_array());
    EXPECT_EQ(candidates.size(), 2u);
    EXPECT_EQ(candidates[0].get<std::string>(), "s1");
    EXPECT_EQ(candidates[1].get<std::string>(), "s2");
}

TEST(SemanticErrorWire, InitialStateUnknownEmptyAvailableSuppressesFix) {
    // When the model has zero declared states, the candidate list is
    // empty and `fix` MUST be omitted (no point suggesting a
    // `replace_one_of` with no choices). Mirrors Rust
    // `scxml_semantic_fields` behaviour: `if available.is_empty()
    // { None } else { Some(Fix::ReplaceOneOf {...}) }`.
    const SemanticInitialStateUnknown err("Initial state 'nope' not found",
                                          /*state_id=*/"nope", SemanticInitialStateUnknown::Scope::DocumentRoot,
                                          /*parent_id=*/"",
                                          /*available=*/{});
    const auto j = err.to_json();
    EXPECT_FALSE(j.contains("fix"));
    EXPECT_EQ(j.at("actual").get<std::string>(), "nope");
}

TEST(SemanticErrorWire, TransitionTargetUnknownConformsToV1Schema) {
    const SemanticTransitionTargetUnknown err(
        "Transition in state 'active' references non-existent target state 'ghost'",
        /*state=*/"active",
        /*target=*/"ghost",
        /*available=*/{"idle", "active"});
    const auto j = err.to_json();
    semantic_conformance::assertSemanticBase(j, "validation/invalid-reference");
    conformance::assertNoUnexpectedKeys(j);
    EXPECT_EQ(j.at("actual").get<std::string>(), "ghost");
    ASSERT_TRUE(j.contains("fix"));
    EXPECT_EQ(j.at("fix").at("candidates").size(), 2u);
}

TEST(SemanticErrorWire, NoStatesConformsToV1Schema) {
    const SemanticNoStates err("No state nodes found in SCXML document");
    const auto j = err.to_json();
    semantic_conformance::assertSemanticBase(j, "validation/empty-collection");
    conformance::assertNoUnexpectedKeys(j);
    // No payload extras — Rust counterpart emits no `actual` or `fix`
    // for `validation/empty-collection` either.
    EXPECT_FALSE(j.contains("actual"));
    EXPECT_FALSE(j.contains("fix"));
}

TEST(SemanticErrorWire, TopLevelScriptUnloadedConformsToV1Schema) {
    // The 1 NEW wire code §wire-W5 D2 introduces. Carries `spec` field
    // ("W3C SCXML §5.8") because the code has a spec_anchor on the
    // Rust side; key ordering is (v, id, code, stage, spec, message,
    // location?, actual?) per the schema's canonical order.
    const SemanticTopLevelScriptUnloaded err("Top-level <script> rejected per W3C SCXML 5.8",
                                             /*index=*/std::optional<std::size_t>{2},
                                             /*src=*/std::optional<std::string>{"init.js"});
    const auto j = err.to_json();
    semantic_conformance::assertSemanticBase(j, "scxml/top-level-script-unloaded");
    conformance::assertNoUnexpectedKeys(j);
    ASSERT_TRUE(j.contains("spec"));
    EXPECT_EQ(j.at("spec").get<std::string>(), "W3C SCXML §5.8");
    ASSERT_TRUE(j.contains("actual"));
    EXPECT_EQ(j.at("actual").get<std::string>(), "init.js");
}

TEST(SemanticErrorWire, TopLevelScriptUnloadedAnalyzerPathOmitsActual) {
    // Analyzer-path producer (Rust `analyzer::can_generate_static`)
    // emits with both `index` and `src` as None — wire output omits
    // `actual` but keeps `spec` and the wire `code`. Pins payload
    // asymmetry symmetric across producers (§wire-W5 anti-pattern #5:
    // "NEW wire code count > NEW Rust producer count" — both sides
    // emit the same code; payload detail varies).
    const SemanticTopLevelScriptUnloaded err("Top-level <script> rejected per W3C SCXML 5.8",
                                             /*index=*/std::nullopt,
                                             /*src=*/std::nullopt);
    const auto j = err.to_json();
    semantic_conformance::assertSemanticBase(j, "scxml/top-level-script-unloaded");
    EXPECT_FALSE(j.contains("actual"));
    EXPECT_TRUE(j.contains("spec"));
}

TEST(SemanticErrorWire, IdDistinguishesScopeOnInitialStateUnknown) {
    // Two `SemanticInitialStateUnknown` instances with the SAME
    // unresolved id but DIFFERENT scope (root vs compound) must yield
    // distinct content-hash ids — otherwise a document with the bug
    // at both the root and a compound state would look like one
    // failure to consumers. The id derives from the message text
    // (which differs across scopes when the throw-site message
    // includes the parent state's id), which is the W4 D4 idiom
    // applied here.
    const SemanticInitialStateUnknown root("Initial state 'X' not found", "X",
                                           SemanticInitialStateUnknown::Scope::DocumentRoot, "", {});
    const SemanticInitialStateUnknown compound("State 'parent' references non-existent initial state 'X'", "X",
                                               SemanticInitialStateUnknown::Scope::CompoundState, "parent", {});
    EXPECT_NE(root.to_json().at("id").get<std::string>(), compound.to_json().at("id").get<std::string>())
        << "root vs compound scope must yield distinct ids";
}

TEST(SemanticErrorWire, IdDiffersAcrossDifferentWireCodes) {
    // Cross-leaf id distinctness when wire codes differ — same as the
    // `ParseErrorWire::IdDiffersAcrossSubtypesWithSameMessage` W4
    // precedent. Leaves that share `validation/invalid-reference`
    // (InitialStateUnknown vs TransitionTargetUnknown) intentionally
    // collide on id when message+payload match — the documented
    // α-strict tradeoff for the fold.
    const std::string msg = "shared message";
    const SemanticInitialStateUnknown a(msg, "x", SemanticInitialStateUnknown::Scope::DocumentRoot, "", {});
    const SemanticNoStates b(msg);
    EXPECT_NE(a.to_json().at("id").get<std::string>(), b.to_json().at("id").get<std::string>())
        << "different wire codes must yield distinct ids even with shared message";
}

// ── Test-as-consumer fragility tests (§wire-W5 D6) ─────────────────
//
// Mirror W4's `ParseErrorConsumer.TypedCodeDistinguishesFailureClass*`
// tests: a hypothetical consumer dispatching on `code()` MUST be able
// to distinguish W5's failure class from any other semantic class
// using only the wire code. For the 3 fold-reused codes, the test
// also pins that the fold honest at the consumer level — a consumer
// branching on `validation/invalid-reference` receives both forge
// ValidationError::InvalidReference AND SCXML
// SemanticInitialStateUnknown / SemanticTransitionTargetUnknown
// uniformly.

TEST(SemanticErrorConsumer, TypedCodeDistinguishesFailureClassInitialStateUnknown) {
    const SemanticInitialStateUnknown err("Initial state 'nope' not found", "nope",
                                          SemanticInitialStateUnknown::Scope::DocumentRoot, "", {"s1"});
    auto dispatch = [](const SemanticError &e) { return e.code() == std::string_view{"validation/invalid-reference"}; };
    EXPECT_TRUE(dispatch(err));

    // Round-trip via the SemanticError base reference confirms the
    // dynamic type's `code()` survives polymorphic erasure (a sliced
    // base copy would dispatch to the pure-virtual override).
    const SemanticError &base = err;
    EXPECT_EQ(base.code(), std::string_view{"validation/invalid-reference"});
}

TEST(SemanticErrorConsumer, TypedCodeDistinguishesFailureClassTransitionTargetUnknown) {
    const SemanticTransitionTargetUnknown err("Transition in state 'a' references non-existent target state 'b'", "a",
                                              "b", {"a", "c"});
    auto dispatch = [](const SemanticError &e) { return e.code() == std::string_view{"validation/invalid-reference"}; };
    EXPECT_TRUE(dispatch(err));
}

TEST(SemanticErrorConsumer, TypedCodeDistinguishesFailureClassHistoryDefaultMissing) {
    const SemanticHistoryDefaultMissing err("History state 'h' in state 'p' declares no default <transition>", "h", "p",
                                            {"a", "b"});
    auto dispatch = [](const SemanticError &e) { return e.code() == std::string_view{"validation/missing-element"}; };
    EXPECT_TRUE(dispatch(err));

    // §scxml-3.10.2 restricts the default configuration to the
    // containing state's descendants, so the payload carries that set
    // rather than every declared id.
    EXPECT_EQ(err.available(), (std::vector<std::string>{"a", "b"}));
    EXPECT_EQ(err.parent_id(), "p");

    // `validation/missing-element` carries `actual` and no `fix` —
    // SCE_ERROR_CONTRACT §3.1 has no add-child-element variant, and the
    // Rust producer makes the same choice. A `fix` on one side only
    // would make the two producers disagree on a shared wire code.
    const auto json = err.to_json();
    EXPECT_EQ(json.at("actual"), "h");
    EXPECT_FALSE(json.contains("fix"));
}

TEST(SemanticErrorConsumer, TypedCodeDistinguishesFailureClassNoStates) {
    const SemanticNoStates err("No state nodes found in SCXML document");
    auto dispatch = [](const SemanticError &e) { return e.code() == std::string_view{"validation/empty-collection"}; };
    EXPECT_TRUE(dispatch(err));
}

TEST(SemanticErrorConsumer, TypedCodeDistinguishesFailureClassTopLevelScriptUnloaded) {
    const SemanticTopLevelScriptUnloaded err("Top-level <script> rejected per W3C SCXML 5.8", std::nullopt,
                                             std::nullopt);
    auto dispatch = [](const SemanticError &e) {
        return e.code() == std::string_view{"scxml/top-level-script-unloaded"};
    };
    EXPECT_TRUE(dispatch(err));
}

TEST(SemanticErrorConsumer, FoldHonestAtConsumerLevel) {
    // A consumer branching on `validation/invalid-reference` MUST
    // receive both the forge surface (where ValidationError emits
    // the same code) AND the SCXML surface uniformly. This pins the
    // W4 D4 fold success criterion at the consumer level, applied
    // symmetrically to W5: same wire code = same consumer branch =
    // same repair logic, regardless of producing document type.
    //
    // For W5 the test focuses on the SCXML half: both
    // SemanticInitialStateUnknown and SemanticTransitionTargetUnknown
    // emit `validation/invalid-reference` — a single consumer branch
    // handles both. (The forge half is pinned by Rust's
    // `fold_invariant_holds_for_invalid_reference` test in
    // `sce-build/src/scxml_semantic.rs`.)
    const SemanticInitialStateUnknown a("msg", "id", SemanticInitialStateUnknown::Scope::DocumentRoot, "", {});
    const SemanticTransitionTargetUnknown b("msg", "s", "t", {});
    EXPECT_EQ(a.code(), b.code()) << "fold reuses `validation/invalid-reference` for both leaves";
    EXPECT_EQ(a.code(), std::string_view{"validation/invalid-reference"});
}

TEST(SemanticErrorConsumer, TypedCodeStableUnderMessageTextEdit) {
    // Mirrors W4's `ParseErrorConsumer.TypedCodeStableUnderMessageTextEdit`.
    // Two leaves with intentionally divergent message text must
    // still share `code()` — agents dispatching on the wire code do
    // not break when the message text changes (e.g. localization,
    // refinement of the human-readable wording).
    const SemanticInitialStateUnknown today("Initial state 'X' not found", "X",
                                            SemanticInitialStateUnknown::Scope::DocumentRoot, "", {});
    const SemanticInitialStateUnknown future_edit(
        "scxml semantic check: cannot resolve initial=\"X\" to a declared state", "X",
        SemanticInitialStateUnknown::Scope::DocumentRoot, "", {});
    EXPECT_EQ(today.code(), future_edit.code());
    EXPECT_NE(std::string(today.what()), std::string(future_edit.what()));
}

// ── SCXMLParser boundary surfacing for SemanticError leaves (W5) ──
//
// Mirror of W4's `SCXMLParserBoundary.ParseFileSurfacesTypedFileNotFoundDiagnostic`
// pattern: feed a doc that trips each W5 throw site; verify
// (a) `parseContent` returns nullptr,
// (b) Q4-B legacy `getErrorMessages()` is populated,
// (c) typed `getDiagnostics()` carries the expected wire `code()`.
//
// Standing-consumer load-bearing-ness: the catch-arm pair
// (`addError` + `recordDiagnostic`) added in `SCXMLParser.cpp`'s
// `catch (const SCE::parsing::SemanticError &se)` is the sole site
// that bridges the typed throw to both surfaces. Removing
// `recordDiagnostic(se.clone())` reds the
// `getDiagnostics().size() == 1u` assertion below; removing
// `addError(se.what())` reds Q4-B coexistence.

TEST(SCXMLParserBoundary, ParseContentSurfacesTypedInitialStateUnknownDiagnostic) {
    constexpr const char *kBrokenScxml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
                                         "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\""
                                         "       version=\"1.0\""
                                         "       initial=\"nope\""
                                         "       name=\"boundary_w5_initial\">"
                                         "  <state id=\"s1\"/>"
                                         "</scxml>";

    SCE::SCXMLParser parser(std::make_shared<SCE::NodeFactory>());
    auto model = parser.parseContent(kBrokenScxml);

    EXPECT_EQ(model, nullptr);
    EXPECT_TRUE(parser.hasErrors());
    EXPECT_FALSE(parser.getErrorMessages().empty());

    const auto &diags = parser.getDiagnostics();
    ASSERT_EQ(diags.size(), 1u);
    ASSERT_NE(diags[0], nullptr);
    EXPECT_EQ(diags[0]->code(), std::string_view{"validation/invalid-reference"});

    const auto j = diags[0]->to_json();
    EXPECT_EQ(j.at("code").get<std::string>(), "validation/invalid-reference");
    EXPECT_EQ(j.at("stage").get<std::string>(), "validation");
    EXPECT_EQ(j.at("actual").get<std::string>(), "nope");
}

TEST(SCXMLParserBoundary, ParseContentSurfacesTypedTransitionTargetUnknownDiagnostic) {
    constexpr const char *kBrokenScxml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
                                         "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\""
                                         "       version=\"1.0\""
                                         "       initial=\"s1\""
                                         "       name=\"boundary_w5_target\">"
                                         "  <state id=\"s1\">"
                                         "    <transition event=\"go\" target=\"ghost\"/>"
                                         "  </state>"
                                         "</scxml>";

    SCE::SCXMLParser parser(std::make_shared<SCE::NodeFactory>());
    auto model = parser.parseContent(kBrokenScxml);

    EXPECT_EQ(model, nullptr);
    EXPECT_TRUE(parser.hasErrors());

    const auto &diags = parser.getDiagnostics();
    ASSERT_EQ(diags.size(), 1u);
    EXPECT_EQ(diags[0]->code(), std::string_view{"validation/invalid-reference"});
    EXPECT_EQ(diags[0]->to_json().at("actual").get<std::string>(), "ghost");
}

TEST(SCXMLParserBoundary, ParseContentSurfacesTypedCompoundInitialUnknownDiagnostic) {
    // §wire-W5 D2 fold: compound-state initial uses the SAME wire code
    // (`validation/invalid-reference`) as document-root initial. The
    // typed leaf carries `Scope::CompoundState` for in-process typed
    // dispatch consumers; wire consumers see one branch.
    constexpr const char *kBrokenScxml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
                                         "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\""
                                         "       version=\"1.0\""
                                         "       initial=\"outer\""
                                         "       name=\"boundary_w5_compound\">"
                                         "  <state id=\"outer\" initial=\"missing\">"
                                         "    <state id=\"inner\"/>"
                                         "  </state>"
                                         "</scxml>";

    SCE::SCXMLParser parser(std::make_shared<SCE::NodeFactory>());
    auto model = parser.parseContent(kBrokenScxml);

    EXPECT_EQ(model, nullptr);
    const auto &diags = parser.getDiagnostics();
    ASSERT_EQ(diags.size(), 1u);
    EXPECT_EQ(diags[0]->code(), std::string_view{"validation/invalid-reference"});
    EXPECT_EQ(diags[0]->to_json().at("actual").get<std::string>(), "missing");

    // In-process typed dispatch — confirm scope is CompoundState.
    const auto *typed = dynamic_cast<const SemanticInitialStateUnknown *>(diags[0].get());
    ASSERT_NE(typed, nullptr) << "diagnostic must preserve dynamic type";
    EXPECT_EQ(typed->scope(), SemanticInitialStateUnknown::Scope::CompoundState);
    EXPECT_EQ(typed->parentId(), "outer");
}

TEST(SCXMLParserBoundary, ParseContentSurfacesTypedNoStatesDiagnostic) {
    // Doc with no top-level state/parallel/final children. W3C SCXML
    // §3.2 requires at least one — folded onto
    // `validation/empty-collection` per §wire-W5 D2.
    constexpr const char *kBrokenScxml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
                                         "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\""
                                         "       version=\"1.0\""
                                         "       name=\"boundary_w5_no_states\">"
                                         "</scxml>";

    SCE::SCXMLParser parser(std::make_shared<SCE::NodeFactory>());
    auto model = parser.parseContent(kBrokenScxml);

    EXPECT_EQ(model, nullptr);
    const auto &diags = parser.getDiagnostics();
    ASSERT_EQ(diags.size(), 1u);
    EXPECT_EQ(diags[0]->code(), std::string_view{"validation/empty-collection"});
}

TEST(SCXMLParserBoundary, ParseContentSurfacesTypedTopLevelScriptUnloadedDiagnostic) {
    // Top-level `<script src="..."/>` referencing a non-existent file
    // — `FileLoadingHelper::loadExternalScript` fails, `ActionParser`
    // returns nullptr, `parseScxmlNode` throws
    // `SemanticTopLevelScriptUnloaded`. This is the C++-side trigger
    // for the 1 NEW wire code §wire-W5 introduces.
    //
    // Note: empty `<script/>` does NOT trigger the throw on C++ —
    // `ActionParser` returns a ScriptAction with empty content (the
    // C++ side doesn't enforce W3C §5.8's "empty script rejected"
    // rule at parse time; that path is Rust-only via
    // `analyzer::can_generate_static`'s `document_rejected` branch).
    // C++ behaviour mirrors the typed surface for the file-load
    // failure case which IS pinned here.
    constexpr const char *kBrokenScxml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
                                         "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\""
                                         "       version=\"1.0\""
                                         "       initial=\"s1\""
                                         "       datamodel=\"ecmascript\""
                                         "       name=\"boundary_w5_script\">"
                                         "  <script src=\"/this/path/should/not/exist/missing.js\"/>"
                                         "  <state id=\"s1\"/>"
                                         "</scxml>";

    SCE::SCXMLParser parser(std::make_shared<SCE::NodeFactory>());
    auto model = parser.parseContent(kBrokenScxml);

    EXPECT_EQ(model, nullptr);
    const auto &diags = parser.getDiagnostics();
    ASSERT_EQ(diags.size(), 1u);
    EXPECT_EQ(diags[0]->code(), std::string_view{"scxml/top-level-script-unloaded"});

    const auto j = diags[0]->to_json();
    EXPECT_EQ(j.at("spec").get<std::string>(), "W3C SCXML §5.8");
    EXPECT_EQ(j.at("stage").get<std::string>(), "validation");

    // In-process typed dispatch — confirm payload preserved through
    // catch-arm `clone()`. Index is 1-based per W3C SCXML §5.8
    // ordinal.
    const auto *typed = dynamic_cast<const SemanticTopLevelScriptUnloaded *>(diags[0].get());
    ASSERT_NE(typed, nullptr) << "diagnostic must preserve dynamic type";
    ASSERT_TRUE(typed->index().has_value());
    EXPECT_EQ(*typed->index(), 1u);
    ASSERT_TRUE(typed->src().has_value());
    EXPECT_EQ(*typed->src(), "/this/path/should/not/exist/missing.js");
}

// ── generator stamp resolution ──────────────────────────────────────

// A build made from this checkout must report a commit, not `unknown`.
//
// `unknown` is a legal schema value and the honest answer for a build
// with no git checkout to read, so every schema-shaped assertion in this
// file accepts it. That acceptance is exactly what makes the value
// unable to police itself: when `SCE_GIT_COMMIT` was first attached to
// `sce_base` — which does not compile `Diagnostic.cpp` — the macro was
// simply undefined here, `beginDiagnosticRecord` took its fallback
// branch, and all 65 tests passed while every record on the wire
// claimed an unidentifiable producer.
//
// This test is the one place that knows something the schema cannot:
// this suite runs inside the repository, so a checkout exists, so
// `unknown` means the resolution broke rather than that there was
// nothing to resolve.
TEST(DiagnosticWire, GeneratorStampNamesThisCheckout) {
    const TemplateNotFound err("probe");
    const auto j = err.to_json();
    ASSERT_TRUE(j.contains("generator")) << j.dump();
    const auto generator = j.at("generator").get<std::string>();
    EXPECT_NE(generator, "unknown") << "this suite builds from a git checkout, so `unknown` means the "
                                       "SCE_GIT_COMMIT definition is not reaching the target that "
                                       "compiles src/parsing/Diagnostic.cpp — see sce/CMakeLists.txt";
    EXPECT_TRUE(std::regex_match(generator, std::regex{R"(^[0-9a-f]{7,40}$)"}))
        << "generator '" << generator << "' is neither a hex commit nor `unknown`";
}

}  // namespace
}  // namespace SCE::parsing
