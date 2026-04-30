// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_LENGTH_REF_H
#define SCE_FORGE_CODEC_LENGTH_REF_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

namespace SCE::Generated::CodecLengthRef {

struct CodecLengthRef {
    uint8_t msgId;
    uint8_t len;
    std::vector<uint8_t> payload;

    static std::optional<CodecLengthRef> decode(const uint8_t* raw, size_t len) {
        if (len < 2) return std::nullopt;
        return CodecLengthRef{
            .msgId = raw[0],
            .len = raw[1],
            .payload = std::vector<uint8_t>(raw + 2, raw + 2 + raw[1]),
        };
    }

    std::vector<uint8_t> encode() const {
        std::vector<uint8_t> r;
        r.reserve(34);
        r.push_back(msgId);
        r.push_back(len);
        r.insert(r.end(), payload.begin(), payload.end());
        return r;
    }
};

}  // namespace SCE::Generated::CodecLengthRef

#endif  // SCE_FORGE_CODEC_LENGTH_REF_H
