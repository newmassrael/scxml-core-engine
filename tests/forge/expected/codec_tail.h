// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_TAIL_H
#define SCE_FORGE_CODEC_TAIL_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

namespace SCE::Generated::CodecTail {

struct CodecTail {
    uint8_t msgId;
    uint8_t status;
    std::vector<uint8_t> payload;

    static std::optional<CodecTail> decode(const uint8_t* raw, size_t len) {
        if (len < 2) return std::nullopt;
        return CodecTail{
            .msgId = raw[0],
            .status = raw[1],
            .payload = std::vector<uint8_t>(raw + 2, raw + len),
        };
    }

    std::vector<uint8_t> encode() const {
        std::vector<uint8_t> r;
        r.reserve(34);
        r.push_back(msgId);
        r.push_back(status);
        r.insert(r.end(), payload.begin(), payload.end());
        return r;
    }
};

}  // namespace SCE::Generated::CodecTail

#endif  // SCE_FORGE_CODEC_TAIL_H
