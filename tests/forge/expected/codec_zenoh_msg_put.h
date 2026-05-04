// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_MSG_PUT_H
#define SCE_FORGE_CODEC_ZENOH_MSG_PUT_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"
#include "codec_zenoh_timestamp.h"
#include "codec_zenoh_encoding.h"
#include "codec_zenoh_ext_entry.h"

namespace SCE::Generated::CodecZenohMsgPut {

struct CodecZenohMsgPut {
    uint8_t header;
    std::optional<::SCE::Generated::CodecZenohTimestamp::CodecZenohTimestamp> timestamp;
    std::optional<::SCE::Generated::CodecZenohEncoding::CodecZenohEncoding> encoding;
    std::optional<std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>> extensions;
    uint64_t payload_len;
    std::vector<uint8_t> payload;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohMsgPut> decode(::SCE::Forge::SceCursor& cursor) {
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
        std::optional<::SCE::Generated::CodecZenohTimestamp::CodecZenohTimestamp> timestamp;
        if ((header & 0x20) != 0) {
            auto _emb = ::SCE::Generated::CodecZenohTimestamp::CodecZenohTimestamp::decode(cursor);
            if (!_emb.has_value()) return std::nullopt;
            timestamp = std::move(*_emb);
        }
        std::optional<::SCE::Generated::CodecZenohEncoding::CodecZenohEncoding> encoding;
        if ((header & 0x40) != 0) {
            auto _emb = ::SCE::Generated::CodecZenohEncoding::CodecZenohEncoding::decode(cursor);
            if (!_emb.has_value()) return std::nullopt;
            encoding = std::move(*_emb);
        }
        std::optional<std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>> extensions;
        if ((header & 0x80) != 0) {
            std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry> _list;
            _list.reserve(4);
            for (std::size_t _i = 0; _i < 4; ++_i) {
                if (cursor.remaining() == 0) break;
                auto _elem = ::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry::decode(cursor);
                if (!_elem.has_value()) return std::nullopt;
                bool _continue = _elem->z();
                _list.push_back(*_elem);
                if (!_continue) break;
            }
            extensions = std::move(_list);
        }
        auto payload_len_opt = cursor.read_vle_u64();
        if (!payload_len_opt.has_value()) return std::nullopt;
        auto payload_len = static_cast<std::uint64_t>(*payload_len_opt);
        std::vector<uint8_t> payload;
        {
            std::size_t _n = static_cast<std::size_t>(payload_len);
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            payload.assign(raw, raw + _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        return CodecZenohMsgPut{
            .header = header,
            .timestamp = timestamp,
            .encoding = encoding,
            .extensions = extensions,
            .payload_len = payload_len,
            .payload = payload,
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

    bool t() const noexcept {
        return (this->header & 0x20) != 0;
    }

    void set_t(bool v) noexcept {
        if (v) {
            this->header = static_cast<uint8_t>(this->header | 0x20);
        } else {
            this->header = static_cast<uint8_t>(this->header & static_cast<uint8_t>(~0x20));
        }
    }

    bool e() const noexcept {
        return (this->header & 0x40) != 0;
    }

    void set_e(bool v) noexcept {
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
        r.reserve(951);
        r.push_back(header);
        if (this->timestamp.has_value()) {
            auto _sub = this->timestamp->encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        if (this->encoding.has_value()) {
            auto _sub = this->encoding->encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        if (this->extensions.has_value()) {
            for (const auto& _e : *this->extensions) {
                auto _sub = _e.encode();
                r.insert(r.end(), _sub.begin(), _sub.end());
            }
        }
        {
            std::uint64_t _w = static_cast<std::uint64_t>(payload_len);
            while (_w >= 0x80) {
                r.push_back(static_cast<std::uint8_t>((_w & 0x7F) | 0x80));
                _w >>= 7;
            }
            r.push_back(static_cast<std::uint8_t>(_w));
        }
        r.insert(r.end(), payload.begin(), payload.end());
        return r;
    }
};

}  // namespace SCE::Generated::CodecZenohMsgPut

#endif  // SCE_FORGE_CODEC_ZENOH_MSG_PUT_H
