// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The integer operations that do not fail: see docs/adr/0063.
//
// An author names one where it is written (`wrappingSub(now, start)`,
// `saturatingAdd(level, step)`), and the generator lowers it to one of these helpers at
// the operands' type, in any forge expression: an algorithm's, may-fail or not, another
// kind's, or a sce-static machine's.
// For a type `T` of width `W` that holds `[m, M]` and the exact result `e` of `+ - *`
// or unary `-` in the integers:
//
// - `Wrapping` answers the `r` in `[m, M]` congruent to `e` modulo `2^W`, which for a
//   signed `T` is the two's-complement one;
// - `Saturating` answers `max(m, min(M, e))`.
//
// Both are total, so neither takes the `Failure` the `Checked` helpers record into. The
// wrapping helpers are Swift's own `&+ &- &*`, which compute exactly that at every
// width. The standard library has no saturating operation, so each saturating helper
// takes the `...ReportingOverflow` answer and, where the exact result does not fit,
// the bound on its side, which the signs of the operands tell.
//
// Cross-language siblings: the runtime row of every backend, listed in docs/adr/0063,
// section 8.

/// The wrapping integer operations, by the width of the type they are called at.
public enum Wrapping {
    /// `a + b` reduced modulo `2^W` into the range of `T`.
    public static func add<T: FixedWidthInteger>(_ a: T, _ b: T) -> T {
        a &+ b
    }

    /// `a - b` reduced modulo `2^W` into the range of `T`.
    public static func sub<T: FixedWidthInteger>(_ a: T, _ b: T) -> T {
        a &- b
    }

    /// `a * b` reduced modulo `2^W` into the range of `T`.
    public static func mul<T: FixedWidthInteger>(_ a: T, _ b: T) -> T {
        a &* b
    }

    /// `-a` reduced modulo `2^W` into the range of `T`: `MIN` for a signed `MIN`, and
    /// `2^W - a` for an unsigned `a` other than 0.
    public static func neg<T: FixedWidthInteger>(_ a: T) -> T {
        T.zero &- a
    }
}

/// The saturating integer operations, by the width of the type they are called at.
public enum Saturating {
    /// `a + b`, or the bound of `T` it passed. A sum leaves the range only past the
    /// bound on the side of `b`'s sign.
    public static func add<T: FixedWidthInteger>(_ a: T, _ b: T) -> T {
        let (value, overflow) = a.addingReportingOverflow(b)
        if overflow {
            return b < 0 ? T.min : T.max
        }
        return value
    }

    /// `a - b`, or the bound of `T` it passed. A difference leaves the range only past
    /// the bound on the side opposite to `b`'s sign: below `m` for a positive `b`, above
    /// `M` for a negative one.
    public static func sub<T: FixedWidthInteger>(_ a: T, _ b: T) -> T {
        let (value, overflow) = a.subtractingReportingOverflow(b)
        if overflow {
            return b > 0 ? T.min : T.max
        }
        return value
    }

    /// `a * b`, or the bound of `T` it passed. A product that leaves the range has two
    /// operands other than 0, and its exact value is negative exactly when their signs
    /// differ: `MIN * -1` is `M`.
    public static func mul<T: FixedWidthInteger>(_ a: T, _ b: T) -> T {
        let (value, overflow) = a.multipliedReportingOverflow(by: b)
        if overflow {
            return (a < 0) != (b < 0) ? T.min : T.max
        }
        return value
    }

    /// `-a`, or `M` for `-MIN`, the one negation that leaves the range. Signed types
    /// only: the saturating negation of an unsigned value is 0 for every input, and the
    /// build refuses it where it is written (docs/adr/0063, section 2).
    public static func neg<T: FixedWidthInteger & SignedInteger>(_ a: T) -> T {
        let (value, overflow) = T.zero.subtractingReportingOverflow(a)
        if overflow {
            return T.max
        }
        return value
    }
}
