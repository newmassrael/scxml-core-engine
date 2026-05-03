// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_OPEN_ACK_H
#define SCE_FORGE_CODEC_ZENOH_OPEN_ACK_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecZenohOpenAck {

struct CodecZenohOpenAck {
    uint64_t lease;
    uint64_t initial_sn;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohOpenAck> decode(::SCE::Forge::SceCursor& cursor) {
        // Streaming codec: each field reads from the cursor directly
        // (VLE base-128 chain, 1..=ceil(N/7) bytes per field). RFC §5.B
        // B4: per-field bit-size dispatch routes Fixed / LengthRef
        // siblings of VLE fields through `present_if_decode_stmt`
        // (predicate=None arms) — pure-VLE codecs stay byte-stable
        // because the non-gated VLE arm there reuses
        // `vle_decode_stmt` verbatim.
        auto lease_opt = cursor.read_vle_u64();
        if (!lease_opt.has_value()) return std::nullopt;
        auto lease = static_cast<std::uint64_t>(*lease_opt);
        auto initial_sn_opt = cursor.read_vle_u64();
        if (!initial_sn_opt.has_value()) return std::nullopt;
        auto initial_sn = static_cast<std::uint64_t>(*initial_sn_opt);
        return CodecZenohOpenAck{
            .lease = lease,
            .initial_sn = initial_sn,
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
        r.reserve(20);
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
        return r;
    }
};

}  // namespace SCE::Generated::CodecZenohOpenAck

#endif  // SCE_FORGE_CODEC_ZENOH_OPEN_ACK_H
