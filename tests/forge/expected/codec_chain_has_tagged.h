// SCE-MAP: codec_chain_has_tagged:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_CHAIN_HAS_TAGGED_H
#define SCE_FORGE_CODEC_CHAIN_HAS_TAGGED_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"
#include "codec_chain_has_tagged_entry.h"

namespace SCE::Generated::CodecChainHasTagged {

struct CodecChainHasTagged {
    uint8_t header;
    std::vector<::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry> entries;
    std::optional<uint8_t> priority;
    std::optional<uint16_t> checksum;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecChainHasTagged> decode(::SCE::Forge::SceCursor& cursor) {
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
        std::vector<::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry> entries;
        entries.reserve(3);
        bool _more = false;
        for (std::size_t _i = 0; _i < 3; ++_i) {
            if (cursor.remaining() == 0) break;
            auto _elem = ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry::decode(cursor);
            if (!_elem.has_value()) return std::nullopt;
            _more = _elem->more();
            entries.push_back(*_elem);
            if (!_more) break;
        }
        if (_more) return std::nullopt;
        bool _has_entries_7 = false;
        for (const auto& _e : entries) {
            if (static_cast<std::uint64_t>(_e.entry_type) == 7ULL) {
                _has_entries_7 = true;
                break;
            }
        }
        bool _has_entries_9 = false;
        for (const auto& _e : entries) {
            if (static_cast<std::uint64_t>(_e.entry_type) == 9ULL) {
                _has_entries_9 = true;
                break;
            }
        }
        std::optional<uint8_t> priority;
        if (_has_entries_7 || (header & 0x01) != 0) {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            priority = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<uint16_t> checksum;
        if (!_has_entries_9) {
            const std::uint8_t* raw = cursor.peek_slice(2);
            if (raw == nullptr) return std::nullopt;
            checksum = static_cast<uint16_t>(static_cast<uint16_t>((static_cast<uint16_t>(raw[0]) << 8) | raw[1]));
            if (!cursor.advance(2)) return std::nullopt;
        }
        return CodecChainHasTagged{
            .header = header,
            .entries = entries,
            .priority = priority,
            .checksum = checksum,
        };
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool wide() const noexcept {
        return (this->header & 0x01) != 0;
    }

    void set_wide(bool v) noexcept {
        if (v) {
            this->header = static_cast<uint8_t>(this->header | 0x01);
        } else {
            this->header = static_cast<uint8_t>(this->header & static_cast<uint8_t>(~0x01));
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 14;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable sinks
    /// (e.g. `VectorSink`) are effectively infallible.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w) const noexcept {
        bool _has_entries_7 = false;
        for (const auto& _e : entries) {
            if (static_cast<std::uint64_t>(_e.entry_type) == 7ULL) {
                _has_entries_7 = true;
                break;
            }
        }
        bool _has_entries_9 = false;
        for (const auto& _e : entries) {
            if (static_cast<std::uint64_t>(_e.entry_type) == 9ULL) {
                _has_entries_9 = true;
                break;
            }
        }
        if (_has_entries_7 || (header & 0x01) != 0) {
            if (!priority.has_value()) return ::SCE::Forge::CodecError::PresentIfMismatch;
        } else if (priority.has_value()) return ::SCE::Forge::CodecError::PresentIfMismatch;
        if (!_has_entries_9) {
            if (!checksum.has_value()) return ::SCE::Forge::CodecError::PresentIfMismatch;
        } else if (checksum.has_value()) return ::SCE::Forge::CodecError::PresentIfMismatch;
        // Streaming cursor encode (SSOT selection: `needs_streaming`).
        // Mirrors the streaming decode: every field appends its own bytes
        // in declaration order through the per-field encode blocks, so a
        // gated field skips its append when absent, and a fixed field after
        // a variable-length payload lands after the payload (the positional
        // path appends variable fields last, placing it ahead on the wire).
        // Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
        // dedicated helpers; everything else uses `present_if_encode_block`.
        if (auto _e = w.write_u8(header); _e) return _e;
        for (const auto& _e : entries) {
            if (auto _se = _e.encode(w); _se) return _se;
        }
        if (priority.has_value()) {
            auto _v = *priority;
            if (auto _e = w.write_u8(_v); _e) return _e;
        }
        if (checksum.has_value()) {
            auto _v = *checksum;
            if (auto _e = w.write_u8(static_cast<uint8_t>((_v >> 8) & 0xFF)); _e) return _e;
            if (auto _e = w.write_u8(static_cast<uint8_t>(_v & 0xFF)); _e) return _e;
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

}  // namespace SCE::Generated::CodecChainHasTagged

#endif  // SCE_FORGE_CODEC_CHAIN_HAS_TAGGED_H
