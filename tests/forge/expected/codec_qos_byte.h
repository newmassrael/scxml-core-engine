// SCE-MAP: codec_qos_byte:15

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_QOS_BYTE_H
#define SCE_FORGE_CODEC_QOS_BYTE_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecQosByte {

struct CodecQosByte {
    uint8_t qos;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecQosByte> decode(::SCE::Forge::SceCursor& cursor) {
        const std::uint8_t* raw = cursor.peek_slice(1);
        if (raw == nullptr) return std::nullopt;
        uint8_t qos = raw[0];
        CodecQosByte _decoded{
            .qos = qos,
        };
        if (!cursor.advance(1)) return std::nullopt;
        return _decoded;
    }

    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    uint8_t priority() const noexcept {
        return static_cast<uint8_t>(
            (this->qos >> 0) & static_cast<uint8_t>(0x07)
        );
    }

    void set_priority(uint8_t v) noexcept {
        const uint8_t _shifted_mask =
            static_cast<uint8_t>(
                static_cast<uint8_t>(0x07) << 0
            );
        const uint8_t _val =
            static_cast<uint8_t>(
                (static_cast<uint8_t>(v) & static_cast<uint8_t>(0x07)) << 0
            );
        this->qos = static_cast<uint8_t>(
            (this->qos & static_cast<uint8_t>(~_shifted_mask)) | _val
        );
    }

    bool reliable() const noexcept {
        return (this->qos & 0x08) != 0;
    }

    void set_reliable(bool v) noexcept {
        if (v) {
            this->qos = static_cast<uint8_t>(this->qos | 0x08);
        } else {
            this->qos = static_cast<uint8_t>(this->qos & static_cast<uint8_t>(~0x08));
        }
    }

    uint8_t congestion() const noexcept {
        return static_cast<uint8_t>(
            (this->qos >> 4) & static_cast<uint8_t>(0x03)
        );
    }

    void set_congestion(uint8_t v) noexcept {
        const uint8_t _shifted_mask =
            static_cast<uint8_t>(
                static_cast<uint8_t>(0x03) << 4
            );
        const uint8_t _val =
            static_cast<uint8_t>(
                (static_cast<uint8_t>(v) & static_cast<uint8_t>(0x03)) << 4
            );
        this->qos = static_cast<uint8_t>(
            (this->qos & static_cast<uint8_t>(~_shifted_mask)) | _val
        );
    }

    bool express() const noexcept {
        return (this->qos & 0x40) != 0;
    }

    void set_express(bool v) noexcept {
        if (v) {
            this->qos = static_cast<uint8_t>(this->qos | 0x40);
        } else {
            this->qos = static_cast<uint8_t>(this->qos & static_cast<uint8_t>(~0x40));
        }
    }

    bool reserved() const noexcept {
        return (this->qos & 0x80) != 0;
    }

    void set_reserved(bool v) noexcept {
        if (v) {
            this->qos = static_cast<uint8_t>(this->qos | 0x80);
        } else {
            this->qos = static_cast<uint8_t>(this->qos & static_cast<uint8_t>(~0x80));
        }
    }

    std::vector<uint8_t> encode() const {
        return {
            qos
        };
    }
};

}  // namespace SCE::Generated::CodecQosByte

#endif  // SCE_FORGE_CODEC_QOS_BYTE_H
