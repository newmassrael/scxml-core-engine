// SCE-MAP: algorithm_crc16:11

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: free function in `namespace SCE::Generated::<Name>`. No
// STL containers, no exceptions. `bytes` lowers to `std::span<const
// std::uint8_t>` (RFC §synth-5-J-5 emitter table).

#pragma once
#ifndef SCE_FORGE_ALGORITHM_CRC16_H
#define SCE_FORGE_ALGORITHM_CRC16_H

#include <cstddef>
#include <cstdint>
#include <span>
namespace SCE::Generated::AlgorithmCrc16 {

inline uint16_t algorithm_crc16(std::span<const std::uint8_t> data) {
    uint16_t crc = 0xFFFF;
    for (std::uint8_t b : data) {
        uint16_t hi = b;
        crc = crc ^ hi << 8;
        uint8_t i = 0;
        while (i < 8) {
            if ((crc & 0x8000) != 0) {
                crc = crc << 1 ^ 0x1021;
            } else {
                crc = crc << 1;
            }
            i = i + 1;
        }
    }
    return crc;
}

}  // namespace SCE::Generated::AlgorithmCrc16

#endif  // SCE_FORGE_ALGORITHM_CRC16_H
