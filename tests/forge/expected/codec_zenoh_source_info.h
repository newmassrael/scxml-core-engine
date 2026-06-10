// SCE-MAP: codec_zenoh_source_info:57

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_SOURCE_INFO_H
#define SCE_FORGE_CODEC_ZENOH_SOURCE_INFO_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecZenohSourceInfo {

struct CodecZenohSourceInfo {
    uint8_t header;
    std::vector<uint8_t> zid;
    uint32_t eid;
    uint32_t sn;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohSourceInfo> decode(::SCE::Forge::SceCursor& cursor) {
        // Streaming codec: each field reads from the cursor directly
        // (VLE base-128 chain, 1..=ceil(N/7) bytes per field). RFC §5.B
        // B4: per-field bit-size dispatch routes Fixed / LengthRef
        // siblings of VLE fields through `present_if_decode_stmt`
        // (predicate=None arms) — pure-VLE codecs stay byte-stable
        // because the non-gated VLE arm there reuses
        // `vle_decode_stmt` verbatim.
        uint8_t header;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            header = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::vector<uint8_t> zid;
        {
            std::size_t _n = static_cast<std::size_t>((header >> 4) & 0xF) + 1;
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            zid.assign(raw, raw + _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        auto eid_opt = cursor.read_vle_u32();
        if (!eid_opt.has_value()) return std::nullopt;
        auto eid = static_cast<std::uint32_t>(*eid_opt);
        auto sn_opt = cursor.read_vle_u32();
        if (!sn_opt.has_value()) return std::nullopt;
        auto sn = static_cast<std::uint32_t>(*sn_opt);
        return CodecZenohSourceInfo{
            .header = header,
            .zid = zid,
            .eid = eid,
            .sn = sn,
        };
    }

    // RFC §5.B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    uint8_t zidlen_m1() const noexcept {
        return static_cast<uint8_t>(
            (this->header >> 4) & static_cast<uint8_t>(0x0F)
        );
    }

    void set_zidlen_m1(uint8_t v) noexcept {
        const uint8_t _shifted_mask =
            static_cast<uint8_t>(
                static_cast<uint8_t>(0x0F) << 4
            );
        const uint8_t _val =
            static_cast<uint8_t>(
                (static_cast<uint8_t>(v) & static_cast<uint8_t>(0x0F)) << 4
            );
        this->header = static_cast<uint8_t>(
            (this->header & static_cast<uint8_t>(~_shifted_mask)) | _val
        );
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 27;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable sinks
    /// (e.g. `VectorSink`) are effectively infallible.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w) const noexcept {
        // RFC §5.B B4: per-field bit-size dispatch routes Fixed /
        // LengthRef siblings of VLE fields through
        // `present_if_encode_block` (predicate=None arms). Pure-VLE
        // codecs stay byte-stable because the non-gated VLE arm there
        // reuses `vle_encode_block` with the language-appropriate
        // self/struct prefix.
        if (auto _e = w.write_u8(header); _e) return _e;
        if (auto _e = w.write_bytes(zid.data(), zid.size()); _e) return _e;
        {
            std::uint64_t _w = static_cast<std::uint64_t>(eid);
            while (_w >= 0x80) {
                if (auto _e = w.write_u8(static_cast<std::uint8_t>((_w & 0x7F) | 0x80)); _e) return _e;
                _w >>= 7;
            }
            if (auto _e = w.write_u8(static_cast<std::uint8_t>(_w)); _e) return _e;
        }
        {
            std::uint64_t _w = static_cast<std::uint64_t>(sn);
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
    [[nodiscard]] std::vector<std::uint8_t> encode_to_vec() const {
        std::vector<std::uint8_t> _sce_v;
        _sce_v.reserve(MAX_ENCODED_BYTES);
        ::SCE::Forge::VectorSink _sce_sink(_sce_v);
        (void)encode(_sce_sink);
        return _sce_v;
    }
};

}  // namespace SCE::Generated::CodecZenohSourceInfo

#endif  // SCE_FORGE_CODEC_ZENOH_SOURCE_INFO_H
