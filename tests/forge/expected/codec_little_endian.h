// SCE-MAP: codec_little_endian:3

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_LITTLE_ENDIAN_H
#define SCE_FORGE_CODEC_LITTLE_ENDIAN_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecLittleEndian {

struct CodecLittleEndian {
    uint8_t sensorId;
    uint16_t value;
    uint8_t status;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecLittleEndian> decode(::SCE::Forge::SceCursor& cursor) {
        const std::uint8_t* raw = cursor.peek_slice(4);
        if (raw == nullptr) return std::nullopt;
        CodecLittleEndian value{
            .sensorId = raw[0],
            .value = static_cast<uint16_t>(raw[1] | (static_cast<uint16_t>(raw[2]) << 8)),
            .status = raw[3],
        };
        if (!cursor.advance(4)) return std::nullopt;
        return value;
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
