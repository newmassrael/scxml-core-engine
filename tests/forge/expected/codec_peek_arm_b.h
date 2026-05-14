// SCE-MAP: codec_peek_arm_b:13

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_PEEK_ARM_B_H
#define SCE_FORGE_CODEC_PEEK_ARM_B_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecPeekArmB {

struct CodecPeekArmB {
    uint8_t header;
    uint16_t payload;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecPeekArmB> decode(::SCE::Forge::SceCursor& cursor) {
        const std::uint8_t* raw = cursor.peek_slice(3);
        if (raw == nullptr) return std::nullopt;
        CodecPeekArmB value{
            .header = raw[0],
            .payload = static_cast<uint16_t>((static_cast<uint16_t>(raw[1]) << 8) | raw[2]),
        };
        if (!cursor.advance(3)) return std::nullopt;
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
            static_cast<uint8_t>((payload >> 8) & 0xFF),
            static_cast<uint8_t>(payload & 0xFF)
        };
    }
};

}  // namespace SCE::Generated::CodecPeekArmB

#endif  // SCE_FORGE_CODEC_PEEK_ARM_B_H
