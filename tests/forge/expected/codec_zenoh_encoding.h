// SCE-MAP: codec_zenoh_encoding:68

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_ENCODING_H
#define SCE_FORGE_CODEC_ZENOH_ENCODING_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <string>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecZenohEncoding {

struct CodecZenohEncoding {
    uint32_t packed_id;
    std::optional<uint64_t> schema_len;
    std::optional<std::string> schema;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohEncoding> decode(::SCE::Forge::SceCursor& cursor) {
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
        auto packed_id_opt = cursor.read_vle_u32();
        if (!packed_id_opt.has_value()) return std::nullopt;
        auto packed_id = static_cast<std::uint32_t>(*packed_id_opt);
        std::optional<uint64_t> schema_len;
        if ((packed_id & 0x00000001) != 0) {
            auto _v_opt = cursor.read_vle_u64();
        if (!_v_opt.has_value()) return std::nullopt;
        auto _v = static_cast<std::uint64_t>(*_v_opt);
            schema_len = _v;
        }
        std::optional<std::string> schema;
        if ((packed_id & 0x00000001) != 0) {
            std::size_t _n = static_cast<std::size_t>(schema_len.value());
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            if (!::SCE::Forge::is_valid_utf8(raw, _n)) return std::nullopt;
            schema.emplace(reinterpret_cast<const char*>(raw), _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        return CodecZenohEncoding{
            .packed_id = packed_id,
            .schema_len = schema_len,
            .schema = schema,
        };
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool has_schema() const noexcept {
        return (this->packed_id & 0x00000001) != 0;
    }

    void set_has_schema(bool v) noexcept {
        if (v) {
            this->packed_id = static_cast<uint32_t>(this->packed_id | 0x00000001);
        } else {
            this->packed_id = static_cast<uint32_t>(this->packed_id & static_cast<uint32_t>(~0x00000001));
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 142;

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
        {
            std::uint64_t _w = static_cast<std::uint64_t>(packed_id);
            std::uint32_t _vn = 0;
            while (_w >= 0x80 && _vn < 4) {
                if (auto _e = w.write_u8(static_cast<std::uint8_t>((_w & 0x7F) | 0x80)); _e) return _e;
                _w >>= 7;
                ++_vn;
            }
            if (auto _e = w.write_u8(static_cast<std::uint8_t>(_w)); _e) return _e;
        }
        if (schema_len.has_value()) {
            auto _v = *schema_len;
        {
            std::uint64_t _w = static_cast<std::uint64_t>(_v);
            std::uint32_t _vn = 0;
            while (_w >= 0x80 && _vn < 8) {
                if (auto _e = w.write_u8(static_cast<std::uint8_t>((_w & 0x7F) | 0x80)); _e) return _e;
                _w >>= 7;
                ++_vn;
            }
            if (auto _e = w.write_u8(static_cast<std::uint8_t>(_w)); _e) return _e;
        }
        }
        if (schema.has_value()) {
            if (auto _e = w.write_bytes(
                reinterpret_cast<const std::uint8_t*>(schema->data()), schema->size()); _e) return _e;
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

}  // namespace SCE::Generated::CodecZenohEncoding

#endif  // SCE_FORGE_CODEC_ZENOH_ENCODING_H
