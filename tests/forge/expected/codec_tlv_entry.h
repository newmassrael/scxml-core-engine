// SCE-MAP: codec_tlv_entry:10

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_TLV_ENTRY_H
#define SCE_FORGE_CODEC_TLV_ENTRY_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecTlvEntry {

struct CodecTlvEntry {
    uint8_t entry_type;
    uint8_t entry_len;
    std::vector<uint8_t> entry_body;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecTlvEntry> decode(::SCE::Forge::SceCursor& cursor) {
        std::size_t _frame_len = cursor.remaining();
        if (_frame_len < 2) return std::nullopt;
        const std::uint8_t* raw = cursor.peek_slice(_frame_len);
        if (raw == nullptr) return std::nullopt;
        uint8_t entry_type = raw[0];
        uint8_t entry_len = raw[1];
        std::vector<uint8_t> entry_body = std::vector<uint8_t>(raw + 2, raw + 2 + entry_len);
        CodecTlvEntry _decoded{
            .entry_type = entry_type,
            .entry_len = entry_len,
            .entry_body = entry_body,
        };
        if (!cursor.advance(_frame_len)) return std::nullopt;
        return _decoded;
    }

    std::vector<uint8_t> encode() const {
        std::vector<uint8_t> r;
        r.reserve(34);
        r.push_back(entry_type);
        r.push_back(entry_len);
        r.insert(r.end(), entry_body.begin(), entry_body.end());
        return r;
    }
};

}  // namespace SCE::Generated::CodecTlvEntry

#endif  // SCE_FORGE_CODEC_TLV_ENTRY_H
