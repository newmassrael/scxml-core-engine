// SCE-MAP: codec_fixed_after_lengthref:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_FIXED_AFTER_LENGTHREF_H
#define SCE_FORGE_CODEC_FIXED_AFTER_LENGTHREF_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecFixedAfterLengthref {

struct CodecFixedAfterLengthref {
    uint8_t header;
    uint16_t payload_len;
    std::vector<uint8_t> payload;
    uint32_t crc32;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecFixedAfterLengthref> decode(::SCE::Forge::SceCursor& cursor) {
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
        uint16_t payload_len;
        {
            const std::uint8_t* raw = cursor.peek_slice(2);
            if (raw == nullptr) return std::nullopt;
            payload_len = static_cast<uint16_t>(static_cast<uint16_t>(raw[0] | (static_cast<uint16_t>(raw[1]) << 8)));
            if (!cursor.advance(2)) return std::nullopt;
        }
        std::vector<uint8_t> payload;
        {
            std::size_t _n = static_cast<std::size_t>(payload_len);
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            payload.assign(raw, raw + _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        uint32_t crc32;
        {
            const std::uint8_t* raw = cursor.peek_slice(4);
            if (raw == nullptr) return std::nullopt;
            crc32 = static_cast<uint32_t>(static_cast<uint32_t>(raw[0] | (static_cast<uint32_t>(raw[1]) << 8) | (static_cast<uint32_t>(raw[2]) << 16) | (static_cast<uint32_t>(raw[3]) << 24)));
            if (!cursor.advance(4)) return std::nullopt;
        }
        return CodecFixedAfterLengthref{
            .header = header,
            .payload_len = payload_len,
            .payload = payload,
            .crc32 = crc32,
        };
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 1508;

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
        if (auto _e = w.write_u8(static_cast<uint8_t>(payload_len & 0xFF)); _e) return _e;
        if (auto _e = w.write_u8(static_cast<uint8_t>((payload_len >> 8) & 0xFF)); _e) return _e;
        if (auto _e = w.write_bytes(payload.data(), payload.size()); _e) return _e;
        if (auto _e = w.write_u8(static_cast<uint8_t>(crc32 & 0xFF)); _e) return _e;
        if (auto _e = w.write_u8(static_cast<uint8_t>((crc32 >> 8) & 0xFF)); _e) return _e;
        if (auto _e = w.write_u8(static_cast<uint8_t>((crc32 >> 16) & 0xFF)); _e) return _e;
        if (auto _e = w.write_u8(static_cast<uint8_t>((crc32 >> 24) & 0xFF)); _e) return _e;
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

}  // namespace SCE::Generated::CodecFixedAfterLengthref

#endif  // SCE_FORGE_CODEC_FIXED_AFTER_LENGTHREF_H
