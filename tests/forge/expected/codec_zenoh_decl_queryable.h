// SCE-MAP: codec_zenoh_decl_queryable:46

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_DECL_QUERYABLE_H
#define SCE_FORGE_CODEC_ZENOH_DECL_QUERYABLE_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"
#include "codec_zenoh_wireexpr.h"

namespace SCE::Generated::CodecZenohDeclQueryable {

struct CodecZenohDeclQueryable {
    uint32_t id;
    ::SCE::Generated::CodecZenohWireexpr::CodecZenohWireexpr wireexpr;
    std::optional<uint8_t> ext_type;
    std::optional<uint64_t> ext_value;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohDeclQueryable> decode(::SCE::Forge::SceCursor& cursor, std::uint8_t n, std::uint8_t z) {
        // Declared-but-unconsumed flag inputs: defensive (void) suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly under -Wunused.
        (void)n;
        (void)z;
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
        auto id_opt = cursor.read_vle_u32();
        if (!id_opt.has_value()) return std::nullopt;
        auto id = static_cast<std::uint32_t>(*id_opt);
        auto _emb_wireexpr = ::SCE::Generated::CodecZenohWireexpr::CodecZenohWireexpr::decode(cursor, n);
        if (!_emb_wireexpr.has_value()) return std::nullopt;
        auto wireexpr = std::move(*_emb_wireexpr);
        std::optional<uint8_t> ext_type;
        if ((z & 0x01) != 0) {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            ext_type = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<uint64_t> ext_value;
        if ((z & 0x01) != 0) {
            auto _v_opt = cursor.read_vle_u64();
        if (!_v_opt.has_value()) return std::nullopt;
        auto _v = static_cast<std::uint64_t>(*_v_opt);
            ext_value = _v;
        }
        return CodecZenohDeclQueryable{
            .id = id,
            .wireexpr = wireexpr,
            .ext_type = ext_type,
            .ext_value = ext_value,
        };
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 273;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable sinks
    /// (e.g. `VectorSink`) are effectively infallible.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w, std::uint8_t n, std::uint8_t z) const noexcept {
        (void)n;
        (void)z;
        // Streaming cursor encode (SSOT selection: `needs_streaming`).
        // Mirrors the streaming decode: every field appends its own bytes
        // in declaration order through the per-field encode blocks, so a
        // gated field skips its append when absent, and a fixed field after
        // a variable-length payload lands after the payload (the positional
        // path appends variable fields last, placing it ahead on the wire).
        // Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
        // dedicated helpers; everything else uses `present_if_encode_block`.
        {
            std::uint64_t _w = static_cast<std::uint64_t>(id);
            std::uint32_t _vn = 0;
            while (_w >= 0x80 && _vn < 4) {
                if (auto _e = w.write_u8(static_cast<std::uint8_t>((_w & 0x7F) | 0x80)); _e) return _e;
                _w >>= 7;
                ++_vn;
            }
            if (auto _e = w.write_u8(static_cast<std::uint8_t>(_w)); _e) return _e;
        }
        if (auto _e = wireexpr.encode(w, n); _e) return _e;
        if (ext_type.has_value()) {
            auto _v = *ext_type;
            if (auto _e = w.write_u8(_v); _e) return _e;
        }
        if (ext_value.has_value()) {
            auto _v = *ext_value;
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
        return std::nullopt;
    }

    /// Heap-backed convenience facade. Pre-reserves `MAX_ENCODED_BYTES`
    /// so the worst-case write path performs at most one allocation,
    /// then delegates to `encode` over a `VectorSink`. Returns the
    /// freshly-encoded byte vector. Callers targeting zero-alloc hot
    /// paths should call `encode` directly against a caller-owned sink.
    [[nodiscard]] std::vector<std::uint8_t> encode_to_vec(std::uint8_t n, std::uint8_t z) const {
        std::vector<std::uint8_t> _sce_v;
        _sce_v.reserve(MAX_ENCODED_BYTES);
        ::SCE::Forge::VectorSink _sce_sink(_sce_v);
        (void)encode(_sce_sink, n, z);
        return _sce_v;
    }
};

}  // namespace SCE::Generated::CodecZenohDeclQueryable

#endif  // SCE_FORGE_CODEC_ZENOH_DECL_QUERYABLE_H
