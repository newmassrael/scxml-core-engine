// SCE-MAP: codec_init_syn_body:30

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_INIT_SYN_BODY_H
#define SCE_FORGE_CODEC_INIT_SYN_BODY_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecInitSynBody {

struct CodecInitSynBody {
    uint8_t version;
    std::optional<uint8_t> sn_res;
    std::optional<uint16_t> batch_size;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecInitSynBody> decode(::SCE::Forge::SceCursor& cursor, std::uint8_t s) {
        // RFC Axis-1 inversion: defensive (void) suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly under -Wunused.
        (void)s;
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
        std::optional<uint8_t> sn_res;
        if ((s & 0x01) != 0) {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            sn_res = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<uint16_t> batch_size;
        if ((s & 0x01) != 0) {
            const std::uint8_t* raw = cursor.peek_slice(2);
            if (raw == nullptr) return std::nullopt;
            batch_size = static_cast<uint16_t>(static_cast<uint16_t>((static_cast<uint16_t>(raw[0]) << 8) | raw[1]));
            if (!cursor.advance(2)) return std::nullopt;
        }
        return CodecInitSynBody{
            .version = version,
            .sn_res = sn_res,
            .batch_size = batch_size,
        };
    }

    std::vector<uint8_t> encode(std::uint8_t s) const {
        (void)s;
        // RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        // append. Gated fields skip the append when the optional is
        // empty. Per-field `is_repeat` routes Repeat fields to the
        // dedicated helper. Branch fires before has_vle_fields so a
        // codec mixing VLE + present-if uses the unified encode path.
        std::vector<uint8_t> r;
        r.reserve(4);
        r.push_back(version);
        if (sn_res.has_value()) {
            auto _v = *sn_res;
            r.push_back(_v);
        }
        if (batch_size.has_value()) {
            auto _v = *batch_size;
            r.push_back(static_cast<std::uint8_t>(_v >> 8));
            r.push_back(static_cast<std::uint8_t>(_v));
        }
        return r;
    }
};

}  // namespace SCE::Generated::CodecInitSynBody

#endif  // SCE_FORGE_CODEC_INIT_SYN_BODY_H
