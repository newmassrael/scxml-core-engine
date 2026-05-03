// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_EMBED_BASIC_H
#define SCE_FORGE_CODEC_EMBED_BASIC_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"
#include "codec_zenoh_locator.h"

namespace SCE::Generated::CodecEmbedBasic {

struct CodecEmbedBasic {
    uint8_t tag;
    ::SCE::Generated::CodecZenohLocator::CodecZenohLocator locator;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecEmbedBasic> decode(::SCE::Forge::SceCursor& cursor) {
        // RFC §5.B B2 repeat (trunk): per-field cursor advance. Fixed
        // prefix fields read via the present-if helper's non-gated
        // branch; repeat fields run a count-driven or until-eof loop
        // over the imported codec's decode(). Element NeedMoreBytes
        // unwinds the partial frame via std::nullopt.
        uint8_t tag;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            tag = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        auto _emb_locator = ::SCE::Generated::CodecZenohLocator::CodecZenohLocator::decode(cursor);
        if (!_emb_locator.has_value()) return std::nullopt;
        auto locator = std::move(*_emb_locator);
        return CodecEmbedBasic{
            .tag = tag,
            .locator = locator,
        };
    }

    std::vector<uint8_t> encode() const {
        // RFC §5.B B2 encode: fixed prefix appends byte-by-byte;
        // repeat fields iterate the host vector and splice each
        // element's encode() into the parent buffer. Author keeps
        // count field == list length (trust contract).
        std::vector<uint8_t> r;
        r.reserve(257);
        r.push_back(tag);
        {
            auto _sub = locator.encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        return r;
    }
};

}  // namespace SCE::Generated::CodecEmbedBasic

#endif  // SCE_FORGE_CODEC_EMBED_BASIC_H
