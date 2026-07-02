# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

"""1D linear and 2D bilinear table interpolation. See SCE_FORGE.md Section 4.9.

Both functions clamp out-of-range inputs (the default `clamp` policy).
`extrapolate` and `error` policies are applied by the generated wrapper before
calling these helpers.
"""

from typing import Sequence


def linear(axis: Sequence[float], values: Sequence[float], x: float) -> float:
    """1D linear interpolation. Axis must be sorted in strictly increasing order."""
    n = len(axis)
    if n == 0:
        return 0.0
    if n == 1 or x <= axis[0]:
        return values[0]
    if x >= axis[n - 1]:
        return values[n - 1]
    for i in range(n - 1):
        if x <= axis[i + 1]:
            x0 = axis[i]
            x1 = axis[i + 1]
            y0 = values[i]
            y1 = values[i + 1]
            t = (x - x0) / (x1 - x0)
            return y0 + t * (y1 - y0)
    return values[n - 1]


def bilinear(
    axis_x: Sequence[float],
    axis_y: Sequence[float],
    values: Sequence[Sequence[float]],
    x: float,
    y: float,
) -> float:
    """2D bilinear interpolation. `axis_x` indexes rows, `axis_y` indexes columns,
    `values[r][c]` is row-major. Both inputs are clamped to their axis ranges.
    """
    rows = len(axis_x)
    cols = len(axis_y)
    if rows == 0 or cols == 0:
        return 0.0

    r0, r1, tx = _locate_axis(axis_x, x)
    c0, c1, ty = _locate_axis(axis_y, y)

    v00 = values[r0][c0]
    v01 = values[r0][c1]
    v10 = values[r1][c0]
    v11 = values[r1][c1]

    v0 = v00 + tx * (v10 - v00)
    v1 = v01 + tx * (v11 - v01)
    return v0 + ty * (v1 - v0)


def _locate_axis(axis: Sequence[float], value: float) -> tuple[int, int, float]:
    """Locate the bracketing pair (i, i+1) and the linear weight t in [0, 1]
    for `value` along `axis`. Out-of-range values are clamped, in which case
    `t` is 0.0 and both indices point at the boundary.
    """
    n = len(axis)
    if n == 1 or value <= axis[0]:
        return (0, 0, 0.0)
    if value >= axis[n - 1]:
        return (n - 1, n - 1, 0.0)
    for i in range(n - 1):
        if value <= axis[i + 1]:
            t = (value - axis[i]) / (axis[i + 1] - axis[i])
            return (i, i + 1, t)
    return (n - 1, n - 1, 0.0)
