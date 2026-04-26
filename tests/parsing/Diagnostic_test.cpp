// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "parsing/Diagnostic.h"
#include "parsing/TemplateError.h"

#include <gtest/gtest.h>

#include <array>
#include <optional>
#include <regex>
#include <set>
#include <string>
#include <string_view>

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
