// SCE-MAP: codec_zenoh_query:51

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_QUERY_H
#define SCE_FORGE_CODEC_ZENOH_QUERY_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"
#include "codec_zenoh_ext_entry.h"

namespace SCE::Generated::CodecZenohQuery {

struct CodecZenohQuery {
    uint8_t header{0x03u};
    std::optional<uint8_t> consolidation;
    std::optional<uint64_t> parameters_len;
    std::optional<std::vector<uint8_t>> parameters;
    std::optional<std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>> extensions;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohQuery> decode(::SCE::Forge::SceCursor& cursor) {
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
        std::optional<uint8_t> consolidation;
        if ((header & 0x20) != 0) {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            consolidation = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<uint64_t> parameters_len;
        if ((header & 0x40) != 0) {
            auto _v_opt = cursor.read_vle_u64();
        if (!_v_opt.has_value()) return std::nullopt;
        auto _v = static_cast<std::uint64_t>(*_v_opt);
            parameters_len = _v;
        }
        std::optional<std::vector<uint8_t>> parameters;
        if ((header & 0x40) != 0) {
            std::size_t _n = static_cast<std::size_t>(parameters_len.value());
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            parameters.emplace(raw, raw + _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        std::optional<std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>> extensions;
        if ((header & 0x80) != 0) {
            std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry> _list;
            _list.reserve(8);
            for (std::size_t _i = 0; _i < 8; ++_i) {
                if (cursor.remaining() == 0) break;
                auto _elem = ::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry::decode(cursor);
                if (!_elem.has_value()) return std::nullopt;
                _list.push_back(*_elem);
            }
            if (cursor.remaining() > 0) return std::nullopt;
            extensions = std::move(_list);
        }
        return CodecZenohQuery{
            .header = header,
            .consolidation = consolidation,
            .parameters_len = parameters_len,
            .parameters = parameters,
            .extensions = extensions,
        };
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    uint8_t mid() const noexcept {
        return static_cast<uint8_t>(
            (this->header >> 0) & static_cast<uint8_t>(0x1F)
        );
    }

    void set_mid(uint8_t v) noexcept {
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

    bool c() const noexcept {
        return (this->header & 0x20) != 0;
    }

    void set_c(bool v) noexcept {
        if (v) {
            this->header = static_cast<uint8_t>(this->header | 0x20);
        } else {
            this->header = static_cast<uint8_t>(this->header & static_cast<uint8_t>(~0x20));
        }
    }

    bool p() const noexcept {
        return (this->header & 0x40) != 0;
    }

    void set_p(bool v) noexcept {
        if (v) {
            this->header = static_cast<uint8_t>(this->header | 0x40);
        } else {
            this->header = static_cast<uint8_t>(this->header & static_cast<uint8_t>(~0x40));
        }
    }

    bool z() const noexcept {
        return (this->header & 0x80) != 0;
    }

    void set_z(bool v) noexcept {
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
    static constexpr std::size_t MAX_ENCODED_BYTES = 603;

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
        if (auto _e = w.write_u8(header); _e) return _e;
        if (consolidation.has_value()) {
            auto _v = *consolidation;
            if (auto _e = w.write_u8(_v); _e) return _e;
        }
        if (parameters_len.has_value()) {
            auto _v = *parameters_len;
        {
            std::uint64_t _w = static_cast<std::uint64_t>(_v);
            std::uint32_t _vn = 0;
            while (_w >= 0x80 && _vn < 8) {
                if (auto _e = w.write_u8(static_cast<std::uint8_t>((_w & 0x7F) | 0x80)); _e) return _e;
                _w >>= 7;
                ++_vn;
            }
            if (auto _e = w.write_u8(static_cast<std::uint8_t>(_w)); _e) return _e;
        }
        }
        if (parameters.has_value()) {
            if (auto _e = w.write_bytes(parameters->data(), parameters->size()); _e) return _e;
        }
        if (this->extensions.has_value()) {
            for (const auto& _e : *this->extensions) {
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
    [[nodiscard]] std::vector<std::uint8_t> encode_to_vec() const {
        std::vector<std::uint8_t> _sce_v;
        _sce_v.reserve(MAX_ENCODED_BYTES);
        ::SCE::Forge::VectorSink _sce_sink(_sce_v);
        (void)encode(_sce_sink);
        return _sce_v;
    }
};

}  // namespace SCE::Generated::CodecZenohQuery

#endif  // SCE_FORGE_CODEC_ZENOH_QUERY_H
