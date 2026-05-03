// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_SCOUT_H
#define SCE_FORGE_CODEC_ZENOH_SCOUT_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecZenohScout {

struct CodecZenohScout {
    uint8_t version;
    uint8_t cbyte;
    std::optional<std::vector<uint8_t>> zid;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohScout> decode(::SCE::Forge::SceCursor& cursor) {
        // RFC §5.B B1-δ + B2-β present-if: per-field cursor advance.
        // Gated fields hold std::optional<T>; B2-β extends gating to
        // Tail / LengthRef / Vle bit-sizes via dispatch inside
        // `present_if_decode_stmt`. Per-field `is_repeat` routes
        // Repeat fields to the dedicated helper. Branch fires before
        // has_vle_fields so a codec mixing VLE + present-if uses the
        // unified streaming path.
        uint8_t version;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            version = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        uint8_t cbyte;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            cbyte = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<std::vector<uint8_t>> zid;
        if ((cbyte & 0x08) != 0) {
            std::size_t _n = static_cast<std::size_t>((cbyte >> 4) & 0xF) + 1;
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            zid.emplace(raw, raw + _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        return CodecZenohScout{
            .version = version,
            .cbyte = cbyte,
            .zid = zid,
        };
    }

    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    uint8_t what() const noexcept {
        return static_cast<uint8_t>(
            (this->cbyte >> 0) & static_cast<uint8_t>(0x07)
        );
    }

    void set_what(uint8_t v) noexcept {
        const uint8_t _shifted_mask =
            static_cast<uint8_t>(
                static_cast<uint8_t>(0x07) << 0
            );
        const uint8_t _val =
            static_cast<uint8_t>(
                (static_cast<uint8_t>(v) & static_cast<uint8_t>(0x07)) << 0
            );
        this->cbyte = static_cast<uint8_t>(
            (this->cbyte & static_cast<uint8_t>(~_shifted_mask)) | _val
        );
    }

    bool i() const noexcept {
        return (this->cbyte & 0x08) != 0;
    }

    void set_i(bool v) noexcept {
        if (v) {
            this->cbyte = static_cast<uint8_t>(this->cbyte | 0x08);
        } else {
            this->cbyte = static_cast<uint8_t>(this->cbyte & static_cast<uint8_t>(~0x08));
        }
    }

    uint8_t zid_len_m1() const noexcept {
        return static_cast<uint8_t>(
            (this->cbyte >> 4) & static_cast<uint8_t>(0x0F)
        );
    }

    void set_zid_len_m1(uint8_t v) noexcept {
        const uint8_t _shifted_mask =
            static_cast<uint8_t>(
                static_cast<uint8_t>(0x0F) << 4
            );
        const uint8_t _val =
            static_cast<uint8_t>(
                (static_cast<uint8_t>(v) & static_cast<uint8_t>(0x0F)) << 4
            );
        this->cbyte = static_cast<uint8_t>(
            (this->cbyte & static_cast<uint8_t>(~_shifted_mask)) | _val
        );
    }

    std::vector<uint8_t> encode() const {
        // RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        // append. Gated fields skip the append when the optional is
        // empty. Per-field `is_repeat` routes Repeat fields to the
        // dedicated helper. Branch fires before has_vle_fields so a
        // codec mixing VLE + present-if uses the unified encode path.
        std::vector<uint8_t> r;
        r.reserve(18);
        r.push_back(version);
        r.push_back(cbyte);
        if (zid.has_value()) {
            r.insert(r.end(), zid->begin(), zid->end());
        }
        return r;
    }
};

}  // namespace SCE::Generated::CodecZenohScout

#endif  // SCE_FORGE_CODEC_ZENOH_SCOUT_H
