// SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation")
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_INTERPOLATION_2D_BILINEAR_H
#define SCE_FORGE_INTERPOLATION_2D_BILINEAR_H

#include <cstdint>
#include <cstddef>

namespace SCE::Generated::Interpolation2dBilinear {

struct Interpolation2dBilinear {
    static constexpr double AXIS_RPM[] = { 800.0, 1200.0, 2000.0, 3000.0 };
    static constexpr double AXIS_LOAD[] = { 10.0, 50.0, 100.0 };
    static constexpr double VALUES[4][3] = {
        { 2.1, 4.5, 7.0 },
        { 2.5, 5.0, 8.0 },
        { 3.0, 6.0, 9.5 },
        { 3.5, 7.0, 11.0 }
    };

    static double lookup(uint16_t rpm, uint8_t load) {
        return bilinearInterpolate(
            AXIS_RPM, 4,
            AXIS_LOAD, 3,
            VALUES,
            static_cast<double>(rpm),
            static_cast<double>(load));
    }

private:
    static double linearInterpolate(
            const double* axis, const double* values, size_t n,
            double x) {
        if (x <= axis[0]) return values[0];
        if (x >= axis[n - 1]) return values[n - 1];
        for (size_t i = 0; i + 1 < n; i++) {
            if (x <= axis[i + 1]) {
                double t = (x - axis[i]) / (axis[i + 1] - axis[i]);
                return values[i] + t * (values[i + 1] - values[i]);
            }
        }
        return values[n - 1];
    }

    static double bilinearInterpolate(
            const double* axisX, size_t nx,
            const double* axisY, size_t ny,
            const double table[][3],
            double x, double y) {
        if (x <= axisX[0]) x = axisX[0];
        else if (x >= axisX[nx - 1]) x = axisX[nx - 1];
        if (y <= axisY[0]) y = axisY[0];
        else if (y >= axisY[ny - 1]) y = axisY[ny - 1];
        size_t ix = 0, iy = 0;
        for (size_t i = 0; i + 1 < nx; i++) { if (x <= axisX[i + 1]) { ix = i; break; } ix = i; }
        for (size_t i = 0; i + 1 < ny; i++) { if (y <= axisY[i + 1]) { iy = i; break; } iy = i; }
        double tx = (x - axisX[ix]) / (axisX[ix + 1] - axisX[ix]);
        double ty = (y - axisY[iy]) / (axisY[iy + 1] - axisY[iy]);
        double c00 = table[ix][iy], c01 = table[ix][iy + 1];
        double c10 = table[ix + 1][iy], c11 = table[ix + 1][iy + 1];
        double a = c00 + tx * (c10 - c00);
        double b = c01 + tx * (c11 - c01);
        return a + ty * (b - a);
    }
};

}  // namespace SCE::Generated::Interpolation2dBilinear

#endif  // SCE_FORGE_INTERPOLATION_2D_BILINEAR_H