// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_PRESENT_IF_DISJUNCTION_H
#define SCE_FORGE_CODEC_PRESENT_IF_DISJUNCTION_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecPresentIfDisjunction {

struct CodecPresentIfDisjunction {
    uint8_t flags;
    std::optional<uint16_t> seq;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecPresentIfDisjunction> decode(::SCE::Forge::SceCursor& cursor) {
        // RFC §5.B B1-δ + B2-β present-if: per-field cursor advance.
        // Gated fields hold std::optional<T>; B2-β extends gating to
        // Tail / LengthRef / Vle bit-sizes via dispatch inside
        // `present_if_decode_stmt`. Per-field `is_repeat` routes
        // Repeat fields to the dedicated helper. Branch fires before
        // has_vle_fields so a codec mixing VLE + present-if uses the
        // unified streaming path.
        uint8_t flags;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            flags = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<uint16_t> seq;
        if ((flags & 0x01) != 0 || (flags & 0x02) != 0) {
            const std::uint8_t* raw = cursor.peek_slice(2);
            if (raw == nullptr) return std::nullopt;
            seq = static_cast<uint16_t>((static_cast<uint16_t>(raw[0]) << 8) | raw[1]);
            if (!cursor.advance(2)) return std::nullopt;
        }
        return CodecPresentIfDisjunction{
            .flags = flags,
            .seq = seq,
        };
    }

    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool wants_a() const noexcept {
        return (this->flags & 0x01) != 0;
    }

    void set_wants_a(bool v) noexcept {
        if (v) {
            this->flags = static_cast<uint8_t>(this->flags | 0x01);
        } else {
            this->flags = static_cast<uint8_t>(this->flags & static_cast<uint8_t>(~0x01));
        }
    }

    bool wants_b() const noexcept {
        return (this->flags & 0x02) != 0;
    }

    void set_wants_b(bool v) noexcept {
        if (v) {
            this->flags = static_cast<uint8_t>(this->flags | 0x02);
        } else {
            this->flags = static_cast<uint8_t>(this->flags & static_cast<uint8_t>(~0x02));
        }
    }

    std::vector<uint8_t> encode() const {
        // RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        // append. Gated fields skip the append when the optional is
        // empty. Per-field `is_repeat` routes Repeat fields to the
        // dedicated helper. Branch fires before has_vle_fields so a
        // codec mixing VLE + present-if uses the unified encode path.
        std::vector<uint8_t> r;
        r.reserve(3);
        r.push_back(flags);
        if (seq.has_value()) {
            auto _v = *seq;
            r.push_back(static_cast<std::uint8_t>(_v >> 8));
            r.push_back(static_cast<std::uint8_t>(_v));
        }
        return r;
    }
};

}  // namespace SCE::Generated::CodecPresentIfDisjunction

#endif  // SCE_FORGE_CODEC_PRESENT_IF_DISJUNCTION_H
