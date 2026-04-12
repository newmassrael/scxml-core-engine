// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_SIMPLE_FRAME_H
#define SCE_FORGE_CODEC_SIMPLE_FRAME_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

namespace SCE::Generated::CodecSimpleFrame {

struct CodecSimpleFrame {
    uint8_t msgId;
    uint8_t length;
    uint16_t payload;

    static std::optional<CodecSimpleFrame> decode(const uint8_t* raw, size_t len) {
        if (len < 4) return std::nullopt;
        return CodecSimpleFrame{
            .msgId = raw[0],
            .length = raw[1],
            .payload = (static_cast<uint16_t>(raw[2]) << 8) | raw[3],
        };
    }

    std::vector<uint8_t> encode() const {
        return {
            msgId,
            length,
            static_cast<uint8_t>((payload >> 8) & 0xFF),
            static_cast<uint8_t>(payload & 0xFF)
        };
    }
};

}  // namespace SCE::Generated::CodecSimpleFrame

#endif  // SCE_FORGE_CODEC_SIMPLE_FRAME_H
