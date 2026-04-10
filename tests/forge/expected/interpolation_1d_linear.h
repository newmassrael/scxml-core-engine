// SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation")
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_INTERPOLATION_1D_LINEAR_H
#define SCE_FORGE_INTERPOLATION_1D_LINEAR_H

#include <cstdint>
#include <cstddef>

namespace SCE::Generated::Interpolation1dLinear {

struct Interpolation1dLinear {
    static constexpr double AXIS_RPM[] = { 800.0, 1200.0, 2000.0, 3000.0, 4000.0, 6000.0 };
    static constexpr double VALUES[] = { 120.0, 145.0, 200.0, 230.0, 210.0, 180.0 };

    static double lookup(uint16_t rpm) {
        return linearInterpolate(
            AXIS_RPM, VALUES, 6,
            static_cast<double>(rpm));
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
};

}  // namespace SCE::Generated::Interpolation1dLinear

#endif  // SCE_FORGE_INTERPOLATION_1D_LINEAR_H