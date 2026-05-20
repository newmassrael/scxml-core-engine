// SCE-MAP: codec_zenoh_decl_kexpr:47

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_DECL_KEXPR_H
#define SCE_FORGE_CODEC_ZENOH_DECL_KEXPR_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"
#include "codec_zenoh_wireexpr.h"

namespace SCE::Generated::CodecZenohDeclKexpr {

struct CodecZenohDeclKexpr {
    uint16_t id;
    ::SCE::Generated::CodecZenohWireexpr::CodecZenohWireexpr wireexpr;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohDeclKexpr> decode(::SCE::Forge::SceCursor& cursor, std::uint8_t n) {
        // RFC Axis-1 inversion: defensive (void) suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly under -Wunused.
        (void)n;
        // Streaming codec: each field reads from the cursor directly
        // (VLE base-128 chain, 1..=ceil(N/7) bytes per field). RFC §5.B
        // B4: per-field bit-size dispatch routes Fixed / LengthRef
        // siblings of VLE fields through `present_if_decode_stmt`
        // (predicate=None arms) — pure-VLE codecs stay byte-stable
        // because the non-gated VLE arm there reuses
        // `vle_decode_stmt` verbatim.
        auto id_opt = cursor.read_vle_u16();
        if (!id_opt.has_value()) return std::nullopt;
        auto id = static_cast<std::uint16_t>(*id_opt);
        auto _emb_wireexpr = ::SCE::Generated::CodecZenohWireexpr::CodecZenohWireexpr::decode(cursor, n);
        if (!_emb_wireexpr.has_value()) return std::nullopt;
        auto wireexpr = std::move(*_emb_wireexpr);
        return CodecZenohDeclKexpr{
            .id = id,
            .wireexpr = wireexpr,
        };
    }

    std::vector<uint8_t> encode(std::uint8_t n) const {
        (void)n;
        // RFC §5.B B4: per-field bit-size dispatch routes Fixed /
        // LengthRef siblings of VLE fields through
        // `present_if_encode_block` (predicate=None arms). Pure-VLE
        // codecs stay byte-stable because the non-gated VLE arm there
        // reuses `vle_encode_block` with the language-appropriate
        // self/struct prefix.
        std::vector<uint8_t> r;
        r.reserve(259);
        {
            std::uint64_t _w = static_cast<std::uint64_t>(id);
            while (_w >= 0x80) {
                r.push_back(static_cast<std::uint8_t>((_w & 0x7F) | 0x80));
                _w >>= 7;
            }
            r.push_back(static_cast<std::uint8_t>(_w));
        }
        {
            auto _sub = wireexpr.encode(n);
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        return r;
    }
};

}  // namespace SCE::Generated::CodecZenohDeclKexpr

#endif  // SCE_FORGE_CODEC_ZENOH_DECL_KEXPR_H
