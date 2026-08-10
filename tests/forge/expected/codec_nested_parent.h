// SCE-MAP: codec_nested_parent:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_NESTED_PARENT_H
#define SCE_FORGE_CODEC_NESTED_PARENT_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"
#include "codec_nested_body.h"

namespace SCE::Generated::CodecNestedParent {

struct CodecNestedParent {
    uint8_t hdr;
    uint8_t m;
    ::SCE::Generated::CodecNestedBody::CodecNestedBody required_body;
    std::optional<::SCE::Generated::CodecNestedBody::CodecNestedBody> optional_body;
    std::vector<::SCE::Generated::CodecNestedBody::CodecNestedBody> body_list;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecNestedParent> decode(::SCE::Forge::SceCursor& cursor) {
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
        uint8_t hdr;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            hdr = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        uint8_t m;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            m = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        auto _emb_required_body = ::SCE::Generated::CodecNestedBody::CodecNestedBody::decode(cursor);
        if (!_emb_required_body.has_value()) return std::nullopt;
        auto required_body = std::move(*_emb_required_body);
        std::optional<::SCE::Generated::CodecNestedBody::CodecNestedBody> optional_body;
        if ((hdr & 0x01) != 0) {
            auto _emb = ::SCE::Generated::CodecNestedBody::CodecNestedBody::decode(cursor);
            if (!_emb.has_value()) return std::nullopt;
            optional_body = std::move(*_emb);
        }
        std::vector<::SCE::Generated::CodecNestedBody::CodecNestedBody> body_list;
        body_list.reserve(m);
        for (auto _i = decltype(m){0}; _i < m; ++_i) {
            auto _elem = ::SCE::Generated::CodecNestedBody::CodecNestedBody::decode(cursor);
            if (!_elem.has_value()) return std::nullopt;
            body_list.push_back(*_elem);
        }
        return CodecNestedParent{
            .hdr = hdr,
            .m = m,
            .required_body = required_body,
            .optional_body = optional_body,
            .body_list = body_list,
        };
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool has_opt() const noexcept {
        return (this->hdr & 0x01) != 0;
    }

    void set_has_opt(bool v) noexcept {
        if (v) {
            this->hdr = static_cast<uint8_t>(this->hdr | 0x01);
        } else {
            this->hdr = static_cast<uint8_t>(this->hdr & static_cast<uint8_t>(~0x01));
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 2710;

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
        if (auto _e = w.write_u8(hdr); _e) return _e;
        if (auto _e = w.write_u8(m); _e) return _e;
        if (auto _e = required_body.encode(w); _e) return _e;
        if (this->optional_body.has_value()) {
            if (auto _e = this->optional_body->encode(w); _e) return _e;
        }
        for (const auto& _e : body_list) {
            if (auto _se = _e.encode(w); _se) return _se;
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

}  // namespace SCE::Generated::CodecNestedParent

#endif  // SCE_FORGE_CODEC_NESTED_PARENT_H
