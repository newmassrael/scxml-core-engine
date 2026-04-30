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
    double x = (double)rpm;
    double y = (double)load;
    size_t r0 = 0, r1 = 0;
    double tx = 0;
    if (x <= interpolation_2d_bilinear_axis_rpm[0]) {
        r0 = 0; r1 = 0; tx = 0;
    } else if (x >= interpolation_2d_bilinear_axis_rpm[4 - 1]) {
        r0 = 4 - 1; r1 = 4 - 1; tx = 0;
    } else {
        for (size_t i = 0; i + 1 < 4; ++i) {
            if (x <= interpolation_2d_bilinear_axis_rpm[i + 1]) {
                r0 = i; r1 = i + 1;
                tx = (x - interpolation_2d_bilinear_axis_rpm[i])
                   / (interpolation_2d_bilinear_axis_rpm[i + 1]
                      - interpolation_2d_bilinear_axis_rpm[i]);
                break;
            }
        }
    }
    size_t c0 = 0, c1 = 0;
    double ty = 0;
    if (y <= interpolation_2d_bilinear_axis_load[0]) {
        c0 = 0; c1 = 0; ty = 0;
    } else if (y >= interpolation_2d_bilinear_axis_load[3 - 1]) {
        c0 = 3 - 1; c1 = 3 - 1; ty = 0;
    } else {
        for (size_t j = 0; j + 1 < 3; ++j) {
            if (y <= interpolation_2d_bilinear_axis_load[j + 1]) {
                c0 = j; c1 = j + 1;
                ty = (y - interpolation_2d_bilinear_axis_load[j])
                   / (interpolation_2d_bilinear_axis_load[j + 1]
                      - interpolation_2d_bilinear_axis_load[j]);
                break;
            }
        }
    }
    double v00 = interpolation_2d_bilinear_values[r0][c0];
    double v01 = interpolation_2d_bilinear_values[r0][c1];
    double v10 = interpolation_2d_bilinear_values[r1][c0];
    double v11 = interpolation_2d_bilinear_values[r1][c1];
    double v0 = v00 + tx * (v10 - v00);
    double v1 = v01 + tx * (v11 - v01);
    return v0 + ty * (v1 - v0);
}

#endif  /* SCE_FORGE_INTERPOLATION_2D_BILINEAR_H */
