// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_WIREEXPR_H
#define SCE_FORGE_CODEC_ZENOH_WIREEXPR_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <string>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecZenohWireexpr {

struct CodecZenohWireexpr {
    uint64_t id;
    std::optional<uint64_t> suffix_len;
    std::optional<std::string> suffix;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohWireexpr> decode(::SCE::Forge::SceCursor& cursor, std::uint8_t parent_flags) {
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
        auto id_opt = cursor.read_vle_u64();
        if (!id_opt.has_value()) return std::nullopt;
        auto id = static_cast<std::uint64_t>(*id_opt);
        std::optional<uint64_t> suffix_len;
        if ((parent_flags & 0x20) != 0) {
            auto _v_opt = cursor.read_vle_u64();
        if (!_v_opt.has_value()) return std::nullopt;
        auto _v = static_cast<std::uint64_t>(*_v_opt);
            suffix_len = _v;
        }
        std::optional<std::string> suffix;
        if ((parent_flags & 0x20) != 0) {
            std::size_t _n = static_cast<std::size_t>(suffix_len.value());
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            if (!::SCE::Forge::is_valid_utf8(raw, _n)) return std::nullopt;
            suffix.emplace(reinterpret_cast<const char*>(raw), _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        return CodecZenohWireexpr{
            .id = id,
            .suffix_len = suffix_len,
            .suffix = suffix,
        };
    }

    std::vector<uint8_t> encode(std::uint8_t parent_flags) const {
        (void)parent_flags;
        // RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        // append. Gated fields skip the append when the optional is
        // empty. Per-field `is_repeat` routes Repeat fields to the
        // dedicated helper. Branch fires before has_vle_fields so a
        // codec mixing VLE + present-if uses the unified encode path.
        std::vector<uint8_t> r;
        r.reserve(148);
        {
            std::uint64_t _w = static_cast<std::uint64_t>(id);
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
        return r;
    }
};

}  // namespace SCE::Generated::CodecZenohWireexpr

#endif  // SCE_FORGE_CODEC_ZENOH_WIREEXPR_H
