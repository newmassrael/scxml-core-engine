// SCE-MAP: interpolation_2d_bilinear:1 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation") */
/* Runtime: sce_forge_runtime */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_INTERPOLATION_2D_BILINEAR_H
#define SCE_FORGE_INTERPOLATION_2D_BILINEAR_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

/* Pre-baked breakpoint and value tables. cpp/Rust use compile-time
 * generic templates over array sizes; C11 has no templates, so the
 * algorithm is inlined per-fixture with the array sizes substituted in
 * directly (`4`, `3`). Out-of-range inputs clamp to
 * the nearest endpoint — the same policy as cpp's linear()/bilinear(). */
static const double interpolation_2d_bilinear_axis_rpm[4] = { 800.0, 1200.0, 2000.0, 3000.0 };
static const double interpolation_2d_bilinear_axis_load[3] = { 10.0, 50.0, 100.0 };
static const double interpolation_2d_bilinear_values[4][3] = {
    { 2.1, 4.5, 7.0 },
    { 2.5, 5.0, 8.0 },
    { 3.0, 6.0, 9.5 },
    { 3.5, 7.0, 11.0 }
};

static inline double interpolation_2d_bilinear_lookup(uint16_t rpm, uint8_t load) {
    /* 2D bilinear: clamp on both axes, then bilinear over the 2x2 cell. */
    double sce_x = (double)rpm;
    double sce_y = (double)load;
    size_t sce_r0 = 0, sce_r1 = 0;
    double sce_tx = 0;
    if (sce_x <= interpolation_2d_bilinear_axis_rpm[0]) {
        sce_r0 = 0; sce_r1 = 0; sce_tx = 0;
    } else if (sce_x >= interpolation_2d_bilinear_axis_rpm[4 - 1]) {
        sce_r0 = 4 - 1; sce_r1 = 4 - 1; sce_tx = 0;
    } else {
        for (size_t sce_i = 0; sce_i + 1 < 4; ++sce_i) {
            if (sce_x <= interpolation_2d_bilinear_axis_rpm[sce_i + 1]) {
                sce_r0 = sce_i; sce_r1 = sce_i + 1;
                sce_tx = (sce_x - interpolation_2d_bilinear_axis_rpm[sce_i])
                   / (interpolation_2d_bilinear_axis_rpm[sce_i + 1]
                      - interpolation_2d_bilinear_axis_rpm[sce_i]);
                break;
            }
        }
    }
    size_t sce_c0 = 0, sce_c1 = 0;
    double sce_ty = 0;
    if (sce_y <= interpolation_2d_bilinear_axis_load[0]) {
        sce_c0 = 0; sce_c1 = 0; sce_ty = 0;
    } else if (sce_y >= interpolation_2d_bilinear_axis_load[3 - 1]) {
        sce_c0 = 3 - 1; sce_c1 = 3 - 1; sce_ty = 0;
    } else {
        for (size_t sce_j = 0; sce_j + 1 < 3; ++sce_j) {
            if (sce_y <= interpolation_2d_bilinear_axis_load[sce_j + 1]) {
                sce_c0 = sce_j; sce_c1 = sce_j + 1;
                sce_ty = (sce_y - interpolation_2d_bilinear_axis_load[sce_j])
                   / (interpolation_2d_bilinear_axis_load[sce_j + 1]
                      - interpolation_2d_bilinear_axis_load[sce_j]);
                break;
            }
        }
    }
    double sce_v00 = interpolation_2d_bilinear_values[sce_r0][sce_c0];
    double sce_v01 = interpolation_2d_bilinear_values[sce_r0][sce_c1];
    double sce_v10 = interpolation_2d_bilinear_values[sce_r1][sce_c0];
    double sce_v11 = interpolation_2d_bilinear_values[sce_r1][sce_c1];
    double sce_v0 = sce_v00 + sce_tx * (sce_v10 - sce_v00);
    double sce_v1 = sce_v01 + sce_tx * (sce_v11 - sce_v01);
    return sce_v0 + sce_ty * (sce_v1 - sce_v0);
}

#endif  /* SCE_FORGE_INTERPOLATION_2D_BILINEAR_H */
