// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! 1D linear and 2D bilinear table interpolation. See SCE_FORGE.md Section 4.9.
//!
//! Both functions clamp out-of-range inputs (the default `clamp` policy).
//! `extrapolate` and `error` policies are applied by the generated wrapper
//! before calling these helpers.

/// 1D linear interpolation. Axis must be sorted in strictly increasing order.
/// Out-of-range x is clamped to the axis endpoints.
pub fn linear<const N: usize>(axis: &[f64; N], values: &[f64; N], x: f64) -> f64 {
    if N == 0 {
        return 0.0;
    }
    if N == 1 || x <= axis[0] {
        return values[0];
    }
    if x >= axis[N - 1] {
        return values[N - 1];
    }
    let mut i = 0;
    while i + 1 < N {
        if x <= axis[i + 1] {
            let x0 = axis[i];
            let x1 = axis[i + 1];
            let y0 = values[i];
            let y1 = values[i + 1];
            let t = (x - x0) / (x1 - x0);
            return y0 + t * (y1 - y0);
        }
        i += 1;
    }
    values[N - 1]
}

/// 2D bilinear interpolation. `axis_x` indexes rows, `axis_y` indexes columns,
/// `values[r][c]` is row-major. Both inputs are clamped to their axis ranges.
pub fn bilinear<const ROWS: usize, const COLS: usize>(
    axis_x: &[f64; ROWS],
    axis_y: &[f64; COLS],
    values: &[[f64; COLS]; ROWS],
    x: f64,
    y: f64,
) -> f64 {
    if ROWS == 0 || COLS == 0 {
        return 0.0;
    }

    let (r0, r1, tx) = locate_axis::<ROWS>(axis_x, x);
    let (c0, c1, ty) = locate_axis::<COLS>(axis_y, y);

    let v00 = values[r0][c0];
    let v01 = values[r0][c1];
    let v10 = values[r1][c0];
    let v11 = values[r1][c1];

    let v0 = v00 + tx * (v10 - v00);
    let v1 = v01 + tx * (v11 - v01);
    v0 + ty * (v1 - v0)
}

/// Locate the bracketing pair `(i, i+1)` and the linear weight `t` in `[0, 1]`
/// for `value` along `axis`. Out-of-range values are clamped, in which case
/// `t` is `0.0` and both indices point at the boundary.
fn locate_axis<const N: usize>(axis: &[f64; N], value: f64) -> (usize, usize, f64) {
    if N == 1 || value <= axis[0] {
        return (0, 0, 0.0);
    }
    if value >= axis[N - 1] {
        return (N - 1, N - 1, 0.0);
    }
    let mut i = 0;
    while i + 1 < N {
        if value <= axis[i + 1] {
            let t = (value - axis[i]) / (axis[i + 1] - axis[i]);
            return (i, i + 1, t);
        }
        i += 1;
    }
    (N - 1, N - 1, 0.0)
}
