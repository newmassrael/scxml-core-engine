// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_FLAGS_BASIC_H
#define SCE_FORGE_CODEC_FLAGS_BASIC_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecFlagsBasic {

struct CodecFlagsBasic {
    uint8_t header;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecFlagsBasic> decode(::SCE::Forge::SceCursor& cursor) {
        const std::uint8_t* raw = cursor.peek_slice(1);
        if (raw == nullptr) return std::nullopt;
        CodecFlagsBasic value{
            .header = raw[0],
        };
        if (!cursor.advance(1)) return std::nullopt;
        return value;
    }

    // RFC §5.B B1-γ flags primitive: per-bit accessors over the carrier
    // field. Read returns a bool from `(field & mask) != 0`; write
    // toggles the bit without disturbing siblings on the same carrier.
    // Wire layout is unchanged — the carrier still occupies its
    // declared bytes.
    bool reliable() const noexcept {
        return (this->header & 0x80) != 0;
    }

    void set_reliable(bool v) noexcept {
        if (v) {
            this->header = static_cast<uint8_t>(this->header | 0x80);
        } else {
            this->header = static_cast<uint8_t>(this->header & static_cast<uint8_t>(~0x80));
        }
    }

    bool more() const noexcept {
        return (this->header & 0x40) != 0;
    }

    void set_more(bool v) noexcept {
        if (v) {
            this->header = static_cast<uint8_t>(this->header | 0x40);
        } else {
            this->header = static_cast<uint8_t>(this->header & static_cast<uint8_t>(~0x40));
        }
    }

    bool drop() const noexcept {
        return (this->header & 0x20) != 0;
    }

    void set_drop(bool v) noexcept {
        if (v) {
            this->header = static_cast<uint8_t>(this->header | 0x20);
        } else {
            this->header = static_cast<uint8_t>(this->header & static_cast<uint8_t>(~0x20));
        }
    }

    bool first() const noexcept {
        return (this->header & 0x10) != 0;
    }

    void set_first(bool v) noexcept {
        if (v) {
            this->header = static_cast<uint8_t>(this->header | 0x10);
        } else {
            this->header = static_cast<uint8_t>(this->header & static_cast<uint8_t>(~0x10));
        }
    }

    std::vector<uint8_t> encode() const {
        return {
            header
        };
    }
};

}  // namespace SCE::Generated::CodecFlagsBasic

#endif  // SCE_FORGE_CODEC_FLAGS_BASIC_H
