// SCE-MAP: codec_peek_arm_a:13

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_PEEK_ARM_A_H
#define SCE_FORGE_CODEC_PEEK_ARM_A_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecPeekArmA {

struct CodecPeekArmA {
    uint8_t header;
    uint8_t payload;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecPeekArmA> decode(::SCE::Forge::SceCursor& cursor) {
        const std::uint8_t* raw = cursor.peek_slice(2);
        if (raw == nullptr) return std::nullopt;
        CodecPeekArmA value{
            .header = raw[0],
            .payload = raw[1],
        };
        if (!cursor.advance(2)) return std::nullopt;
        return value;
    }

    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool kind() const noexcept {
        return (this->header & 0x01) != 0;
    }

    void set_kind(bool v) noexcept {
        if (v) {
            this->header = static_cast<uint8_t>(this->header | 0x01);
        } else {
            this->header = static_cast<uint8_t>(this->header & static_cast<uint8_t>(~0x01));
        }
    }

    std::vector<uint8_t> encode() const {
        return {
            header,
            payload
        };
    }
};

}  // namespace SCE::Generated::CodecPeekArmA

#endif  // SCE_FORGE_CODEC_PEEK_ARM_A_H
