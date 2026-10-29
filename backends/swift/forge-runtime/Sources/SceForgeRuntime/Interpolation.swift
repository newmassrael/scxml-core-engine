// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The table interpolation of the `interpolation` kind (SCE_FORGE.md Section 4.9): 1D linear and
// 2D bilinear, both clamping an input outside its axis to the axis' end. Siblings:
// `com.sce.forge.runtime.linear`/`bilinear` (Kotlin), `sce_forge_runtime::interpolation` (Rust)
// and `scealgorithm.Linear`/`Bilinear` (Go). A document that asks for any other out-of-range
// policy is refused where it is generated, so no caller reaches this function with one.

/// The reads of an interpolation table.
public enum Interpolation {
    /// 1D linear interpolation over `axis`, which is sorted in strictly increasing order.
    ///
    /// An input at or below the first breakpoint answers the first value and one at or above the
    /// last answers the last. An input that is neither (a NaN is) is placed by the first breakpoint
    /// it does not pass, and one no breakpoint is placed by answers the last value, as the other
    /// backends answer it.
    public static func linear(_ axis: [Double], _ values: [Double], _ x: Double) -> Double {
        let n = axis.count
        if n == 0 {
            return 0.0
        }
        if n == 1 || x <= axis[0] {
            return values[0]
        }
        if x >= axis[n - 1] {
            return values[n - 1]
        }
        for i in 0..<(n - 1) where x <= axis[i + 1] {
            let t = (x - axis[i]) / (axis[i + 1] - axis[i])
            return values[i] + t * (values[i + 1] - values[i])
        }
        return values[n - 1]
    }

    /// 2D bilinear interpolation: `axisX` indexes rows, `axisY` indexes columns and `values[r][c]`
    /// is row-major. Both inputs are clamped to their axis ranges.
    public static func bilinear(
        _ axisX: [Double], _ axisY: [Double], _ values: [[Double]], _ x: Double, _ y: Double
    ) -> Double {
        if axisX.isEmpty || axisY.isEmpty {
            return 0.0
        }
        let (r0, r1, tx) = locate(axisX, x)
        let (c0, c1, ty) = locate(axisY, y)
        let v00 = values[r0][c0]
        let v01 = values[r0][c1]
        let v10 = values[r1][c0]
        let v11 = values[r1][c1]
        let v0 = v00 + tx * (v10 - v00)
        let v1 = v01 + tx * (v11 - v01)
        return v0 + ty * (v1 - v0)
    }

    /// The two breakpoints `value` lies between and how far along them it is; the end it is
    /// clamped to, twice, where it lies outside the axis.
    private static func locate(_ axis: [Double], _ value: Double) -> (Int, Int, Double) {
        let n = axis.count
        if n == 1 || value <= axis[0] {
            return (0, 0, 0.0)
        }
        if value >= axis[n - 1] {
            return (n - 1, n - 1, 0.0)
        }
        for i in 0..<(n - 1) where value <= axis[i + 1] {
            return (i, i + 1, (value - axis[i]) / (axis[i + 1] - axis[i]))
        }
        return (n - 1, n - 1, 0.0)
    }
}
