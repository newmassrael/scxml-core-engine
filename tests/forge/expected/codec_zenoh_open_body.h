// SCE-MAP: codec_zenoh_open_body:41

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_OPEN_BODY_H
#define SCE_FORGE_CODEC_ZENOH_OPEN_BODY_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecZenohOpenBody {

struct CodecZenohOpenBody {
    uint64_t lease;
    uint64_t initial_sn;
    std::optional<uint64_t> cookie_len;
    std::optional<std::vector<uint8_t>> cookie;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohOpenBody> decode(::SCE::Forge::SceCursor& cursor, std::uint8_t a) {
        // RFC Axis-1 inversion: defensive (void) suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly under -Wunused.
        (void)a;
        // RFC §5.B B1-δ + B2-β present-if: per-field cursor advance.
        // Gated fields hold std::optional<T>; B2-β extends gating to
        // Tail / LengthRef / Vle bit-sizes via dispatch inside
        // `present_if_decode_stmt`. Per-field `is_repeat` routes
        // Repeat fields to the dedicated helper. Branch fires before
        // has_vle_fields so a codec mixing VLE + present-if uses the
        // unified streaming path.
        auto lease_opt = cursor.read_vle_u64();
        if (!lease_opt.has_value()) return std::nullopt;
        auto lease = static_cast<std::uint64_t>(*lease_opt);
        auto initial_sn_opt = cursor.read_vle_u64();
        if (!initial_sn_opt.has_value()) return std::nullopt;
        auto initial_sn = static_cast<std::uint64_t>(*initial_sn_opt);
        std::optional<uint64_t> cookie_len;
        if ((a & 0x01) == 0) {
            auto _v_opt = cursor.read_vle_u64();
        if (!_v_opt.has_value()) return std::nullopt;
        auto _v = static_cast<std::uint64_t>(*_v_opt);
            cookie_len = _v;
        }
        std::optional<std::vector<uint8_t>> cookie;
        if ((a & 0x01) == 0) {
            std::size_t _n = static_cast<std::size_t>(cookie_len.value());
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            cookie.emplace(raw, raw + _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        return CodecZenohOpenBody{
            .lease = lease,
            .initial_sn = initial_sn,
            .cookie_len = cookie_len,
            .cookie = cookie,
        };
    }

    std::vector<uint8_t> encode(std::uint8_t a) const {
        (void)a;
        // RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        // append. Gated fields skip the append when the optional is
        // empty. Per-field `is_repeat` routes Repeat fields to the
        // dedicated helper. Branch fires before has_vle_fields so a
        // codec mixing VLE + present-if uses the unified encode path.
        std::vector<uint8_t> r;
        r.reserve(158);
        {
            std::uint64_t _w = static_cast<std::uint64_t>(lease);
            while (_w >= 0x80) {
                r.push_back(static_cast<std::uint8_t>((_w & 0x7F) | 0x80));
                _w >>= 7;
            }
            r.push_back(static_cast<std::uint8_t>(_w));
        }
        {
            std::uint64_t _w = static_cast<std::uint64_t>(initial_sn);
            while (_w >= 0x80) {
                r.push_back(static_cast<std::uint8_t>((_w & 0x7F) | 0x80));
                _w >>= 7;
            }
            r.push_back(static_cast<std::uint8_t>(_w));
        }
        if (cookie_len.has_value()) {
            auto _v = *cookie_len;
        {
            std::uint64_t _w = static_cast<std::uint64_t>(_v);
            while (_w >= 0x80) {
                r.push_back(static_cast<std::uint8_t>((_w & 0x7F) | 0x80));
                _w >>= 7;
            }
            r.push_back(static_cast<std::uint8_t>(_w));
        }
        }
        if (cookie.has_value()) {
            r.insert(r.end(), cookie->begin(), cookie->end());
        }
        return r;
    }
};

}  // namespace SCE::Generated::CodecZenohOpenBody

#endif  // SCE_FORGE_CODEC_ZENOH_OPEN_BODY_H
