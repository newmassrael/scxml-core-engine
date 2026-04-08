// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_TRANSFORM_MULTI_OUTPUT_H
#define SCE_FORGE_TRANSFORM_MULTI_OUTPUT_H

#include <cstdint>
#include <string>

namespace SCE::Generated::TransformMultiOutput {

inline double computeFahrenheit(double celsius) {
    return celsius * 9 / 5 + 32;
}

inline double computeKelvin(double celsius) {
    return celsius + 273.15;
}

}  // namespace SCE::Generated::TransformMultiOutput

#endif  // SCE_FORGE_TRANSFORM_MULTI_OUTPUT_H