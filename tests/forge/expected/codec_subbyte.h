// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_SUBBYTE_H
#define SCE_FORGE_CODEC_SUBBYTE_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

namespace SCE::Generated::CodecSubbyte {

struct CodecSubbyte {
    uint8_t priority;
    uint8_t channel;
    uint8_t direction;

    static std::optional<CodecSubbyte> decode(const uint8_t* raw, size_t len) {
        if (len < 1) return std::nullopt;
        return CodecSubbyte{
            .priority = static_cast<uint8_t>((raw[0] >> 5) & 0x07),
            .channel = static_cast<uint8_t>((raw[0] >> 2) & 0x07),
            .direction = static_cast<uint8_t>((raw[0] >> 0) & 0x03),
        };
    }

    std::vector<uint8_t> encode() const {
        return {
            static_cast<uint8_t>(((priority & 0x07) << 5) | ((channel & 0x07) << 2) | ((direction & 0x03) << 0))
        };
    }
};

}  // namespace SCE::Generated::CodecSubbyte

#endif  // SCE_FORGE_CODEC_SUBBYTE_H
