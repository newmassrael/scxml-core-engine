// SCE-MAP: codec_zenoh_interest_body:56

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_INTEREST_BODY_H
#define SCE_FORGE_CODEC_ZENOH_INTEREST_BODY_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"
#include "codec_zenoh_wireexpr.h"

namespace SCE::Generated::CodecZenohInterestBody {

struct CodecZenohInterestBody {
    uint8_t header;
    std::optional<::SCE::Generated::CodecZenohWireexpr::CodecZenohWireexpr> keyexpr;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohInterestBody> decode(::SCE::Forge::SceCursor& cursor) {
        // RFC §5.B B1-δ + B2-β present-if: per-field cursor advance.
        // Gated fields hold std::optional<T>; B2-β extends gating to
        // Tail / LengthRef / Vle bit-sizes via dispatch inside
        // `present_if_decode_stmt`. Per-field `is_repeat` routes
        // Repeat fields to the dedicated helper. Branch fires before
        // has_vle_fields so a codec mixing VLE + present-if uses the
        // unified streaming path.
        uint8_t header;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            header = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<::SCE::Generated::CodecZenohWireexpr::CodecZenohWireexpr> keyexpr;
        if ((header & 0x10) != 0) {
            auto _emb = ::SCE::Generated::CodecZenohWireexpr::CodecZenohWireexpr::decode(cursor, static_cast<std::uint8_t>((header >> 5) & 0x1));
            if (!_emb.has_value()) return std::nullopt;
            keyexpr = std::move(*_emb);
        }
        return CodecZenohInterestBody{
            .header = header,
            .keyexpr = keyexpr,
        };
    }

    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool keyexprs() const noexcept {
        return (this->header & 0x01) != 0;
    }

    void set_keyexprs(bool v) noexcept {
        if (v) {
            this->header = static_cast<uint8_t>(this->header | 0x01);
        } else {
            this->header = static_cast<uint8_t>(this->header & static_cast<uint8_t>(~0x01));
        }
    }

    bool subscribers() const noexcept {
        return (this->header & 0x02) != 0;
    }

    void set_subscribers(bool v) noexcept {
        if (v) {
            this->header = static_cast<uint8_t>(this->header | 0x02);
        } else {
            this->header = static_cast<uint8_t>(this->header & static_cast<uint8_t>(~0x02));
        }
    }

    bool queryables() const noexcept {
        return (this->header & 0x04) != 0;
    }

    void set_queryables(bool v) noexcept {
        if (v) {
            this->header = static_cast<uint8_t>(this->header | 0x04);
        } else {
            this->header = static_cast<uint8_t>(this->header & static_cast<uint8_t>(~0x04));
        }
    }

    bool tokens() const noexcept {
        return (this->header & 0x08) != 0;
    }

    void set_tokens(bool v) noexcept {
        if (v) {
            this->header = static_cast<uint8_t>(this->header | 0x08);
        } else {
            this->header = static_cast<uint8_t>(this->header & static_cast<uint8_t>(~0x08));
        }
    }

    bool restricted() const noexcept {
        return (this->header & 0x10) != 0;
    }

    void set_restricted(bool v) noexcept {
        if (v) {
            this->header = static_cast<uint8_t>(this->header | 0x10);
        } else {
            this->header = static_cast<uint8_t>(this->header & static_cast<uint8_t>(~0x10));
        }
    }

    bool n() const noexcept {
        return (this->header & 0x20) != 0;
    }

    void set_n(bool v) noexcept {
        if (v) {
            this->header = static_cast<uint8_t>(this->header | 0x20);
        } else {
            this->header = static_cast<uint8_t>(this->header & static_cast<uint8_t>(~0x20));
        }
    }

    bool m() const noexcept {
        return (this->header & 0x40) != 0;
    }

    void set_m(bool v) noexcept {
        if (v) {
            this->header = static_cast<uint8_t>(this->header | 0x40);
        } else {
            this->header = static_cast<uint8_t>(this->header & static_cast<uint8_t>(~0x40));
        }
    }

    bool aggregate() const noexcept {
        return (this->header & 0x80) != 0;
    }

    void set_aggregate(bool v) noexcept {
        if (v) {
            this->header = static_cast<uint8_t>(this->header | 0x80);
        } else {
            this->header = static_cast<uint8_t>(this->header & static_cast<uint8_t>(~0x80));
        }
    }

    std::vector<uint8_t> encode() const {
        // RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        // append. Gated fields skip the append when the optional is
        // empty. Per-field `is_repeat` routes Repeat fields to the
        // dedicated helper. Branch fires before has_vle_fields so a
        // codec mixing VLE + present-if uses the unified encode path.
        std::vector<uint8_t> r;
        r.reserve(257);
        r.push_back(header);
        if (this->keyexpr.has_value()) {
            auto _sub = this->keyexpr->encode(static_cast<std::uint8_t>((this->header >> 5) & 0x1));
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        return r;
    }
};

}  // namespace SCE::Generated::CodecZenohInterestBody

#endif  // SCE_FORGE_CODEC_ZENOH_INTEREST_BODY_H
