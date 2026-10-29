// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The builtins `round(real)` and `floor(real)`, which turn a real into an integer
// (SCE_FORGE.md §3.4.1): the runtime half of what the Swift emitter writes as
// `Real.round(x, to: Int64.self)`.
//
// `round` is half away from zero (0.5 is 1, -0.5 is -1) and `floor` is toward
// negative infinity, whatever the language's own rounding does at a tie. A value the
// target cannot hold stands at the nearest one it can, and NaN is 0: the answer Rust's
// `as` and Kotlin's `toLong` give, and the only one a conversion that must not trap
// can give, since Swift's `Int64(x)` traps on a value outside the range and on NaN.

/// A real turned into an integer by the contract's rounding, saturating at the
/// target's range.
public enum Real {
    /// `x` rounded half away from zero, as an integer of the type `target`.
    public static func round<T: FixedWidthInteger>(_ x: Double, to target: T.Type) -> T {
        saturate(x.rounded(.toNearestOrAwayFromZero), to: target)
    }

    /// `x` rounded toward negative infinity, as an integer of the type `target`.
    public static func floor<T: FixedWidthInteger>(_ x: Double, to target: T.Type) -> T {
        saturate(x.rounded(.down), to: target)
    }

    /// An integral `x` as `T`, or the nearest value `T` holds. `Double(T.max)` is
    /// exact up to 32 bits and rounds up to a power of two beyond that, so `x >=` it is
    /// the first `x` that no longer fits, and every `x` below it converts exactly.
    private static func saturate<T: FixedWidthInteger>(_ x: Double, to target: T.Type) -> T {
        if x.isNaN {
            return 0
        }
        if x >= Double(T.max) {
            return T.max
        }
        if x <= Double(T.min) {
            return T.min
        }
        return T(x)
    }
}
