// SCE-MAP: codec_zenoh_hello:41

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_HELLO_H
#define SCE_FORGE_CODEC_ZENOH_HELLO_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"
#include "codec_zenoh_locator.h"

namespace SCE::Generated::CodecZenohHello {

struct CodecZenohHello {
    uint8_t version;
    uint8_t cbyte;
    std::vector<uint8_t> zid;
    std::optional<uint64_t> num_locators;
    std::optional<std::vector<::SCE::Generated::CodecZenohLocator::CodecZenohLocator>> locators;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohHello> decode(::SCE::Forge::SceCursor& cursor, std::uint8_t parent_flags) {
        // RFC §5.B B5-γ: `parent_flags` is the parent codec's flags
        // carrier value, threaded by the variant arm dispatcher.
        // Defensive (void) suppress for codecs that declare
        // `<sce:requires-parent-flags>` but don't yet wire any
        // gated field — keeps the param accessible to per-field
        // gates without UB on unused-parameter warnings.
        (void)parent_flags;
        // RFC §5.B B1-δ + B2-β present-if: per-field cursor advance.
        // Gated fields hold std::optional<T>; B2-β extends gating to
        // Tail / LengthRef / Vle bit-sizes via dispatch inside
        // `present_if_decode_stmt`. Per-field `is_repeat` routes
        // Repeat fields to the dedicated helper. Branch fires before
        // has_vle_fields so a codec mixing VLE + present-if uses the
        // unified streaming path.
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
        std::optional<uint64_t> num_locators;
        if ((parent_flags & 0x20) != 0) {
            auto _v_opt = cursor.read_vle_u64();
        if (!_v_opt.has_value()) return std::nullopt;
        auto _v = static_cast<std::uint64_t>(*_v_opt);
            num_locators = _v;
        }
        std::optional<std::vector<::SCE::Generated::CodecZenohLocator::CodecZenohLocator>> locators;
        if ((parent_flags & 0x20) != 0) {
            auto _n = num_locators.value();
            std::vector<::SCE::Generated::CodecZenohLocator::CodecZenohLocator> _list;
            _list.reserve(_n);
            for (auto _i = decltype(_n){0}; _i < _n; ++_i) {
                auto _elem = ::SCE::Generated::CodecZenohLocator::CodecZenohLocator::decode(cursor);
                if (!_elem.has_value()) return std::nullopt;
                _list.push_back(*_elem);
            }
            locators = std::move(_list);
        }
        return CodecZenohHello{
            .version = version,
            .cbyte = cbyte,
            .zid = zid,
            .num_locators = num_locators,
            .locators = locators,
        };
    }

    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors.
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

    std::vector<uint8_t> encode(std::uint8_t parent_flags) const {
        (void)parent_flags;
        // RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        // append. Gated fields skip the append when the optional is
        // empty. Per-field `is_repeat` routes Repeat fields to the
        // dedicated helper. Branch fires before has_vle_fields so a
        // codec mixing VLE + present-if uses the unified encode path.
        std::vector<uint8_t> r;
        r.reserve(8860);
        r.push_back(version);
        r.push_back(cbyte);
        r.insert(r.end(), zid.begin(), zid.end());
        if (num_locators.has_value()) {
            auto _v = *num_locators;
        {
            std::uint64_t _w = static_cast<std::uint64_t>(_v);
            while (_w >= 0x80) {
                r.push_back(static_cast<std::uint8_t>((_w & 0x7F) | 0x80));
                _w >>= 7;
            }
            r.push_back(static_cast<std::uint8_t>(_w));
        }
        }
        if (this->locators.has_value()) {
            for (const auto& _e : *this->locators) {
                auto _sub = _e.encode();
                r.insert(r.end(), _sub.begin(), _sub.end());
            }
        }
        return r;
    }
};

}  // namespace SCE::Generated::CodecZenohHello

#endif  // SCE_FORGE_CODEC_ZENOH_HELLO_H
