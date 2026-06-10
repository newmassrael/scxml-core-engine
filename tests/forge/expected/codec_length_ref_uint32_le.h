// SCE-MAP: codec_length_ref_uint32_le:13

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_LENGTH_REF_UINT32_LE_H
#define SCE_FORGE_CODEC_LENGTH_REF_UINT32_LE_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecLengthRefUint32Le {

struct CodecLengthRefUint32Le {
    uint32_t payload_len;
    std::vector<uint8_t> payload;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecLengthRefUint32Le> decode(::SCE::Forge::SceCursor& cursor) {
        std::size_t _frame_len = cursor.remaining();
        if (_frame_len < 4) return std::nullopt;
        const std::uint8_t* raw = cursor.peek_slice(_frame_len);
        if (raw == nullptr) return std::nullopt;
        uint32_t payload_len = static_cast<uint32_t>(raw[0] | (static_cast<uint32_t>(raw[1]) << 8) | (static_cast<uint32_t>(raw[2]) << 16) | (static_cast<uint32_t>(raw[3]) << 24));
        std::vector<uint8_t> payload = std::vector<uint8_t>(raw + 4, raw + 4 + payload_len);
        CodecLengthRefUint32Le _decoded{
            .payload_len = payload_len,
            .payload = payload,
        };
        if (!cursor.advance(_frame_len)) return std::nullopt;
        return _decoded;
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 1028;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable sinks
    /// (e.g. `VectorSink`) are effectively infallible.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w) const noexcept {
        if (auto _e = w.write_u8(static_cast<uint8_t>(payload_len & 0xFF)); _e) return _e;
        if (auto _e = w.write_u8(static_cast<uint8_t>((payload_len >> 8) & 0xFF)); _e) return _e;
        if (auto _e = w.write_u8(static_cast<uint8_t>((payload_len >> 16) & 0xFF)); _e) return _e;
        if (auto _e = w.write_u8(static_cast<uint8_t>((payload_len >> 24) & 0xFF)); _e) return _e;
        if (auto _e = w.write_bytes(this->payload.data(), this->payload.size()); _e) return _e;
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

}  // namespace SCE::Generated::CodecLengthRefUint32Le

#endif  // SCE_FORGE_CODEC_LENGTH_REF_UINT32_LE_H
