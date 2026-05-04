// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_SOURCE_INFO_EXT_H
#define SCE_FORGE_CODEC_ZENOH_SOURCE_INFO_EXT_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"
#include "codec_zenoh_source_info.h"

namespace SCE::Generated::CodecZenohSourceInfoExt {

struct CodecZenohSourceInfoExt {
    uint64_t ext_size;
    ::SCE::Generated::CodecZenohSourceInfo::CodecZenohSourceInfo info;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohSourceInfoExt> decode(::SCE::Forge::SceCursor& cursor) {
        // Streaming codec: each field reads from the cursor directly
        // (VLE base-128 chain, 1..=ceil(N/7) bytes per field). RFC §5.B
        // B4: per-field bit-size dispatch routes Fixed / LengthRef
        // siblings of VLE fields through `present_if_decode_stmt`
        // (predicate=None arms) — pure-VLE codecs stay byte-stable
        // because the non-gated VLE arm there reuses
        // `vle_decode_stmt` verbatim.
        auto ext_size_opt = cursor.read_vle_u64();
        if (!ext_size_opt.has_value()) return std::nullopt;
        auto ext_size = static_cast<std::uint64_t>(*ext_size_opt);
        ::SCE::Generated::CodecZenohSourceInfo::CodecZenohSourceInfo info;
        {
            std::size_t _len = static_cast<std::size_t>(ext_size);
            const std::uint8_t* _raw = cursor.peek_slice(_len);
            if (_raw == nullptr) return std::nullopt;
            ::SCE::Forge::SceCursor _inner(_raw, _len);
            auto _emb = ::SCE::Generated::CodecZenohSourceInfo::CodecZenohSourceInfo::decode(_inner);
            if (!_emb.has_value()) return std::nullopt;
            if (!cursor.advance(_len)) return std::nullopt;
            info = std::move(*_emb);
        }
        return CodecZenohSourceInfoExt{
            .ext_size = ext_size,
            .info = info,
        };
    }

    std::vector<uint8_t> encode() const {
        // RFC §5.B B4: per-field bit-size dispatch routes Fixed /
        // LengthRef siblings of VLE fields through
        // `present_if_encode_block` (predicate=None arms). Pure-VLE
        // codecs stay byte-stable because the non-gated VLE arm there
        // reuses `vle_encode_block` with the language-appropriate
        // self/struct prefix.
        std::vector<uint8_t> r;
        r.reserve(266);
        {
            std::uint64_t _w = static_cast<std::uint64_t>(ext_size);
            while (_w >= 0x80) {
                r.push_back(static_cast<std::uint8_t>((_w & 0x7F) | 0x80));
                _w >>= 7;
            }
            r.push_back(static_cast<std::uint8_t>(_w));
        }
        {
            auto _sub = info.encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        return r;
    }
};

}  // namespace SCE::Generated::CodecZenohSourceInfoExt

#endif  // SCE_FORGE_CODEC_ZENOH_SOURCE_INFO_EXT_H
