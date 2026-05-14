// SCE-MAP: codec_tlv_chain_present_if_basic:37

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_TLV_CHAIN_PRESENT_IF_BASIC_H
#define SCE_FORGE_CODEC_TLV_CHAIN_PRESENT_IF_BASIC_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"
#include "codec_tlv_entry.h"

namespace SCE::Generated::CodecTlvChainPresentIfBasic {

struct CodecTlvChainPresentIfBasic {
    uint8_t carrier;
    std::optional<std::vector<::SCE::Generated::CodecTlvEntry::CodecTlvEntry>> entries;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecTlvChainPresentIfBasic> decode(::SCE::Forge::SceCursor& cursor) {
        // RFC §5.B B1-δ + B2-β present-if: per-field cursor advance.
        // Gated fields hold std::optional<T>; B2-β extends gating to
        // Tail / LengthRef / Vle bit-sizes via dispatch inside
        // `present_if_decode_stmt`. Per-field `is_repeat` routes
        // Repeat fields to the dedicated helper. Branch fires before
        // has_vle_fields so a codec mixing VLE + present-if uses the
        // unified streaming path.
        uint8_t carrier;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            carrier = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<std::vector<::SCE::Generated::CodecTlvEntry::CodecTlvEntry>> entries;
        if ((carrier & 0x01) != 0) {
            std::vector<::SCE::Generated::CodecTlvEntry::CodecTlvEntry> _list;
            _list.reserve(4);
            for (std::size_t _i = 0; _i < 4; ++_i) {
                if (cursor.remaining() == 0) break;
                auto _elem = ::SCE::Generated::CodecTlvEntry::CodecTlvEntry::decode(cursor);
                if (!_elem.has_value()) return std::nullopt;
                _list.push_back(*_elem);
            }
            if (cursor.remaining() > 0) return std::nullopt;
            entries = std::move(_list);
        }
        return CodecTlvChainPresentIfBasic{
            .carrier = carrier,
            .entries = entries,
        };
    }

    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool has_chain() const noexcept {
        return (this->carrier & 0x01) != 0;
    }

    void set_has_chain(bool v) noexcept {
        if (v) {
            this->carrier = static_cast<uint8_t>(this->carrier | 0x01);
        } else {
            this->carrier = static_cast<uint8_t>(this->carrier & static_cast<uint8_t>(~0x01));
        }
    }

    std::vector<uint8_t> encode() const {
        // RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        // append. Gated fields skip the append when the optional is
        // empty. Per-field `is_repeat` routes Repeat fields to the
        // dedicated helper. Branch fires before has_vle_fields so a
        // codec mixing VLE + present-if uses the unified encode path.
        std::vector<uint8_t> r;
        r.reserve(137);
        r.push_back(carrier);
        if (this->entries.has_value()) {
            for (const auto& _e : *this->entries) {
                auto _sub = _e.encode();
                r.insert(r.end(), _sub.begin(), _sub.end());
            }
        }
        return r;
    }
};

}  // namespace SCE::Generated::CodecTlvChainPresentIfBasic

#endif  // SCE_FORGE_CODEC_TLV_CHAIN_PRESENT_IF_BASIC_H
