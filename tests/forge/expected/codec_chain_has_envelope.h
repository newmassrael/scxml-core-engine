// SCE-MAP: codec_chain_has_envelope:23 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_CHAIN_HAS_ENVELOPE_H
#define SCE_FORGE_CODEC_CHAIN_HAS_ENVELOPE_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"
#include "codec_zenoh_ext_entry.h"
#include "codec_chain_has_slice.h"

namespace SCE::Generated::CodecChainHasEnvelope {

struct CodecChainHasEnvelope {
    uint8_t header;
    std::optional<std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>> extensions;
    std::optional<uint64_t> payload_len;
    std::optional<std::vector<uint8_t>> payload;
    std::optional<uint32_t> slice_count;
    std::optional<std::vector<::SCE::Generated::CodecChainHasSlice::CodecChainHasSlice>> slices;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecChainHasEnvelope> decode(::SCE::Forge::SceCursor& cursor) {
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
        uint8_t header;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            header = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>> extensions;
        if ((header & 0x80) != 0) {
            std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry> _list;
            _list.reserve(4);
            bool _more = false;
            for (std::size_t _i = 0; _i < 4; ++_i) {
                if (cursor.remaining() == 0) break;
                auto _elem = ::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry::decode(cursor);
                if (!_elem.has_value()) return std::nullopt;
                _more = _elem->z();
                _list.push_back(*_elem);
                if (!_more) break;
            }
            if (_more) return std::nullopt;
            extensions = std::move(_list);
        }
        bool _has_extensions_2 = false;
        if (extensions.has_value()) {
            for (const auto& _e : *extensions) {
                if (static_cast<std::uint64_t>(_e.ext_id()) == 2ULL) {
                    _has_extensions_2 = true;
                    break;
                }
            }
        }
        std::optional<uint64_t> payload_len;
        if (!_has_extensions_2) {
            auto _v_opt = cursor.read_vle_u64();
        if (!_v_opt.has_value()) return std::nullopt;
        auto _v = static_cast<std::uint64_t>(*_v_opt);
            payload_len = _v;
        }
        std::optional<std::vector<uint8_t>> payload;
        if (!_has_extensions_2) {
            std::size_t _n = static_cast<std::size_t>(payload_len.value());
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            payload.emplace(raw, raw + _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        std::optional<uint32_t> slice_count;
        if (_has_extensions_2) {
            auto _v_opt = cursor.read_vle_u32();
        if (!_v_opt.has_value()) return std::nullopt;
        auto _v = static_cast<std::uint32_t>(*_v_opt);
            slice_count = _v;
        }
        std::optional<std::vector<::SCE::Generated::CodecChainHasSlice::CodecChainHasSlice>> slices;
        if (_has_extensions_2) {
            auto _n = slice_count.value();
            std::vector<::SCE::Generated::CodecChainHasSlice::CodecChainHasSlice> _list;
            _list.reserve(_n);
            for (auto _i = decltype(_n){0}; _i < _n; ++_i) {
                auto _elem = ::SCE::Generated::CodecChainHasSlice::CodecChainHasSlice::decode(cursor);
                if (!_elem.has_value()) return std::nullopt;
                _list.push_back(*_elem);
            }
            slices = std::move(_list);
        }
        return CodecChainHasEnvelope{
            .header = header,
            .extensions = extensions,
            .payload_len = payload_len,
            .payload = payload,
            .slice_count = slice_count,
            .slices = slices,
        };
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    uint8_t kind() const noexcept {
        return static_cast<uint8_t>(
            (this->header >> 0) & static_cast<uint8_t>(0x1F)
        );
    }

    void set_kind(uint8_t v) noexcept {
        const uint8_t _shifted_mask =
            static_cast<uint8_t>(
                static_cast<uint8_t>(0x1F) << 0
            );
        const uint8_t _val =
            static_cast<uint8_t>(
                (static_cast<uint8_t>(v) & static_cast<uint8_t>(0x1F)) << 0
            );
        this->header = static_cast<uint8_t>(
            (this->header & static_cast<uint8_t>(~_shifted_mask)) | _val
        );
    }

    bool e() const noexcept {
        return (this->header & 0x80) != 0;
    }

    void set_e(bool v) noexcept {
        if (v) {
            this->header = static_cast<uint8_t>(this->header | 0x80);
        } else {
            this->header = static_cast<uint8_t>(this->header & static_cast<uint8_t>(~0x80));
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 1511;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable sinks
    /// (e.g. `VectorSink`) are effectively infallible.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w) const noexcept {
        bool _has_extensions_2 = false;
        if (extensions.has_value()) {
            for (const auto& _e : *extensions) {
                if (static_cast<std::uint64_t>(_e.ext_id()) == 2ULL) {
                    _has_extensions_2 = true;
                    break;
                }
            }
        }
        if (!_has_extensions_2) {
            if (!payload_len.has_value()) return ::SCE::Forge::CodecError::PresentIfMismatch;
        } else if (payload_len.has_value()) return ::SCE::Forge::CodecError::PresentIfMismatch;
        if (!_has_extensions_2) {
            if (!payload.has_value()) return ::SCE::Forge::CodecError::PresentIfMismatch;
        } else if (payload.has_value()) return ::SCE::Forge::CodecError::PresentIfMismatch;
        if (_has_extensions_2) {
            if (!slice_count.has_value()) return ::SCE::Forge::CodecError::PresentIfMismatch;
        } else if (slice_count.has_value()) return ::SCE::Forge::CodecError::PresentIfMismatch;
        if (_has_extensions_2) {
            if (!slices.has_value()) return ::SCE::Forge::CodecError::PresentIfMismatch;
        } else if (slices.has_value()) return ::SCE::Forge::CodecError::PresentIfMismatch;
        // Streaming cursor encode (SSOT selection: `needs_streaming`).
        // Mirrors the streaming decode: every field appends its own bytes
        // in declaration order through the per-field encode blocks, so a
        // gated field skips its append when absent, and a fixed field after
        // a variable-length payload lands after the payload (the positional
        // path appends variable fields last, placing it ahead on the wire).
        // Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
        // dedicated helpers; everything else uses `present_if_encode_block`.
        if (auto _e = w.write_u8(header); _e) return _e;
        if (this->extensions.has_value()) {
            for (const auto& _e : *this->extensions) {
                if (auto _se = _e.encode(w); _se) return _se;
            }
        }
        if (payload_len.has_value()) {
            auto _v = *payload_len;
        if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(_v)); _e) return _e;
        }
        if (payload.has_value()) {
            if (auto _e = w.write_bytes(payload->data(), payload->size()); _e) return _e;
        }
        if (slice_count.has_value()) {
            auto _v = *slice_count;
        if (auto _e = w.write_vle_u32(static_cast<std::uint32_t>(_v)); _e) return _e;
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
    /// freshly-encoded byte vector, or `std::nullopt` when `encode`
    /// refuses a message whose chain and a gated field contradict each
    /// other (`CodecError::PresentIfMismatch`). Callers targeting
    /// zero-alloc hot paths should call `encode` directly against a
    /// caller-owned sink.
    [[nodiscard]] std::optional<std::vector<std::uint8_t>> encode_to_vec() const {
        std::vector<std::uint8_t> _sce_v;
        _sce_v.reserve(MAX_ENCODED_BYTES);
        ::SCE::Forge::VectorSink _sce_sink(_sce_v);
        if (encode(_sce_sink)) {
            return std::nullopt;
        }
        return _sce_v;
    }
};

}  // namespace SCE::Generated::CodecChainHasEnvelope

#endif  // SCE_FORGE_CODEC_CHAIN_HAS_ENVELOPE_H
