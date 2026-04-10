// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// Package interpolation provides 1D linear and 2D bilinear table
// interpolation. See SCE_FORGE.md Section 4.9.
//
// Both functions clamp out-of-range inputs (the default `clamp` policy).
// `extrapolate` and `error` policies are applied by the generated wrapper
// before calling these helpers.
package interpolation

// Linear performs 1D linear interpolation. Axis must be sorted in strictly
// increasing order. Out-of-range x is clamped to the axis endpoints.
func Linear(axis, values []float64, x float64) float64 {
	n := len(axis)
	if n == 0 {
		return 0
	}
	if n == 1 || x <= axis[0] {
		return values[0]
	}
	if x >= axis[n-1] {
		return values[n-1]
	}
	for i := 0; i < n-1; i++ {
		if x <= axis[i+1] {
			x0 := axis[i]
			x1 := axis[i+1]
			y0 := values[i]
			y1 := values[i+1]
			t := (x - x0) / (x1 - x0)
			return y0 + t*(y1-y0)
		}
	}
	return values[n-1]
}

// Bilinear performs 2D bilinear interpolation. axisX indexes rows, axisY
// indexes columns, values[r][c] is row-major. Both inputs are clamped to
// their axis ranges.
func Bilinear(axisX, axisY []float64, values [][]float64, x, y float64) float64 {
	rows := len(axisX)
	cols := len(axisY)
	if rows == 0 || cols == 0 {
		return 0
	}

	r0, r1, tx := locateAxis(axisX, x)
	c0, c1, ty := locateAxis(axisY, y)

	v00 := values[r0][c0]
	v01 := values[r0][c1]
	v10 := values[r1][c0]
	v11 := values[r1][c1]

	v0 := v00 + tx*(v10-v00)
	v1 := v01 + tx*(v11-v01)
	return v0 + ty*(v1-v0)
}

// locateAxis returns the bracketing pair (low, high) and the linear weight t
// in [0, 1] for value along axis. Out-of-range values are clamped, in which
// case t is 0 and both indices point at the boundary.
func locateAxis(axis []float64, value float64) (int, int, float64) {
	n := len(axis)
	if n == 1 || value <= axis[0] {
		return 0, 0, 0
	}
	if value >= axis[n-1] {
		return n - 1, n - 1, 0
	}
	for i := 0; i < n-1; i++ {
		if value <= axis[i+1] {
			t := (value - axis[i]) / (axis[i+1] - axis[i])
			return i, i + 1, t
		}
	}
	return n - 1, n - 1, 0
}
