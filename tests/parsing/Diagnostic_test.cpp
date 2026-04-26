// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "parsing/Diagnostic.h"

#include <gtest/gtest.h>

#include <optional>
#include <string_view>

// Standing consumer for the abstract `SCE::parsing::Diagnostic` base
// while the concrete `TemplateError` refit lands in the next commit
// (RFC §W1 commit-series). Keeps the interface load-bearing per
// `feedback_built_but_unconsumed.md`: removing `= 0` from any of the
// three pure virtuals makes this fixture's `FakeDiagnostic` non-abstract
// in a way the linker rejects, so the contract bites if we relax it.

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

}  // namespace
}  // namespace SCE::parsing
