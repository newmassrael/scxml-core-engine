// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_TRANSFORM_BITWISE_H
#define SCE_FORGE_TRANSFORM_BITWISE_H

#include <cstdint>
#include <string>

namespace SCE::Generated::TransformBitwise {

inline uint8_t computeHighNibble(uint8_t byte) {
    return byte >> 4 & 0x0F;
}

inline uint8_t computeLowNibble(uint8_t byte) {
    return byte & 0x0F;
}

}  // namespace SCE::Generated::TransformBitwise

#endif  // SCE_FORGE_TRANSFORM_BITWISE_H