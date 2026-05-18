// SCE-MAP: codec_zenoh_response:75

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_RESPONSE_H
#define SCE_FORGE_CODEC_ZENOH_RESPONSE_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <variant>
#include <string>

#include "sce/forge/codec.h"
#include "codec_zenoh_ext_entry.h"
#include "codec_zenoh_reply.h"
#include "codec_zenoh_err.h"

namespace SCE::Generated::CodecZenohResponse {

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
// codec's tag-field suffix. `std::variant` carries one of N arm bodies
// (each an imported codec type); the optional Default arm is a small
// struct that bundles the runtime tag value with the catch-all body.
struct CodecZenohResponseDefault {
    uint8_t tag;
    ::SCE::Generated::CodecZenohReply::CodecZenohReply body;
};
using CodecZenohResponseVariant = std::variant<
    ::SCE::Generated::CodecZenohReply::CodecZenohReply,
    ::SCE::Generated::CodecZenohErr::CodecZenohErr,
    CodecZenohResponseDefault
>;

struct CodecZenohResponse {
    uint8_t header;
    uint64_t request_id;
    uint32_t key_id;
    std::optional<uint64_t> suffix_len;
    std::optional<std::string> suffix;
    std::optional<std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>> extensions;
    // RFC variant-default-uniformity Atomic β-cpp: the
    // `std::in_place_index_t<N>{}` tag-type selects the arm marked
    // `<sce:arm default="true"/>` by index so a freshly-constructed
    // envelope holds that arm (not the first declared alternative
    // which `std::variant`'s default constructor would otherwise
    // pick), encoding its wire-MID for byte-exact round-trip.
    // (We construct the tag type explicitly — `std::in_place_index<N>`
    // is a variable template of type `std::in_place_index_t<N>` and
    // the brace-init form `std::in_place_index<N>{}` does not parse
    // in a member-init context.)
    CodecZenohResponseVariant body{std::in_place_index_t<0>{}};

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohResponse> decode(::SCE::Forge::SceCursor& cursor) {
        // RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
        // streaming prefix decode (variable-length fields supported via
        // per-field present_if/tlv-chain/embed/repeat helpers). Peek-byte
        // mode additionally peeks the cursor's next byte for variant tag
        // without advancing — arm body decoder reads it as its own
        // header byte.
        uint8_t header;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            header = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        auto request_id_opt = cursor.read_vle_u64();
        if (!request_id_opt.has_value()) return std::nullopt;
        auto request_id = static_cast<std::uint64_t>(*request_id_opt);
        auto key_id_opt = cursor.read_vle_u32();
        if (!key_id_opt.has_value()) return std::nullopt;
        auto key_id = static_cast<std::uint32_t>(*key_id_opt);
        std::optional<uint64_t> suffix_len;
        if ((header & 0x20) != 0) {
            auto _v_opt = cursor.read_vle_u64();
        if (!_v_opt.has_value()) return std::nullopt;
        auto _v = static_cast<std::uint64_t>(*_v_opt);
            suffix_len = _v;
        }
        std::optional<std::string> suffix;
        if ((header & 0x20) != 0) {
            std::size_t _n = static_cast<std::size_t>(suffix_len.value());
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            if (!::SCE::Forge::is_valid_utf8(raw, _n)) return std::nullopt;
            suffix.emplace(reinterpret_cast<const char*>(raw), _n);
            if (!cursor.advance(_n)) return std::nullopt;
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
        const std::uint8_t* _peek_raw = cursor.peek_slice(1);
        if (_peek_raw == nullptr) return std::nullopt;
        const std::uint8_t _peek = _peek_raw[0];
        // Dispatch on tag value into the matching arm body.
        CodecZenohResponseVariant body;
        switch (static_cast<uint8_t>((_peek >> 0) & static_cast<uint8_t>(0x1F))) {
            case 4: {
                auto _arm = ::SCE::Generated::CodecZenohReply::CodecZenohReply::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 5: {
                auto _arm = ::SCE::Generated::CodecZenohErr::CodecZenohErr::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            default: {
                auto _arm = ::SCE::Generated::CodecZenohReply::CodecZenohReply::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = CodecZenohResponseDefault{
                    .tag = static_cast<uint8_t>((_peek >> 0) & static_cast<uint8_t>(0x1F)),
                    .body = *_arm,
                };
                break;
            }
        }
        return CodecZenohResponse{
            .header = header,
            .request_id = request_id,
            .key_id = key_id,
            .suffix_len = suffix_len,
            .suffix = suffix,
            .extensions = extensions,
            .body = body,
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

    bool n() const noexcept {
        return (this->header & 0x20) != 0;
    }

    void set_n(bool v) noexcept {
        if (v) {
            this->header = static_cast<uint8_t>(this->header | 0x20);
        } else {
            this->header = static_cast<uint8_t>(this->header & static_cast<uint8_t>(~0x20));
        }
    }

    bool m() const noexcept {
        return (this->header & 0x40) != 0;
    }

    void set_m(bool v) noexcept {
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
        // RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
        // streaming prefix encode. Peek-byte mode: arm body's encode
        // prepends its own header byte (which the decoder peeked); no
        // separate tag byte here. Streaming-prefix mode (own-field):
        // carrier is part of the prefix fields and emits via the same
        // per-field path.
        std::vector<uint8_t> r;
        r.reserve(977);
        r.push_back(header);
        {
            std::uint64_t _w = static_cast<std::uint64_t>(request_id);
            while (_w >= 0x80) {
                r.push_back(static_cast<std::uint8_t>((_w & 0x7F) | 0x80));
                _w >>= 7;
            }
            r.push_back(static_cast<std::uint8_t>(_w));
        }
        {
            std::uint64_t _w = static_cast<std::uint64_t>(key_id);
            while (_w >= 0x80) {
                r.push_back(static_cast<std::uint8_t>((_w & 0x7F) | 0x80));
                _w >>= 7;
            }
            r.push_back(static_cast<std::uint8_t>(_w));
        }
        if (suffix_len.has_value()) {
            auto _v = *suffix_len;
        {
            std::uint64_t _w = static_cast<std::uint64_t>(_v);
            while (_w >= 0x80) {
                r.push_back(static_cast<std::uint8_t>((_w & 0x7F) | 0x80));
                _w >>= 7;
            }
            r.push_back(static_cast<std::uint8_t>(_w));
        }
        }
        if (suffix.has_value()) {
            r.insert(r.end(),
                reinterpret_cast<const std::uint8_t*>(suffix->data()),
                reinterpret_cast<const std::uint8_t*>(suffix->data()) + suffix->size());
        }
        if (this->extensions.has_value()) {
            for (const auto& _e : *this->extensions) {
                auto _sub = _e.encode();
                r.insert(r.end(), _sub.begin(), _sub.end());
            }
        }
        // Append the active arm body's encoded bytes.
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohReply::CodecZenohReply>(&body)) {
            auto _sub = _p->encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohErr::CodecZenohErr>(&body)) {
            auto _sub = _p->encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        if (auto _p = std::get_if<CodecZenohResponseDefault>(&body)) {
            auto _sub = _p->body.encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        return r;
    }
};

}  // namespace SCE::Generated::CodecZenohResponse

#endif  // SCE_FORGE_CODEC_ZENOH_RESPONSE_H
