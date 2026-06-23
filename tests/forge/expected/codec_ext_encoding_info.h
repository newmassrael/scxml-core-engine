// SCE-MAP: codec_ext_encoding_info:44

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_EXT_ENCODING_INFO_H
#define SCE_FORGE_CODEC_EXT_ENCODING_INFO_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecExtEncodingInfo {

struct CodecExtEncodingInfo {
    uint32_t combined_id;
    uint8_t schema_size;
    std::optional<std::vector<uint8_t>> schema;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecExtEncodingInfo> decode(::SCE::Forge::SceCursor& cursor) {
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
        auto combined_id_opt = cursor.read_vle_u32();
        if (!combined_id_opt.has_value()) return std::nullopt;
        auto combined_id = static_cast<std::uint32_t>(*combined_id_opt);
        uint8_t schema_size;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            schema_size = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<std::vector<uint8_t>> schema;
        if ((combined_id & 0x00000001) != 0) {
            std::size_t _n = static_cast<std::size_t>(schema_size);
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            schema.emplace(raw, raw + _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        return CodecExtEncodingInfo{
            .combined_id = combined_id,
            .schema_size = schema_size,
            .schema = schema,
        };
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool has_schema() const noexcept {
        return (this->combined_id & 0x00000001) != 0;
    }

    void set_has_schema(bool v) noexcept {
        if (v) {
            this->combined_id = static_cast<uint32_t>(this->combined_id | 0x00000001);
        } else {
            this->combined_id = static_cast<uint32_t>(this->combined_id & static_cast<uint32_t>(~0x00000001));
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 71;

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
        if (auto _e = w.write_vle_u32(static_cast<std::uint32_t>(combined_id)); _e) return _e;
        if (auto _e = w.write_u8(schema_size); _e) return _e;
        if (schema.has_value()) {
            if (auto _e = w.write_bytes(schema->data(), schema->size()); _e) return _e;
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

}  // namespace SCE::Generated::CodecExtEncodingInfo

#endif  // SCE_FORGE_CODEC_EXT_ENCODING_INFO_H
