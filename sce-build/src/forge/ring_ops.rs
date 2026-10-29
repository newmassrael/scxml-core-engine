// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The ring operations of the integers, `+ − ×` and negation, and the rules
//! that bring their exact result into a fixed width (docs/adr/0063).
//!
//! ℤ is closed under these four, so each has an exact result `e` on every
//! input. An integer type `T` of width `W` holds `[m, M]`, and an operation
//! of `T` is `e` reduced into that range by one of three rules:
//!
//! - **checked** (`+ - *` and unary `-` as written, SCE_FORGE.md §3.4.1):
//!   `e` where `m ≤ e ≤ M`, and no value — the failure `overflow` —
//!   otherwise;
//! - **wrapping**: the `r ∈ [m, M]` with `r ≡ e (mod 2^W)`, the quotient map
//!   onto ℤ/2^Wℤ with the representative `T` holds (two's complement for a
//!   signed `T`);
//! - **saturating**: `max(m, min(M, e))`.
//!
//! The last two are total, and an author names them in an expression with
//! the eight names of [`VOCABULARY`]. This module is the one definition the
//! parser, the build-time fold ([`crate::forge::const_fold`]) and the
//! interval analysis ([`crate::forge::int_ranges`]) read; each backend's
//! runtime implements the same definition for its own types.
//!
//! Every value here is an `i128`. The bounds of every type SCE declares and
//! the exact sum or difference of two of its values fit; the exact product of
//! two `uint64` values does not, and is computed as an unsigned 128-bit one.

/// A ring operation of ℤ: the four operations whose exact result always
/// exists. Division is not one — `a / 0` has none — so it has no wrapping or
/// saturating form (docs/adr/0063, section 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RingOp {
    Add,
    Sub,
    Mul,
    Neg,
}

impl RingOp {
    /// How many operands the operation takes.
    pub(crate) fn arity(self) -> usize {
        match self {
            Self::Neg => 1,
            Self::Add | Self::Sub | Self::Mul => 2,
        }
    }

    /// The operation's name in every runtime's helper families (`add`,
    /// `sub`, `mul`, `neg`), which the checked family shares.
    pub(crate) fn helper(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Sub => "sub",
            Self::Mul => "mul",
            Self::Neg => "neg",
        }
    }
}

/// How an operation's exact result is brought into its type when the author
/// names the rule: the two total reductions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverflowRule {
    Wrapping,
    Saturating,
}

impl OverflowRule {
    /// The rule's name in every runtime's helper families
    /// (`wrapping_sub`, `WrappingSubUint32`, `SceWrapping.sub`, …).
    pub(crate) fn helper(self) -> &'static str {
        match self {
            Self::Wrapping => "wrapping",
            Self::Saturating => "saturating",
        }
    }
}

/// The eight names of the expression vocabulary, each one operation under one
/// rule. A saturating negation of an unsigned value is 0 for every input, so
/// `saturatingNeg` is refused for an unsigned type where it is written
/// ([`admits`]) rather than missing from the table.
pub(crate) const VOCABULARY: [(&str, RingOp, OverflowRule); 8] = [
    ("wrappingAdd", RingOp::Add, OverflowRule::Wrapping),
    ("wrappingSub", RingOp::Sub, OverflowRule::Wrapping),
    ("wrappingMul", RingOp::Mul, OverflowRule::Wrapping),
    ("wrappingNeg", RingOp::Neg, OverflowRule::Wrapping),
    ("saturatingAdd", RingOp::Add, OverflowRule::Saturating),
    ("saturatingSub", RingOp::Sub, OverflowRule::Saturating),
    ("saturatingMul", RingOp::Mul, OverflowRule::Saturating),
    ("saturatingNeg", RingOp::Neg, OverflowRule::Saturating),
];

/// The operation and rule a vocabulary name stands for.
pub(crate) fn lookup(name: &str) -> Option<(RingOp, OverflowRule)> {
    VOCABULARY
        .iter()
        .find(|(spelled, ..)| *spelled == name)
        .map(|&(_, op, rule)| (op, rule))
}

/// The vocabulary name of `op` under `rule`, as the author writes it and a
/// refusal quotes it.
pub(crate) fn name(op: RingOp, rule: OverflowRule) -> &'static str {
    VOCABULARY
        .iter()
        .find(|&&(_, o, r)| o == op && r == rule)
        .map(|&(spelled, ..)| spelled)
        .expect("every operation has a name under every rule")
}

/// Whether `op` under `rule` is an operation of a type that holds `[lo, hi]`:
/// every one is but the saturating negation of an unsigned type, whose value
/// does not depend on its operand.
pub(crate) fn admits(op: RingOp, rule: OverflowRule, lo: i128) -> bool {
    !(op == RingOp::Neg && rule == OverflowRule::Saturating && lo >= 0)
}

