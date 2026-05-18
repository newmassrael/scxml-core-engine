// SCE-MAP: codec_zenoh_decl_final:19

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_DECL_FINAL_H
#define SCE_FORGE_CODEC_ZENOH_DECL_FINAL_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecZenohDeclFinal {

struct CodecZenohDeclFinal {

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohDeclFinal> decode(::SCE::Forge::SceCursor& cursor) {
        // RFC §5.B B5-α empty body — zero-byte payload, no cursor work.
        (void)cursor;
        return CodecZenohDeclFinal{};
    }

    std::vector<uint8_t> encode() const {
        // RFC §5.B B5-α empty body — zero-byte payload.
        return {};
    }
};

}  // namespace SCE::Generated::CodecZenohDeclFinal

#endif  // SCE_FORGE_CODEC_ZENOH_DECL_FINAL_H
