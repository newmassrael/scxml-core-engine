// SCE-MAP: codec_length_ref_dotted_basic:27

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_LENGTH_REF_DOTTED_BASIC_H
#define SCE_FORGE_CODEC_LENGTH_REF_DOTTED_BASIC_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecLengthRefDottedBasic {

struct CodecLengthRefDottedBasic {
    uint8_t carrier;
    std::vector<uint8_t> payload;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecLengthRefDottedBasic> decode(::SCE::Forge::SceCursor& cursor) {
        std::size_t _frame_len = cursor.remaining();
        if (_frame_len < 1) return std::nullopt;
        const std::uint8_t* raw = cursor.peek_slice(_frame_len);
        if (raw == nullptr) return std::nullopt;
        std::size_t len = _frame_len;  // shadowed for decode_expr(`raw + len`).
        (void)len;
        CodecLengthRefDottedBasic value{
            .carrier = raw[0],
            .payload = std::vector<uint8_t>(raw + 1, raw + 1 + ((raw[0] >> 4) & 0xF)),
        };
        if (!cursor.advance(_frame_len)) return std::nullopt;
        return value;
    }

    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    uint8_t hdr() const noexcept {
        return static_cast<uint8_t>(
            (this->carrier >> 0) & static_cast<uint8_t>(0x0F)
        );
    }

    void set_hdr(uint8_t v) noexcept {
        const uint8_t _shifted_mask =
            static_cast<uint8_t>(
                static_cast<uint8_t>(0x0F) << 0
            );
        const uint8_t _val =
            static_cast<uint8_t>(
                (static_cast<uint8_t>(v) & static_cast<uint8_t>(0x0F)) << 0
            );
        this->carrier = static_cast<uint8_t>(
            (this->carrier & static_cast<uint8_t>(~_shifted_mask)) | _val
        );
    }

    uint8_t payload_len() const noexcept {
        return static_cast<uint8_t>(
            (this->carrier >> 4) & static_cast<uint8_t>(0x0F)
        );
    }

    void set_payload_len(uint8_t v) noexcept {
        const uint8_t _shifted_mask =
            static_cast<uint8_t>(
                static_cast<uint8_t>(0x0F) << 4
            );
        const uint8_t _val =
            static_cast<uint8_t>(
                (static_cast<uint8_t>(v) & static_cast<uint8_t>(0x0F)) << 4
            );
        this->carrier = static_cast<uint8_t>(
            (this->carrier & static_cast<uint8_t>(~_shifted_mask)) | _val
        );
    }

    std::vector<uint8_t> encode() const {
        std::vector<uint8_t> r;
        r.reserve(16);
        r.push_back(carrier);
        r.insert(r.end(), payload.begin(), payload.end());
        return r;
    }
};

}  // namespace SCE::Generated::CodecLengthRefDottedBasic

#endif  // SCE_FORGE_CODEC_LENGTH_REF_DOTTED_BASIC_H
