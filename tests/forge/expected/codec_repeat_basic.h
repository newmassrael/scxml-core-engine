// SCE-MAP: codec_repeat_basic:11

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_REPEAT_BASIC_H
#define SCE_FORGE_CODEC_REPEAT_BASIC_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"
#include "codec_repeat_elem.h"

namespace SCE::Generated::CodecRepeatBasic {

struct CodecRepeatBasic {
    uint8_t num_frags;
    std::vector<::SCE::Generated::CodecRepeatElem::CodecRepeatElem> frags;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecRepeatBasic> decode(::SCE::Forge::SceCursor& cursor) {
        // RFC §5.B B2 repeat (trunk): per-field cursor advance. Fixed
        // prefix fields read via the present-if helper's non-gated
        // branch; repeat fields run a count-driven or until-eof loop
        // over the imported codec's decode(). Element NeedMoreBytes
        // unwinds the partial frame via std::nullopt.
        uint8_t num_frags;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            num_frags = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::vector<::SCE::Generated::CodecRepeatElem::CodecRepeatElem> frags;
        frags.reserve(num_frags);
        for (auto _i = decltype(num_frags){0}; _i < num_frags; ++_i) {
            auto _elem = ::SCE::Generated::CodecRepeatElem::CodecRepeatElem::decode(cursor);
            if (!_elem.has_value()) return std::nullopt;
            frags.push_back(*_elem);
        }
        return CodecRepeatBasic{
            .num_frags = num_frags,
            .frags = frags,
        };
    }

    std::vector<uint8_t> encode() const {
        // RFC §5.B B2 encode: fixed prefix appends byte-by-byte;
        // repeat fields iterate the host vector and splice each
        // element's encode() into the parent buffer. Author keeps
        // count field == list length (trust contract).
        std::vector<uint8_t> r;
        r.reserve(65);
        r.push_back(num_frags);
        for (const auto& _e : frags) {
            auto _sub = _e.encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        return r;
    }
};

}  // namespace SCE::Generated::CodecRepeatBasic

#endif  // SCE_FORGE_CODEC_REPEAT_BASIC_H
