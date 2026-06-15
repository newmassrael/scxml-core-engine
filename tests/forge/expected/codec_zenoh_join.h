// SCE-MAP: codec_zenoh_join:41

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_JOIN_H
#define SCE_FORGE_CODEC_ZENOH_JOIN_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecZenohJoin {

struct CodecZenohJoin {
    uint8_t version;
    uint8_t cbyte;
    std::vector<uint8_t> zid;
    std::optional<uint8_t> sn_res;
    std::optional<uint16_t> batch_size;
    uint64_t lease;
    uint64_t next_sn_reliable;
    uint64_t next_sn_best_effort;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohJoin> decode(::SCE::Forge::SceCursor& cursor, std::uint8_t s) {
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
        uint8_t cbyte;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            cbyte = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::vector<uint8_t> zid;
        {
            std::size_t _n = static_cast<std::size_t>((cbyte >> 4) & 0xF) + 1;
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            zid.assign(raw, raw + _n);
            if (!cursor.advance(_n)) return std::nullopt;
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
            batch_size = static_cast<uint16_t>(static_cast<uint16_t>(raw[0] | (static_cast<uint16_t>(raw[1]) << 8)));
            if (!cursor.advance(2)) return std::nullopt;
        }
        auto lease_opt = cursor.read_vle_u64();
        if (!lease_opt.has_value()) return std::nullopt;
        auto lease = static_cast<std::uint64_t>(*lease_opt);
        auto next_sn_reliable_opt = cursor.read_vle_u64();
        if (!next_sn_reliable_opt.has_value()) return std::nullopt;
        auto next_sn_reliable = static_cast<std::uint64_t>(*next_sn_reliable_opt);
        auto next_sn_best_effort_opt = cursor.read_vle_u64();
        if (!next_sn_best_effort_opt.has_value()) return std::nullopt;
        auto next_sn_best_effort = static_cast<std::uint64_t>(*next_sn_best_effort_opt);
        return CodecZenohJoin{
            .version = version,
            .cbyte = cbyte,
            .zid = zid,
            .sn_res = sn_res,
            .batch_size = batch_size,
            .lease = lease,
            .next_sn_reliable = next_sn_reliable,
            .next_sn_best_effort = next_sn_best_effort,
        };
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    uint8_t whatami() const noexcept {
        return static_cast<uint8_t>(
            (this->cbyte >> 0) & static_cast<uint8_t>(0x03)
        );
    }

    void set_whatami(uint8_t v) noexcept {
        const uint8_t _shifted_mask =
            static_cast<uint8_t>(
                static_cast<uint8_t>(0x03) << 0
            );
        const uint8_t _val =
            static_cast<uint8_t>(
                (static_cast<uint8_t>(v) & static_cast<uint8_t>(0x03)) << 0
            );
        this->cbyte = static_cast<uint8_t>(
            (this->cbyte & static_cast<uint8_t>(~_shifted_mask)) | _val
        );
    }

    uint8_t zid_len_m1() const noexcept {
        return static_cast<uint8_t>(
            (this->cbyte >> 4) & static_cast<uint8_t>(0x0F)
        );
    }

    void set_zid_len_m1(uint8_t v) noexcept {
        const uint8_t _shifted_mask =
            static_cast<uint8_t>(
                static_cast<uint8_t>(0x0F) << 4
            );
        const uint8_t _val =
            static_cast<uint8_t>(
                (static_cast<uint8_t>(v) & static_cast<uint8_t>(0x0F)) << 4
            );
        this->cbyte = static_cast<uint8_t>(
            (this->cbyte & static_cast<uint8_t>(~_shifted_mask)) | _val
        );
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 52;

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
        if (auto _e = w.write_u8(cbyte); _e) return _e;
        if (auto _e = w.write_bytes(zid.data(), zid.size()); _e) return _e;
        if (sn_res.has_value()) {
            auto _v = *sn_res;
            if (auto _e = w.write_u8(_v); _e) return _e;
        }
        if (batch_size.has_value()) {
            auto _v = *batch_size;
            if (auto _e = w.write_u8(static_cast<std::uint8_t>(_v)); _e) return _e;
            if (auto _e = w.write_u8(static_cast<std::uint8_t>(_v >> 8)); _e) return _e;
        }
        {
            std::uint64_t _w = static_cast<std::uint64_t>(lease);
            while (_w >= 0x80) {
                if (auto _e = w.write_u8(static_cast<std::uint8_t>((_w & 0x7F) | 0x80)); _e) return _e;
                _w >>= 7;
            }
            if (auto _e = w.write_u8(static_cast<std::uint8_t>(_w)); _e) return _e;
        }
        {
            std::uint64_t _w = static_cast<std::uint64_t>(next_sn_reliable);
            while (_w >= 0x80) {
                if (auto _e = w.write_u8(static_cast<std::uint8_t>((_w & 0x7F) | 0x80)); _e) return _e;
                _w >>= 7;
            }
            if (auto _e = w.write_u8(static_cast<std::uint8_t>(_w)); _e) return _e;
        }
        {
            std::uint64_t _w = static_cast<std::uint64_t>(next_sn_best_effort);
            while (_w >= 0x80) {
                if (auto _e = w.write_u8(static_cast<std::uint8_t>((_w & 0x7F) | 0x80)); _e) return _e;
                _w >>= 7;
            }
            if (auto _e = w.write_u8(static_cast<std::uint8_t>(_w)); _e) return _e;
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

}  // namespace SCE::Generated::CodecZenohJoin

#endif  // SCE_FORGE_CODEC_ZENOH_JOIN_H
