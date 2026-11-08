// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The verdicts of the test profile (docs/SCE_TEST_PROFILE.md §5): the four a document sets with
// `<sce:setverdict>` (P12) and `error`, which only the runtime sets (P13), in the order ETSI ES 201
// 873-1 §24.1 gives them.
//
// Cross-language siblings: `sce_rust_runtime::test::Verdict` (Rust), `SCE::Test::Verdict` (C++),
// `com.sce.runtime.test.Verdict` (Kotlin), `testprofile.Verdict` (Go),
// `sce_runtime.testprofile.Verdict` (Python), `sce_test_verdict_t` (C11).

/// A test verdict, declared in the order `none < pass < inconclusive < fail < error`.
///
/// The order is the one ES 201 873-1 §24.1 overwrites by: a verdict only ever becomes worse, so the
/// order of the cases is the order. `spelling` is the verdict's spelling in the test log and in
/// TTCN-3 (`inconc` for `inconclusive`), which a case's own name cannot be.
public enum Verdict: Int, Comparable, CaseIterable, Sendable {
    /// `none`: no verdict was set.
    case none
    /// `pass`.
    case pass
    /// `inconc`.
    case inconclusive
    /// `fail`.
    case fail
    /// `error`: set only by the runtime (P13).
    case error

    /// The verdict as the test log and TTCN-3 spell it.
    public var spelling: String {
        switch self {
        case .none: return "none"
        case .pass: return "pass"
        case .inconclusive: return "inconc"
        case .fail: return "fail"
        case .error: return "error"
        }
    }

    public static func < (lhs: Verdict, rhs: Verdict) -> Bool {
        lhs.rawValue < rhs.rawValue
    }
}

/// The worse of `a` and `b`: what `setverdict` leaves a local verdict at (P12, -4 §9.45), and how the
/// test case verdict folds the local ones (P14).
public func worse(_ a: Verdict, _ b: Verdict) -> Verdict {
    max(a, b)
}
