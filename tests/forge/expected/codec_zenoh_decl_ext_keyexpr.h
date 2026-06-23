// SCE-MAP: codec_zenoh_decl_ext_keyexpr:89

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_DECL_EXT_KEYEXPR_H
#define SCE_FORGE_CODEC_ZENOH_DECL_EXT_KEYEXPR_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"
#include "codec_zenoh_decl_ext_keyexpr_inner.h"

namespace SCE::Generated::CodecZenohDeclExtKeyexpr {

struct CodecZenohDeclExtKeyexpr {
    uint8_t outer_header;
    uint64_t total_length;
    ::SCE::Generated::CodecZenohDeclExtKeyexprInner::CodecZenohDeclExtKeyexprInner inner;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohDeclExtKeyexpr> decode(::SCE::Forge::SceCursor& cursor) {
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
        uint8_t outer_header;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            outer_header = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        auto total_length_opt = cursor.read_vle_u64();
        if (!total_length_opt.has_value()) return std::nullopt;
        auto total_length = static_cast<std::uint64_t>(*total_length_opt);
        ::SCE::Generated::CodecZenohDeclExtKeyexprInner::CodecZenohDeclExtKeyexprInner inner;
        {
            std::size_t _len = static_cast<std::size_t>(total_length);
            const std::uint8_t* _raw = cursor.peek_slice(_len);
            if (_raw == nullptr) return std::nullopt;
            ::SCE::Forge::SceCursor _inner(_raw, _len);
            auto _emb = ::SCE::Generated::CodecZenohDeclExtKeyexprInner::CodecZenohDeclExtKeyexprInner::decode(_inner);
            if (!_emb.has_value()) return std::nullopt;
            if (!cursor.advance(_len)) return std::nullopt;
            inner = std::move(*_emb);
        }
        return CodecZenohDeclExtKeyexpr{
            .outer_header = outer_header,
            .total_length = total_length,
            .inner = inner,
        };
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    uint8_t ext_id() const noexcept {
        return static_cast<uint8_t>(
            (this->outer_header >> 0) & static_cast<uint8_t>(0x0F)
        );
    }

    void set_ext_id(uint8_t v) noexcept {
        const uint8_t _shifted_mask =
            static_cast<uint8_t>(
                static_cast<uint8_t>(0x0F) << 0
            );
        const uint8_t _val =
            static_cast<uint8_t>(
                (static_cast<uint8_t>(v) & static_cast<uint8_t>(0x0F)) << 0
            );
        this->outer_header = static_cast<uint8_t>(
            (this->outer_header & static_cast<uint8_t>(~_shifted_mask)) | _val
        );
    }

    bool m() const noexcept {
        return (this->outer_header & 0x10) != 0;
    }

    void set_m(bool v) noexcept {
        if (v) {
            this->outer_header = static_cast<uint8_t>(this->outer_header | 0x10);
        } else {
            this->outer_header = static_cast<uint8_t>(this->outer_header & static_cast<uint8_t>(~0x10));
        }
    }

    uint8_t enc() const noexcept {
        return static_cast<uint8_t>(
            (this->outer_header >> 5) & static_cast<uint8_t>(0x03)
        );
    }

    void set_enc(uint8_t v) noexcept {
        const uint8_t _shifted_mask =
            static_cast<uint8_t>(
                static_cast<uint8_t>(0x03) << 5
            );
        const uint8_t _val =
            static_cast<uint8_t>(
                (static_cast<uint8_t>(v) & static_cast<uint8_t>(0x03)) << 5
            );
        this->outer_header = static_cast<uint8_t>(
            (this->outer_header & static_cast<uint8_t>(~_shifted_mask)) | _val
        );
    }

    bool z() const noexcept {
        return (this->outer_header & 0x80) != 0;
    }

    void set_z(bool v) noexcept {
        if (v) {
            this->outer_header = static_cast<uint8_t>(this->outer_header | 0x80);
        } else {
            this->outer_header = static_cast<uint8_t>(this->outer_header & static_cast<uint8_t>(~0x80));
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 266;

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
        if (auto _e = w.write_u8(outer_header); _e) return _e;
        if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(total_length)); _e) return _e;
        if (auto _e = inner.encode(w); _e) return _e;
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

}  // namespace SCE::Generated::CodecZenohDeclExtKeyexpr

#endif  // SCE_FORGE_CODEC_ZENOH_DECL_EXT_KEYEXPR_H
