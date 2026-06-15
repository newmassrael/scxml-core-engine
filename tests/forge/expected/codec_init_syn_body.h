// SCE-MAP: codec_init_syn_body:30

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_INIT_SYN_BODY_H
#define SCE_FORGE_CODEC_INIT_SYN_BODY_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecInitSynBody {

struct CodecInitSynBody {
    uint8_t version;
    std::optional<uint8_t> sn_res;
    std::optional<uint16_t> batch_size;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecInitSynBody> decode(::SCE::Forge::SceCursor& cursor, std::uint8_t s) {
        // Declared-but-unconsumed flag inputs: defensive (void) suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly under -Wunused.
        (void)s;
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
        uint8_t version;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            version = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<uint8_t> sn_res;
        if ((s & 0x01) != 0) {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            sn_res = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<uint16_t> batch_size;
        if ((s & 0x01) != 0) {
            const std::uint8_t* raw = cursor.peek_slice(2);
            if (raw == nullptr) return std::nullopt;
            batch_size = static_cast<uint16_t>(static_cast<uint16_t>((static_cast<uint16_t>(raw[0]) << 8) | raw[1]));
            if (!cursor.advance(2)) return std::nullopt;
        }
        return CodecInitSynBody{
            .version = version,
            .sn_res = sn_res,
            .batch_size = batch_size,
        };
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 4;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable sinks
    /// (e.g. `VectorSink`) are effectively infallible.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w, std::uint8_t s) const noexcept {
        (void)s;
        // Streaming cursor encode (SSOT selection: `needs_streaming`).
        // Mirrors the streaming decode: every field appends its own bytes
        // in declaration order through the per-field encode blocks, so a
        // gated field skips its append when absent, and a fixed field after
        // a variable-length payload lands after the payload (the positional
        // path appends variable fields last, placing it ahead on the wire).
        // Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
        // dedicated helpers; everything else uses `present_if_encode_block`.
        if (auto _e = w.write_u8(version); _e) return _e;
        if (sn_res.has_value()) {
            auto _v = *sn_res;
            if (auto _e = w.write_u8(_v); _e) return _e;
        }
        if (batch_size.has_value()) {
            auto _v = *batch_size;
            if (auto _e = w.write_u8(static_cast<uint8_t>((_v >> 8) & 0xFF)); _e) return _e;
            if (auto _e = w.write_u8(static_cast<uint8_t>(_v & 0xFF)); _e) return _e;
        }
        return std::nullopt;
    }

    /// Heap-backed convenience facade. Pre-reserves `MAX_ENCODED_BYTES`
    /// so the worst-case write path performs at most one allocation,
    /// then delegates to `encode` over a `VectorSink`. Returns the
    /// freshly-encoded byte vector. Callers targeting zero-alloc hot
    /// paths should call `encode` directly against a caller-owned sink.
    [[nodiscard]] std::vector<std::uint8_t> encode_to_vec(std::uint8_t s) const {
        std::vector<std::uint8_t> _sce_v;
        _sce_v.reserve(MAX_ENCODED_BYTES);
        ::SCE::Forge::VectorSink _sce_sink(_sce_v);
        (void)encode(_sce_sink, s);
        return _sce_v;
    }
};

}  // namespace SCE::Generated::CodecInitSynBody

#endif  // SCE_FORGE_CODEC_INIT_SYN_BODY_H
