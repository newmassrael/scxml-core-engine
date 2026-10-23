// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0055 under generated C++: the helpers a machine reads a value that may be absent
// through. The machines that hold such values are replayed by their scenarios
// (static_optional, static_optional_widened, static_state_algorithm_optional); this file
// holds the helpers themselves, where evaluating a default or a bound too eagerly would
// leave no trace in a machine's fields.

#include <cstdint>
#include <optional>
#include <string>

#include <gtest/gtest.h>

#include <sce/forge/optional.h>

#include "static_block_ends_sm.h"

namespace SCE::Tests {
namespace {

// `x ?? d` evaluates `d` only when `x` is absent, so a default that would overflow records
// the failure only then, and a default that may be absent keeps the value optional; an
// optional text is held to its bound only when it is present; and a `<log>` shows an
// absent value as `null` and a present one as the other engines show it.
TEST(AStaticOptionalValueTest, AnOptionalValueIsReadWithADefaultOnlyWhenAbsent) {
    int asked = 0;
    const auto five = [&asked] {
        ++asked;
        return std::int64_t{5};
    };
    EXPECT_EQ(SCE::Forge::otherwise(std::optional<std::int64_t>(12), five), 12);
    EXPECT_EQ(asked, 0) << "a present value does not evaluate its default";
    EXPECT_EQ(SCE::Forge::otherwise(std::optional<std::int64_t>(), five), 5);
    EXPECT_EQ(asked, 1);

    SCE::Forge::AlgorithmFailure failure;
    const std::int64_t wide = 4294967296;
    const auto overflowing = [&] { return SCE::Forge::Checked::mul<std::int64_t>(failure, wide, wide); };
    EXPECT_EQ(SCE::Forge::otherwise(std::optional<std::int64_t>(7), overflowing), 7);
    EXPECT_FALSE(failure.failed()) << "the default that overflows is not evaluated beside a present value";
    SCE::Forge::otherwise(std::optional<std::int64_t>(), overflowing);
    EXPECT_TRUE(failure.failed()) << "the default is evaluated for an absent value, and its overflow recorded";

    EXPECT_EQ(SCE::Forge::otherwise(std::optional<std::string>(), [] { return "none"; }), "none");
    EXPECT_FALSE(SCE::Forge::otherwise(std::optional<std::string>(), [] { return std::nullopt; }).has_value());
    EXPECT_EQ(SCE::Forge::otherwise(std::optional<std::string>("soon"), [] { return std::optional<std::string>(); }),
              std::optional<std::string>("soon"));

    SCE::Forge::AlgorithmFailure bound;
    const auto held = [&bound](const std::string &text) { return SCE::Forge::Checked::bounded(bound, text, 4u); };
    EXPECT_FALSE(SCE::Forge::mapPresent(std::optional<std::string>(), held).has_value());
    EXPECT_EQ(SCE::Forge::mapPresent(std::optional<std::string>("soon"), held), std::optional<std::string>("soon"));
    EXPECT_FALSE(bound.failed()) << "an absent text and one within the bound fit it";
    SCE::Forge::mapPresent(std::optional<std::string>("later"), held);
    EXPECT_TRUE(bound.failed()) << "a present text past the bound is a capacity failure";

    namespace E = ::SCE::Generated::static_block_ends;
    EXPECT_EQ(E::sceLogName(std::optional<std::int64_t>()), "null");
    EXPECT_EQ(E::sceLogName(std::optional<std::int64_t>(-12)), "-12");
    EXPECT_EQ(E::sceLogName(std::optional<bool>(true)), "true");
    EXPECT_EQ(E::sceLogName(std::optional<double>(1.5)), "1.5");
    EXPECT_EQ(E::sceLogName(std::optional<std::string>("soon")), "soon");
}

}  // namespace
}  // namespace SCE::Tests
