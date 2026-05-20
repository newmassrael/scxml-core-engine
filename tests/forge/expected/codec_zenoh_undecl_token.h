// SCE-MAP: codec_zenoh_undecl_token:16

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_UNDECL_TOKEN_H
#define SCE_FORGE_CODEC_ZENOH_UNDECL_TOKEN_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"
#include "codec_zenoh_decl_ext_keyexpr.h"

namespace SCE::Generated::CodecZenohUndeclToken {

struct CodecZenohUndeclToken {
    uint32_t id;
    std::optional<::SCE::Generated::CodecZenohDeclExtKeyexpr::CodecZenohDeclExtKeyexpr> ext_keyexpr;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohUndeclToken> decode(::SCE::Forge::SceCursor& cursor, std::uint8_t z) {
        // RFC Axis-1 inversion: defensive (void) suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly under -Wunused.
        (void)z;
        // RFC §5.B B1-δ + B2-β present-if: per-field cursor advance.
        // Gated fields hold std::optional<T>; B2-β extends gating to
        // Tail / LengthRef / Vle bit-sizes via dispatch inside
        // `present_if_decode_stmt`. Per-field `is_repeat` routes
        // Repeat fields to the dedicated helper. Branch fires before
        // has_vle_fields so a codec mixing VLE + present-if uses the
        // unified streaming path.
        auto id_opt = cursor.read_vle_u32();
        if (!id_opt.has_value()) return std::nullopt;
        auto id = static_cast<std::uint32_t>(*id_opt);
        std::optional<::SCE::Generated::CodecZenohDeclExtKeyexpr::CodecZenohDeclExtKeyexpr> ext_keyexpr;
        if ((z & 0x01) != 0) {
            auto _emb = ::SCE::Generated::CodecZenohDeclExtKeyexpr::CodecZenohDeclExtKeyexpr::decode(cursor);
            if (!_emb.has_value()) return std::nullopt;
            ext_keyexpr = std::move(*_emb);
        }
        return CodecZenohUndeclToken{
            .id = id,
            .ext_keyexpr = ext_keyexpr,
        };
    }

    std::vector<uint8_t> encode(std::uint8_t z) const {
        (void)z;
        // RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        // append. Gated fields skip the append when the optional is
        // empty. Per-field `is_repeat` routes Repeat fields to the
        // dedicated helper. Branch fires before has_vle_fields so a
        // codec mixing VLE + present-if uses the unified encode path.
        std::vector<uint8_t> r;
        r.reserve(261);
        {
            std::uint64_t _w = static_cast<std::uint64_t>(id);
            while (_w >= 0x80) {
                r.push_back(static_cast<std::uint8_t>((_w & 0x7F) | 0x80));
                _w >>= 7;
            }
            r.push_back(static_cast<std::uint8_t>(_w));
        }
        if (this->ext_keyexpr.has_value()) {
            auto _sub = this->ext_keyexpr->encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        return r;
    }
};

}  // namespace SCE::Generated::CodecZenohUndeclToken

#endif  // SCE_FORGE_CODEC_ZENOH_UNDECL_TOKEN_H
