// SCE-MAP: codec_zenoh_query_value:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_QUERY_VALUE_H
#define SCE_FORGE_CODEC_ZENOH_QUERY_VALUE_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"
#include "codec_zenoh_value_entry.h"

namespace SCE::Generated::CodecZenohQueryValue {

struct CodecZenohQueryValue {
    std::vector<::SCE::Generated::CodecZenohValueEntry::CodecZenohValueEntry> exts;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohQueryValue> decode(::SCE::Forge::SceCursor& cursor) {
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
        std::vector<::SCE::Generated::CodecZenohValueEntry::CodecZenohValueEntry> exts;
        exts.reserve(4);
        std::uint8_t _prev_exts_after_shm = 0;
        bool _more = false;
        for (std::size_t _i = 0; _i < 4; ++_i) {
            if (cursor.remaining() == 0) break;
            auto _elem = ::SCE::Generated::CodecZenohValueEntry::CodecZenohValueEntry::decode(cursor, _prev_exts_after_shm);
            if (!_elem.has_value()) return std::nullopt;
            _more = _elem->z();
            _prev_exts_after_shm = static_cast<std::uint8_t>(static_cast<std::uint64_t>((*_elem).ext_id()) == 4ULL);
            exts.push_back(*_elem);
            if (!_more) break;
        }
        if (_more) return std::nullopt;
        return CodecZenohQueryValue{
            .exts = exts,
        };
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 656;

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
        std::uint8_t _prev_exts_after_shm = 0;
        for (const auto& _e : exts) {
            if (auto _se = _e.encode(w, _prev_exts_after_shm); _se) return _se;
            _prev_exts_after_shm = static_cast<std::uint8_t>(static_cast<std::uint64_t>(_e.ext_id()) == 4ULL);
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

}  // namespace SCE::Generated::CodecZenohQueryValue

#endif  // SCE_FORGE_CODEC_ZENOH_QUERY_VALUE_H
