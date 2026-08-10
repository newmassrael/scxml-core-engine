// SCE-MAP: interpolation_2d_bilinear:1 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_INTERPOLATION_2D_BILINEAR_H
#define SCE_FORGE_INTERPOLATION_2D_BILINEAR_H

#include <cstdint>
#include <cstddef>
#include <sce/forge/interpolation.h>

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
        return SCE::Forge::bilinear(
            AXIS_RPM, AXIS_LOAD, VALUES,
            static_cast<double>(rpm),
            static_cast<double>(load));
    }
};

}  // namespace SCE::Generated::Interpolation2dBilinear

#endif  // SCE_FORGE_INTERPOLATION_2D_BILINEAR_H
