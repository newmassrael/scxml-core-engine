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
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohInterestBody> decode(::SCE::Forge::SceCursor& cursor) {
        // Streaming cursor decode (SSOT selection: `needs_streaming`).
        // The positional `raw[byte_off]` path is valid only when every
        // field's absolute offset is fixed at codegen time; this branch
        // handles every codec where it is not — present-if-gated fields
        // (runtime presence), VLE / repeat / TLV-chain / embed fields
        // (runtime width), string fields (UTF-8 decode), and a fixed field
        // after a variable-length payload (offset depends on the payload
        // length). Each field reads its own bytes from the cursor and
        // advances past exactly what it consumed. Per-field `is_repeat` /
        // `is_tlv_chain` / `is_embed` route to their dedicated helpers;
        // every other field flows through `present_if_decode_stmt`.
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

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
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

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 257;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable sinks
    /// (e.g. `VectorSink`) are effectively infallible.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w) const noexcept {
        // Streaming cursor encode (SSOT selection: `needs_streaming`).
        // Mirrors the streaming decode: every field appends its own bytes
        // in declaration order through the per-field encode blocks, so a
        // gated field skips its append when absent, and a fixed field after
        // a variable-length payload lands after the payload (the positional
        // path appends variable fields last, placing it ahead on the wire).
        // Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
        // dedicated helpers; everything else uses `present_if_encode_block`.
        if (auto _e = w.write_u8(header); _e) return _e;
        if (this->keyexpr.has_value()) {
            if (auto _e = this->keyexpr->encode(w, static_cast<std::uint8_t>((this->header >> 5) & 0x1)); _e) return _e;
        }
        return std::nullopt;
    }

    /// Heap-backed convenience facade. Pre-reserves `MAX_ENCODED_BYTES`
    /// so the worst-case write path performs at most one allocation,
    /// then delegates to `encode` over a `VectorSink`. Returns the
    /// freshly-encoded byte vector. Callers targeting zero-alloc hot
    /// paths should call `encode` directly against a caller-owned sink.
    [[nodiscard]] std::vector<std::uint8_t> encode_to_vec() const {
        std::vector<std::uint8_t> _sce_v;
        _sce_v.reserve(MAX_ENCODED_BYTES);
        ::SCE::Forge::VectorSink _sce_sink(_sce_v);
        (void)encode(_sce_sink);
        return _sce_v;
    }
};

}  // namespace SCE::Generated::CodecZenohInterestBody

#endif  // SCE_FORGE_CODEC_ZENOH_INTEREST_BODY_H
