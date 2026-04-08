// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_TRANSFORM_TEMPERATURE_H
#define SCE_FORGE_TRANSFORM_TEMPERATURE_H

#include <cstdint>
#include <string>

namespace SCE::Generated::TransformTemperature {

inline double computeTemperature(uint16_t raw) {
    return raw * 0.1 - 40.0;
}

}  // namespace SCE::Generated::TransformTemperature

#endif  // SCE_FORGE_TRANSFORM_TEMPERATURE_H