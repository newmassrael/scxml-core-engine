// SCE-MAP: codec_chain_has_tagged_entry:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_CHAIN_HAS_TAGGED_ENTRY_H
#define SCE_FORGE_CODEC_CHAIN_HAS_TAGGED_ENTRY_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecChainHasTaggedEntry {

struct CodecChainHasTaggedEntry {
    uint8_t entry_type;
    uint8_t ctl;
    uint8_t body;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecChainHasTaggedEntry> decode(::SCE::Forge::SceCursor& cursor) {
        const std::uint8_t* raw = cursor.peek_slice(3);
        if (raw == nullptr) return std::nullopt;
        uint8_t entry_type = raw[0];
        uint8_t ctl = raw[1];
        uint8_t body = raw[2];
        CodecChainHasTaggedEntry _decoded{
            .entry_type = entry_type,
            .ctl = ctl,
            .body = body,
        };
        if (!cursor.advance(3)) return std::nullopt;
        return _decoded;
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool more() const noexcept {
        return (this->ctl & 0x01) != 0;
    }

    void set_more(bool v) noexcept {
        if (v) {
            this->ctl = static_cast<uint8_t>(this->ctl | 0x01);
        } else {
            this->ctl = static_cast<uint8_t>(this->ctl & static_cast<uint8_t>(~0x01));
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 3;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable sinks
    /// (e.g. `VectorSink`) are effectively infallible.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w) const noexcept {
        if (auto _e = w.write_u8(entry_type); _e) return _e;
        if (auto _e = w.write_u8(ctl); _e) return _e;
        if (auto _e = w.write_u8(body); _e) return _e;
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

}  // namespace SCE::Generated::CodecChainHasTaggedEntry

#endif  // SCE_FORGE_CODEC_CHAIN_HAS_TAGGED_ENTRY_H