/// `op` of `a` and `b` (`b` unread for a negation) in the type `[lo, hi]`,
/// under `rule`. `a` and `b` lie in the type.
pub(crate) fn apply(
    op: RingOp,
    rule: OverflowRule,
    (lo, hi): (i128, i128),
    a: i128,
    b: i128,
) -> i128 {
    let exact = match op {
        RingOp::Add => a.checked_add(b),
        RingOp::Sub => a.checked_sub(b),
        RingOp::Mul => a.checked_mul(b),
        RingOp::Neg => a.checked_neg(),
    };
    match exact {
        Some(e) => reduce(rule, (lo, hi), e),
        // Only a product of two `uint64` values leaves `i128`, and both are
        // non-negative: its exact value is the unsigned 128-bit product.
        None => {
            let (a, b) = (unsigned(a), unsigned(b));
            match rule {
                // 2^W divides 2^128, so the product modulo 2^128 is the
                // product modulo 2^W.
                OverflowRule::Wrapping => {
                    let span = unsigned(hi - lo + 1);
                    i128::try_from(a.wrapping_mul(b) % span).expect("a residue fits the type") + lo
                }
                OverflowRule::Saturating => hi,
            }
        }
    }
}

fn unsigned(v: i128) -> u128 {
    u128::try_from(v).expect("only a product of two non-negative uint64 values leaves i128")
}

/// The exact value `e` brought into `[lo, hi]` by `rule`.
pub(crate) fn reduce(rule: OverflowRule, (lo, hi): (i128, i128), e: i128) -> i128 {
    match rule {
        OverflowRule::Wrapping => (e - lo).rem_euclid(hi - lo + 1) + lo,
        OverflowRule::Saturating => e.clamp(lo, hi),
    }
}

/// The exact results of `op` over the operand ranges `l` and `r` (`r` unread
/// for a negation), as an interval. Each operation is monotone in each
/// operand over a box of operands, so its extremes are at the corners. A
/// corner past `i128` is held at `i128`'s bound, which keeps the interval a
/// superset of the exact one: only a product of two `uint64` ranges reaches
/// one.
pub(crate) fn exact_range(op: RingOp, l: (i128, i128), r: (i128, i128)) -> (i128, i128) {
    match op {
        RingOp::Add => (l.0.saturating_add(r.0), l.1.saturating_add(r.1)),
        RingOp::Sub => (l.0.saturating_sub(r.1), l.1.saturating_sub(r.0)),
        RingOp::Neg => (l.1.saturating_neg(), l.0.saturating_neg()),
        RingOp::Mul => {
            let corners = [
                l.0.saturating_mul(r.0),
                l.0.saturating_mul(r.1),
                l.1.saturating_mul(r.0),
                l.1.saturating_mul(r.1),
            ];
            (
                corners.into_iter().min().expect("four corners"),
                corners.into_iter().max().expect("four corners"),
            )
        }
    }
}

