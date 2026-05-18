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
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohQuery> decode(::SCE::Forge::SceCursor& cursor) {
        // RFC §5.B B1-δ + B2-β present-if: per-field cursor advance.
        // Gated fields hold std::optional<T>; B2-β extends gating to
        // Tail / LengthRef / Vle bit-sizes via dispatch inside
        // `present_if_decode_stmt`. Per-field `is_repeat` routes
        // Repeat fields to the dedicated helper. Branch fires before
        // has_vle_fields so a codec mixing VLE + present-if uses the
        // unified streaming path.
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

    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors.
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

    std::vector<uint8_t> encode() const {
        // RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        // append. Gated fields skip the append when the optional is
        // empty. Per-field `is_repeat` routes Repeat fields to the
        // dedicated helper. Branch fires before has_vle_fields so a
        // codec mixing VLE + present-if uses the unified encode path.
        std::vector<uint8_t> r;
        r.reserve(612);
        r.push_back(header);
        if (consolidation.has_value()) {
            auto _v = *consolidation;
            r.push_back(_v);
        }
        if (parameters_len.has_value()) {
            auto _v = *parameters_len;
        {
            std::uint64_t _w = static_cast<std::uint64_t>(_v);
            while (_w >= 0x80) {
                r.push_back(static_cast<std::uint8_t>((_w & 0x7F) | 0x80));
                _w >>= 7;
            }
            r.push_back(static_cast<std::uint8_t>(_w));
        }
        }
        if (parameters.has_value()) {
            r.insert(r.end(), parameters->begin(), parameters->end());
        }
        if (this->extensions.has_value()) {
            for (const auto& _e : *this->extensions) {
                auto _sub = _e.encode();
                r.insert(r.end(), _sub.begin(), _sub.end());
            }
        }
        return r;
    }
};

}  // namespace SCE::Generated::CodecZenohQuery

#endif  // SCE_FORGE_CODEC_ZENOH_QUERY_H
