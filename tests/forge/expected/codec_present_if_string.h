// SCE-MAP: codec_present_if_string:47

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_PRESENT_IF_STRING_H
#define SCE_FORGE_CODEC_PRESENT_IF_STRING_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <string>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecPresentIfString {

struct CodecPresentIfString {
    uint8_t carrier;
    std::optional<uint8_t> text_len;
    std::optional<std::string> text;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecPresentIfString> decode(::SCE::Forge::SceCursor& cursor) {
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
        std::optional<uint8_t> text_len;
        if ((carrier & 0x01) != 0) {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            text_len = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<std::string> text;
        if ((carrier & 0x01) != 0) {
            std::size_t _n = static_cast<std::size_t>(text_len.value());
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            if (!::SCE::Forge::is_valid_utf8(raw, _n)) return std::nullopt;
            text.emplace(reinterpret_cast<const char*>(raw), _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        return CodecPresentIfString{
            .carrier = carrier,
            .text_len = text_len,
            .text = text,
        };
    }

    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool has_text() const noexcept {
        return (this->carrier & 0x01) != 0;
    }

    void set_has_text(bool v) noexcept {
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
        r.reserve(34);
        r.push_back(carrier);
        if (text_len.has_value()) {
            auto _v = *text_len;
            r.push_back(_v);
        }
        if (text.has_value()) {
            r.insert(r.end(),
                reinterpret_cast<const std::uint8_t*>(text->data()),
                reinterpret_cast<const std::uint8_t*>(text->data()) + text->size());
        }
        return r;
    }
};

}  // namespace SCE::Generated::CodecPresentIfString

#endif  // SCE_FORGE_CODEC_PRESENT_IF_STRING_H
