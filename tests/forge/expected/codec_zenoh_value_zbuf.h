// SCE-MAP: codec_zenoh_value_zbuf:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_VALUE_ZBUF_H
#define SCE_FORGE_CODEC_ZENOH_VALUE_ZBUF_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"
#include "codec_zenoh_value_slice.h"

namespace SCE::Generated::CodecZenohValueZbuf {

struct CodecZenohValueZbuf {
    uint64_t value_len;
    std::optional<std::vector<uint8_t>> value;
    std::optional<uint64_t> encoding;
    std::optional<uint64_t> slice_count;
    std::optional<std::vector<::SCE::Generated::CodecZenohValueSlice::CodecZenohValueSlice>> slices;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohValueZbuf> decode(::SCE::Forge::SceCursor& cursor, std::uint8_t after_shm) {
        // Declared-but-unconsumed flag inputs: defensive (void) suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly under -Wunused.
        (void)after_shm;
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
        auto value_len_opt = cursor.read_vle_u64();
        if (!value_len_opt.has_value()) return std::nullopt;
        auto value_len = static_cast<std::uint64_t>(*value_len_opt);
        std::optional<std::vector<uint8_t>> value;
        if ((after_shm & 0x01) == 0) {
            std::size_t _n = static_cast<std::size_t>(value_len);
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            value.emplace(raw, raw + _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        std::optional<uint64_t> encoding;
        if ((after_shm & 0x01) != 0) {
            auto _v_opt = cursor.read_vle_u64();
        if (!_v_opt.has_value()) return std::nullopt;
        auto _v = static_cast<std::uint64_t>(*_v_opt);
            encoding = _v;
        }
        std::optional<uint64_t> slice_count;
        if ((after_shm & 0x01) != 0) {
            auto _v_opt = cursor.read_vle_u64();
        if (!_v_opt.has_value()) return std::nullopt;
        auto _v = static_cast<std::uint64_t>(*_v_opt);
            slice_count = _v;
        }
        std::optional<std::vector<::SCE::Generated::CodecZenohValueSlice::CodecZenohValueSlice>> slices;
        if ((after_shm & 0x01) != 0) {
            auto _n = slice_count.value();
            std::vector<::SCE::Generated::CodecZenohValueSlice::CodecZenohValueSlice> _list;
            _list.reserve(_n);
            for (auto _i = decltype(_n){0}; _i < _n; ++_i) {
                auto _elem = ::SCE::Generated::CodecZenohValueSlice::CodecZenohValueSlice::decode(cursor);
                if (!_elem.has_value()) return std::nullopt;
                _list.push_back(*_elem);
            }
            slices = std::move(_list);
        }
        return CodecZenohValueZbuf{
            .value_len = value_len,
            .value = value,
            .encoding = encoding,
            .slice_count = slice_count,
            .slices = slices,
        };
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 163;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable sinks
    /// (e.g. `VectorSink`) are effectively infallible.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w, std::uint8_t after_shm) const noexcept {
        (void)after_shm;
        // Streaming cursor encode (SSOT selection: `needs_streaming`).
        // Mirrors the streaming decode: every field appends its own bytes
        // in declaration order through the per-field encode blocks, so a
        // gated field skips its append when absent, and a fixed field after
        // a variable-length payload lands after the payload (the positional
        // path appends variable fields last, placing it ahead on the wire).
        // Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
        // dedicated helpers; everything else uses `present_if_encode_block`.
        if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(value_len)); _e) return _e;
        if (value.has_value()) {
            if (auto _e = w.write_bytes(value->data(), value->size()); _e) return _e;
        }
        if (encoding.has_value()) {
            auto _v = *encoding;
        if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(_v)); _e) return _e;
        }
        if (slice_count.has_value()) {
            auto _v = *slice_count;
        if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(_v)); _e) return _e;
        }
        if (this->slices.has_value()) {
            for (const auto& _e : *this->slices) {
                if (auto _se = _e.encode(w); _se) return _se;
            }
        }
        return std::nullopt;
    }

    /// Heap-backed convenience facade. Pre-reserves `MAX_ENCODED_BYTES`
    /// so the worst-case write path performs at most one allocation,
    /// then delegates to `encode` over a `VectorSink`. Returns the
    /// freshly-encoded byte vector. Callers targeting zero-alloc hot
    /// paths should call `encode` directly against a caller-owned sink.
    [[nodiscard]] std::vector<std::uint8_t> encode_to_vec(std::uint8_t after_shm) const {
        std::vector<std::uint8_t> _sce_v;
        _sce_v.reserve(MAX_ENCODED_BYTES);
        ::SCE::Forge::VectorSink _sce_sink(_sce_v);
        (void)encode(_sce_sink, after_shm);
        return _sce_v;
    }
};

}  // namespace SCE::Generated::CodecZenohValueZbuf

#endif  // SCE_FORGE_CODEC_ZENOH_VALUE_ZBUF_H
