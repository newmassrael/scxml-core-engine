// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_DECL_EXT_KEYEXPR_INNER_H
#define SCE_FORGE_CODEC_ZENOH_DECL_EXT_KEYEXPR_INNER_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecZenohDeclExtKeyexprInner {

struct CodecZenohDeclExtKeyexprInner {
    uint8_t inner_header;
    uint64_t id;
    std::optional<std::vector<uint8_t>> suffix;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohDeclExtKeyexprInner> decode(::SCE::Forge::SceCursor& cursor) {
        // RFC §5.B B1-δ + B2-β present-if: per-field cursor advance.
        // Gated fields hold std::optional<T>; B2-β extends gating to
        // Tail / LengthRef / Vle bit-sizes via dispatch inside
        // `present_if_decode_stmt`. Per-field `is_repeat` routes
        // Repeat fields to the dedicated helper. Branch fires before
        // has_vle_fields so a codec mixing VLE + present-if uses the
        // unified streaming path.
        uint8_t inner_header;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            inner_header = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        auto id_opt = cursor.read_vle_u64();
        if (!id_opt.has_value()) return std::nullopt;
        auto id = static_cast<std::uint64_t>(*id_opt);
        std::optional<std::vector<uint8_t>> suffix;
        if ((inner_header & 0x01) != 0) {
            std::size_t _n = cursor.remaining();
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            suffix.emplace(raw, raw + _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        return CodecZenohDeclExtKeyexprInner{
            .inner_header = inner_header,
            .id = id,
            .suffix = suffix,
        };
    }

    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool n() const noexcept {
        return (this->inner_header & 0x01) != 0;
    }

    void set_n(bool v) noexcept {
        if (v) {
            this->inner_header = static_cast<uint8_t>(this->inner_header | 0x01);
        } else {
            this->inner_header = static_cast<uint8_t>(this->inner_header & static_cast<uint8_t>(~0x01));
        }
    }

    bool m() const noexcept {
        return (this->inner_header & 0x02) != 0;
    }

    void set_m(bool v) noexcept {
        if (v) {
            this->inner_header = static_cast<uint8_t>(this->inner_header | 0x02);
        } else {
            this->inner_header = static_cast<uint8_t>(this->inner_header & static_cast<uint8_t>(~0x02));
        }
    }

    std::vector<uint8_t> encode() const {
        // RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        // append. Gated fields skip the append when the optional is
        // empty. Per-field `is_repeat` routes Repeat fields to the
        // dedicated helper. Branch fires before has_vle_fields so a
        // codec mixing VLE + present-if uses the unified encode path.
        std::vector<uint8_t> r;
        r.reserve(139);
        r.push_back(inner_header);
        {
            std::uint64_t _w = static_cast<std::uint64_t>(id);
            while (_w >= 0x80) {
                r.push_back(static_cast<std::uint8_t>((_w & 0x7F) | 0x80));
                _w >>= 7;
            }
            r.push_back(static_cast<std::uint8_t>(_w));
        }
        if (suffix.has_value()) {
            r.insert(r.end(), suffix->begin(), suffix->end());
        }
        return r;
    }
};

}  // namespace SCE::Generated::CodecZenohDeclExtKeyexprInner

#endif  // SCE_FORGE_CODEC_ZENOH_DECL_EXT_KEYEXPR_INNER_H