/// The values `op` under `rule` takes over the operand ranges `l` and `r`, in
/// the type `[lo, hi]`: the exact image of the interval analysis's transfer
/// (docs/adr/0063, section 5).
///
/// Saturation is monotone, so the clamped ends bound its image exactly. A
/// wrap moves every exact value by a multiple of `2^W`; where the whole exact
/// range lies in one window `[m + k·2^W, M + k·2^W]` every value moves by the
/// same `k·2^W` and the range moves whole, and otherwise the image is the
/// whole type (the wrapped-interval treatment of Navas et al., APLAS 2012,
/// restricted to one interval per value).
pub(crate) fn image(
    op: RingOp,
    rule: OverflowRule,
    (lo, hi): (i128, i128),
    l: (i128, i128),
    r: (i128, i128),
) -> (i128, i128) {
    let (elo, ehi) = exact_range(op, l, r);
    match rule {
        OverflowRule::Saturating => (elo.clamp(lo, hi), ehi.clamp(lo, hi)),
        OverflowRule::Wrapping => {
            // An end held at `i128`'s bound is not the exact one, and its
            // window cannot be read from it.
            if elo == i128::MIN || ehi == i128::MAX {
                return (lo, hi);
            }
            let span = hi - lo + 1;
            let k = (elo - lo).div_euclid(span);
            if (ehi - lo).div_euclid(span) == k {
                (elo - k * span, ehi - k * span)
            } else {
                (lo, hi)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const U8: (i128, i128) = (0, 255);
    const I8: (i128, i128) = (-128, 127);
    const U64: (i128, i128) = (0, u64::MAX as i128);
    const I64: (i128, i128) = (i64::MIN as i128, i64::MAX as i128);

    #[test]
    fn every_name_is_one_operation_under_one_rule() {
        for (spelled, op, rule) in VOCABULARY {
            assert_eq!(lookup(spelled), Some((op, rule)));
            assert_eq!(name(op, rule), spelled);
        }
        assert_eq!(lookup("wrappingDiv"), None);
    }

    #[test]
    fn a_wrapping_operation_is_its_exact_result_modulo_the_width() {
        use OverflowRule::Wrapping as W;
        assert_eq!(apply(RingOp::Sub, W, U8, 0, 1), 255);
        assert_eq!(apply(RingOp::Add, W, U8, 255, 1), 0);
        assert_eq!(apply(RingOp::Mul, W, U8, 16, 16), 0);
        assert_eq!(apply(RingOp::Neg, W, U8, 1, 0), 255);
        assert_eq!(apply(RingOp::Add, W, I8, 127, 1), -128);
        assert_eq!(apply(RingOp::Neg, W, I8, -128, 0), -128);
        assert_eq!(apply(RingOp::Mul, W, I8, -128, -1), -128);
        // The product of two uint64 values leaves i128 and is still reduced
        // exactly: (2^64 - 1)^2 ≡ 1 (mod 2^64).
        let max = u64::MAX as i128;
        assert_eq!(apply(RingOp::Mul, W, U64, max, max), 1);
        assert_eq!(
            apply(RingOp::Mul, W, I64, i64::MIN as i128, -1),
            i64::MIN as i128
        );
    }

    #[test]
    fn a_saturating_operation_is_its_exact_result_held_to_the_bounds() {
        use OverflowRule::Saturating as S;
        assert_eq!(apply(RingOp::Sub, S, U8, 0, 1), 0);
        assert_eq!(apply(RingOp::Add, S, U8, 255, 1), 255);
        assert_eq!(apply(RingOp::Mul, S, I8, -128, -1), 127);
        assert_eq!(apply(RingOp::Neg, S, I8, -128, 0), 127);
        let max = u64::MAX as i128;
        assert_eq!(apply(RingOp::Mul, S, U64, max, 2), max);
        assert_eq!(
            apply(RingOp::Mul, S, I64, i64::MIN as i128, 2),
            i64::MIN as i128
        );
    }

    #[test]
    fn saturation_does_not_associate() {
        use OverflowRule::Saturating as S;
        let inner_first = apply(RingOp::Add, S, I8, apply(RingOp::Add, S, I8, 127, 1), -1);
        let outer_first = apply(RingOp::Add, S, I8, 127, apply(RingOp::Add, S, I8, 1, -1));
        assert_eq!((inner_first, outer_first), (126, 127));
    }

    #[test]
    fn only_the_unsigned_saturating_negation_is_refused() {
        assert!(!admits(RingOp::Neg, OverflowRule::Saturating, 0));
        assert!(admits(RingOp::Neg, OverflowRule::Saturating, -128));
        assert!(admits(RingOp::Neg, OverflowRule::Wrapping, 0));
    }

    #[test]
    fn a_range_that_does_not_cross_a_wrap_point_moves_whole() {
        use OverflowRule::Wrapping as W;
        // Nothing wraps: the exact range.
        assert_eq!(image(RingOp::Add, W, U8, (0, 10), (0, 5)), (0, 15));
        // Every value wraps once: the range moved down by 256.
        assert_eq!(image(RingOp::Add, W, U8, (250, 255), (10, 10)), (4, 9));
        // Below zero, every value moves up by 256.
        assert_eq!(image(RingOp::Sub, W, U8, (0, 3), (10, 10)), (246, 249));
    }

    #[test]
    fn a_range_that_crosses_a_wrap_point_is_the_whole_type() {
        use OverflowRule::Wrapping as W;
        assert_eq!(image(RingOp::Add, W, U8, (250, 255), (0, 10)), U8);
        assert_eq!(image(RingOp::Sub, W, U64, U64, U64), U64);
        let max = u64::MAX as i128;
        assert_eq!(image(RingOp::Mul, W, U64, (max, max), (max, max)), U64);
    }

    #[test]
    fn a_saturating_range_is_its_exact_range_clamped() {
        use OverflowRule::Saturating as S;
        assert_eq!(image(RingOp::Sub, S, U8, (0, 255), (0, 255)), (0, 255));
        assert_eq!(image(RingOp::Sub, S, U8, (100, 200), (150, 150)), (0, 50));
        assert_eq!(
            image(RingOp::Add, S, I8, (100, 127), (100, 127)),
            (127, 127)
        );
    }

    #[test]
    fn an_exact_product_past_i128_is_bounded_not_wrapped() {
        let max = u64::MAX as i128;
        assert_eq!(exact_range(RingOp::Mul, (0, max), (0, max)), (0, i128::MAX));
    }
}
