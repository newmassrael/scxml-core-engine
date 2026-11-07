// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The verdicts of the test profile (docs/SCE_TEST_PROFILE.md §5): the four a
//! document sets with `<sce:setverdict>` (P12) and `error`, which only the
//! runtime sets (P13), in the order ETSI ES 201 873-1 §24.1 gives them.

/// A test verdict, declared in the order `none < pass < inconclusive < fail < error`.
///
/// The order is the one ES 201 873-1 §24.1 overwrites by: a verdict only ever
/// becomes worse, so the derived order of the enumeration is the order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Verdict {
    /// `none`: no verdict was set.
    #[default]
    None,
    /// `pass`.
    Pass,
    /// `inconc`.
    Inconclusive,
    /// `fail`.
    Fail,
    /// `error`: set only by the runtime (P13).
    Error,
}

impl Verdict {
    /// The verdict's spelling in the test log and in TTCN-3.
    pub const fn name(self) -> &'static str {
        match self {
            Verdict::None => "none",
            Verdict::Pass => "pass",
            Verdict::Inconclusive => "inconc",
            Verdict::Fail => "fail",
            Verdict::Error => "error",
        }
    }
}

/// The worse of `a` and `b`: what `setverdict` leaves a local verdict at
/// (P12, -4 §9.45), and how the test case verdict folds the local ones (P14).
pub fn worse(a: Verdict, b: Verdict) -> Verdict {
    a.max(b)
}
