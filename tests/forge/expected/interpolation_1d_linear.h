// SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation")
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_INTERPOLATION_1D_LINEAR_H
#define SCE_FORGE_INTERPOLATION_1D_LINEAR_H

#include <cstdint>
#include <cstddef>
#include <sce/forge/interpolation.h>

namespace SCE::Generated::Interpolation1dLinear {

struct Interpolation1dLinear {
    static constexpr double AXIS_RPM[] = { 800.0, 1200.0, 2000.0, 3000.0, 4000.0, 6000.0 };
    static constexpr double VALUES[] = { 120.0, 145.0, 200.0, 230.0, 210.0, 180.0 };

    static double lookup(uint16_t rpm) {
        return sce::forge::linear(
            AXIS_RPM, VALUES,
            static_cast<double>(rpm));
    }
};

}  // namespace SCE::Generated::Interpolation1dLinear

#endif  // SCE_FORGE_INTERPOLATION_1D_LINEAR_H
