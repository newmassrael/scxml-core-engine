// sce_forge_runtime — interpolation
// 1D linear and 2D bilinear table interpolation. See SCE_FORGE.md §4.9.

#pragma once

#include <cstddef>

namespace sce::forge {

/// 1D linear interpolation. Axis must be strictly increasing.
/// Out-of-range x is clamped (extrapolate / error policies are applied by
/// the generated code before this call).
template <std::size_t N>
constexpr double linear(const double (&axis)[N],
                        const double (&values)[N],
                        double x) noexcept {
    static_assert(N >= 2, "linear interpolation requires at least 2 axis points");

    if (x <= axis[0]) return values[0];
    if (x >= axis[N - 1]) return values[N - 1];

    for (std::size_t i = 0; i + 1 < N; ++i) {
        if (x <= axis[i + 1]) {
            const double x0 = axis[i];
            const double x1 = axis[i + 1];
            const double y0 = values[i];
            const double y1 = values[i + 1];
            const double t = (x - x0) / (x1 - x0);
            return y0 + t * (y1 - y0);
        }
    }
    return values[N - 1];
}

/// 2D bilinear interpolation. `axis_x` indexes rows, `axis_y` indexes columns,
/// `values[r][c]` is row-major. Both inputs are clamped to their axis ranges.
template <std::size_t Rows, std::size_t Cols>
constexpr double bilinear(const double (&axis_x)[Rows],
                          const double (&axis_y)[Cols],
                          const double (&values)[Rows][Cols],
                          double x,
                          double y) noexcept {
    static_assert(Rows >= 2, "bilinear interpolation requires at least 2 rows");
    static_assert(Cols >= 2, "bilinear interpolation requires at least 2 cols");

    std::size_t r0 = 0;
    std::size_t r1 = 0;
    double tx = 0.0;
    if (x <= axis_x[0]) {
        r0 = 0;
        r1 = 0;
        tx = 0.0;
    } else if (x >= axis_x[Rows - 1]) {
        r0 = Rows - 1;
        r1 = Rows - 1;
        tx = 0.0;
    } else {
        for (std::size_t i = 0; i + 1 < Rows; ++i) {
            if (x <= axis_x[i + 1]) {
                r0 = i;
                r1 = i + 1;
                tx = (x - axis_x[i]) / (axis_x[i + 1] - axis_x[i]);
                break;
            }
        }
    }

    std::size_t c0 = 0;
    std::size_t c1 = 0;
    double ty = 0.0;
    if (y <= axis_y[0]) {
        c0 = 0;
        c1 = 0;
        ty = 0.0;
    } else if (y >= axis_y[Cols - 1]) {
        c0 = Cols - 1;
        c1 = Cols - 1;
        ty = 0.0;
    } else {
        for (std::size_t j = 0; j + 1 < Cols; ++j) {
            if (y <= axis_y[j + 1]) {
                c0 = j;
                c1 = j + 1;
                ty = (y - axis_y[j]) / (axis_y[j + 1] - axis_y[j]);
                break;
            }
        }
    }

    const double v00 = values[r0][c0];
    const double v01 = values[r0][c1];
    const double v10 = values[r1][c0];
    const double v11 = values[r1][c1];

    const double v0 = v00 + tx * (v10 - v00);
    const double v1 = v01 + tx * (v11 - v01);
    return v0 + ty * (v1 - v0);
}

} // namespace sce::forge
