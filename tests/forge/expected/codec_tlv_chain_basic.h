// SCE-MAP: codec_tlv_chain_basic:16

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_TLV_CHAIN_BASIC_H
#define SCE_FORGE_CODEC_TLV_CHAIN_BASIC_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"
#include "codec_tlv_entry.h"

namespace SCE::Generated::CodecTlvChainBasic {

struct CodecTlvChainBasic {
    uint8_t header_flags;
    std::vector<::SCE::Generated::CodecTlvEntry::CodecTlvEntry> extensions;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecTlvChainBasic> decode(::SCE::Forge::SceCursor& cursor) {
        // RFC §5.B B2 repeat (trunk): per-field cursor advance. Fixed
        // prefix fields read via the present-if helper's non-gated
        // branch; repeat fields run a count-driven or until-eof loop
        // over the imported codec's decode(). Element NeedMoreBytes
        // unwinds the partial frame via std::nullopt.
        uint8_t header_flags;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            header_flags = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::vector<::SCE::Generated::CodecTlvEntry::CodecTlvEntry> extensions;
        extensions.reserve(8);
        for (std::size_t _i = 0; _i < 8; ++_i) {
            if (cursor.remaining() == 0) break;
            auto _elem = ::SCE::Generated::CodecTlvEntry::CodecTlvEntry::decode(cursor);
            if (!_elem.has_value()) return std::nullopt;
            extensions.push_back(*_elem);
        }
        if (cursor.remaining() > 0) return std::nullopt;
        return CodecTlvChainBasic{
            .header_flags = header_flags,
            .extensions = extensions,
        };
    }

    std::vector<uint8_t> encode() const {
        // RFC §5.B B2 encode: fixed prefix appends byte-by-byte;
        // repeat fields iterate the host vector and splice each
        // element's encode() into the parent buffer. Author keeps
        // count field == list length (trust contract).
        std::vector<uint8_t> r;
        r.reserve(273);
        r.push_back(header_flags);
        for (const auto& _e : extensions) {
            auto _sub = _e.encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        return r;
    }
};

}  // namespace SCE::Generated::CodecTlvChainBasic

#endif  // SCE_FORGE_CODEC_TLV_CHAIN_BASIC_H
