// SCE-MAP: codec_chain_prev_carrier:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_CHAIN_PREV_CARRIER_H
#define SCE_FORGE_CODEC_CHAIN_PREV_CARRIER_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"
#include "codec_chain_prev_entry.h"

namespace SCE::Generated::CodecChainPrevCarrier {

struct CodecChainPrevCarrier {
    uint8_t head;
    std::vector<::SCE::Generated::CodecChainPrevEntry::CodecChainPrevEntry> entries;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecChainPrevCarrier> decode(::SCE::Forge::SceCursor& cursor) {
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
        uint8_t head;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            head = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::vector<::SCE::Generated::CodecChainPrevEntry::CodecChainPrevEntry> entries;
        entries.reserve(4);
        std::uint8_t _prev_entries_after_marker = 0;
        bool _more = false;
        for (std::size_t _i = 0; _i < 4; ++_i) {
            if (cursor.remaining() == 0) break;
            auto _elem = ::SCE::Generated::CodecChainPrevEntry::CodecChainPrevEntry::decode(cursor, _prev_entries_after_marker, static_cast<std::uint8_t>((head >> 0) & 0x1));
            if (!_elem.has_value()) return std::nullopt;
            _more = _elem->z();
            _prev_entries_after_marker = static_cast<std::uint8_t>(static_cast<std::uint64_t>((*_elem).ext_id()) == 4ULL);
            entries.push_back(*_elem);
            if (!_more) break;
        }
        if (_more) return std::nullopt;
        return CodecChainPrevCarrier{
            .head = head,
            .entries = entries,
        };
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool t() const noexcept {
        return (this->head & 0x01) != 0;
    }

    void set_t(bool v) noexcept {
        if (v) {
            this->head = static_cast<uint8_t>(this->head | 0x01);
        } else {
            this->head = static_cast<uint8_t>(this->head & static_cast<uint8_t>(~0x01));
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 21;

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
        if (auto _e = w.write_u8(head); _e) return _e;
        std::uint8_t _prev_entries_after_marker = 0;
        for (const auto& _e : entries) {
            if (auto _se = _e.encode(w, _prev_entries_after_marker, static_cast<std::uint8_t>((this->head >> 0) & 0x1)); _se) return _se;
            _prev_entries_after_marker = static_cast<std::uint8_t>(static_cast<std::uint64_t>(_e.ext_id()) == 4ULL);
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

}  // namespace SCE::Generated::CodecChainPrevCarrier

#endif  // SCE_FORGE_CODEC_CHAIN_PREV_CARRIER_H
