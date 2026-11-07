// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The verdicts of the test profile (docs/SCE_TEST_PROFILE.md §5): the four a
// document sets with `<sce:setverdict>` (P12) and `error`, which only the
// runtime sets (P13), in the order ETSI ES 201 873-1 §24.1 gives them.

#pragma once

#include <cstdint>

namespace SCE::Test {

/**
 * @brief A test verdict, declared in the order `none < pass < inconclusive < fail < error`
 *
 * The order is the one ES 201 873-1 §24.1 overwrites by: a verdict only ever
 * becomes worse, so the comparison operators of the enumeration are the order.
 */
enum class Verdict : std::uint8_t { None, Pass, Inconclusive, Fail, Error };

/// The worse of @p a and @p b: what `setverdict` leaves a local verdict at
/// (P12, -4 §9.45), and how the test case verdict folds the local ones (P14).
constexpr Verdict worse(Verdict a, Verdict b) {
    return a < b ? b : a;
}

/// The verdict's spelling in the test log and in TTCN-3.
constexpr const char *verdictName(Verdict verdict) {
    switch (verdict) {
    case Verdict::None:
        return "none";
    case Verdict::Pass:
        return "pass";
    case Verdict::Inconclusive:
        return "inconc";
    case Verdict::Fail:
        return "fail";
    case Verdict::Error:
        return "error";
    }
    return "error";
}

}  // namespace SCE::Test
