// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package com.sce.forge.runtime

/**
 * 1D linear and 2D bilinear table interpolation. See SCE_FORGE.md Section 4.9.
 *
 * Both functions clamp out-of-range inputs (the default `clamp` policy).
 * `extrapolate` and `error` policies are applied by the generated wrapper
 * before calling these helpers.
 */

/** 1D linear interpolation. Axis must be sorted in strictly increasing order. */
public fun linear(axis: DoubleArray, values: DoubleArray, x: Double): Double {
    val n = axis.size
    if (n == 0) return 0.0
    if (n == 1 || x <= axis[0]) return values[0]
    if (x >= axis[n - 1]) return values[n - 1]
    for (i in 0 until n - 1) {
        if (x <= axis[i + 1]) {
            val x0 = axis[i]
            val x1 = axis[i + 1]
            val y0 = values[i]
            val y1 = values[i + 1]
            val t = (x - x0) / (x1 - x0)
            return y0 + t * (y1 - y0)
        }
    }
    return values[n - 1]
}

/**
 * 2D bilinear interpolation. `axisX` indexes rows, `axisY` indexes columns,
 * `values[r][c]` is row-major. Both inputs are clamped to their axis ranges.
 */
public fun bilinear(
    axisX: DoubleArray,
    axisY: DoubleArray,
    values: Array<DoubleArray>,
    x: Double,
    y: Double,
): Double {
    val rows = axisX.size
    val cols = axisY.size
    if (rows == 0 || cols == 0) return 0.0

    val (r0, r1, tx) = locateAxis(axisX, x)
    val (c0, c1, ty) = locateAxis(axisY, y)

    val v00 = values[r0][c0]
    val v01 = values[r0][c1]
    val v10 = values[r1][c0]
    val v11 = values[r1][c1]

    val v0 = v00 + tx * (v10 - v00)
    val v1 = v01 + tx * (v11 - v01)
    return v0 + ty * (v1 - v0)
}

private data class AxisLocation(val low: Int, val high: Int, val t: Double)

private operator fun AxisLocation.component1(): Int = low
private operator fun AxisLocation.component2(): Int = high
private operator fun AxisLocation.component3(): Double = t

private fun locateAxis(axis: DoubleArray, value: Double): AxisLocation {
    val n = axis.size
    if (n == 1 || value <= axis[0]) return AxisLocation(0, 0, 0.0)
    if (value >= axis[n - 1]) return AxisLocation(n - 1, n - 1, 0.0)
    for (i in 0 until n - 1) {
        if (value <= axis[i + 1]) {
            val t = (value - axis[i]) / (axis[i + 1] - axis[i])
            return AxisLocation(i, i + 1, t)
        }
    }
    return AxisLocation(n - 1, n - 1, 0.0)
}
