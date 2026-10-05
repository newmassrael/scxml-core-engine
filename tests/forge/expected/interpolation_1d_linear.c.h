// SCE-MAP: interpolation_1d_linear:1 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation") */
/* Runtime: sce_forge_runtime */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_INTERPOLATION_1D_LINEAR_H
#define SCE_FORGE_INTERPOLATION_1D_LINEAR_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

/* Pre-baked breakpoint and value tables. cpp/Rust use compile-time
 * generic templates over array sizes; C11 has no templates, so the
 * algorithm is inlined per-fixture with the array sizes substituted in
 * directly (`6`, `0`). Out-of-range inputs clamp to
 * the nearest endpoint — the same policy as cpp's linear()/bilinear(). */
static const double interpolation_1d_linear_axis_rpm[6] = { 800.0, 1200.0, 2000.0, 3000.0, 4000.0, 6000.0 };
static const double interpolation_1d_linear_values[6] = { 120.0, 145.0, 200.0, 230.0, 210.0, 180.0 };

static inline double interpolation_1d_linear_lookup(uint16_t rpm) {
    /* 1D linear: clamp out-of-range, otherwise locate cell and lerp. */
    double sce_x = (double)rpm;
    if (sce_x <= interpolation_1d_linear_axis_rpm[0]) {
        return interpolation_1d_linear_values[0];
    }
    if (sce_x >= interpolation_1d_linear_axis_rpm[6 - 1]) {
        return interpolation_1d_linear_values[6 - 1];
    }
    for (size_t sce_i = 0; sce_i + 1 < 6; ++sce_i) {
        if (sce_x <= interpolation_1d_linear_axis_rpm[sce_i + 1]) {
            double sce_x0 = interpolation_1d_linear_axis_rpm[sce_i];
            double sce_x1 = interpolation_1d_linear_axis_rpm[sce_i + 1];
            double sce_y0 = interpolation_1d_linear_values[sce_i];
            double sce_y1 = interpolation_1d_linear_values[sce_i + 1];
            double sce_t = (sce_x - sce_x0) / (sce_x1 - sce_x0);
            return sce_y0 + sce_t * (sce_y1 - sce_y0);
        }
    }
    return interpolation_1d_linear_values[6 - 1];
}

#endif  /* SCE_FORGE_INTERPOLATION_1D_LINEAR_H */
