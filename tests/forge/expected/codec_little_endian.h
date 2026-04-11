// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_LITTLE_ENDIAN_H
#define SCE_FORGE_CODEC_LITTLE_ENDIAN_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

namespace SCE::Generated::CodecLittleEndian {

struct CodecLittleEndian {
    uint8_t sensorId;
    uint16_t value;
    uint8_t status;

    static std::optional<CodecLittleEndian> decode(const uint8_t* raw, size_t len) {
        if (len < 4) return std::nullopt;
        return CodecLittleEndian{
            .sensorId = raw[0],
            .value = raw[1] | (static_cast<uint16_t>(raw[2]) << 8),
            .status = raw[3],
        };
    }

    std::vector<uint8_t> encode() const {
        return {
            sensorId,
            static_cast<uint8_t>(value & 0xFF),
            static_cast<uint8_t>((value >> 8) & 0xFF),
            status
        };
    }
};

}  // namespace SCE::Generated::CodecLittleEndian

#endif  // SCE_FORGE_CODEC_LITTLE_ENDIAN_H
