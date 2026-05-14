// SCE-MAP: codec_zenoh_ext_envelope:35

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_EXT_ENVELOPE_H
#define SCE_FORGE_CODEC_ZENOH_EXT_ENVELOPE_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"
#include "codec_zenoh_ext_entry.h"

namespace SCE::Generated::CodecZenohExtEnvelope {

struct CodecZenohExtEnvelope {
    uint8_t header_flags;
    std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry> extensions;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohExtEnvelope> decode(::SCE::Forge::SceCursor& cursor) {
        // RFC §5.B B2 repeat (trunk): per-field cursor advance. Fixed
        // prefix fields read via the present-if helper's non-gated
        // branch; repeat fields run a count-driven or until-eof loop
        // over the imported codec's decode(). Element NeedMoreBytes
        // unwinds the partial frame via std::nullopt.
        uint8_t header_flags;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            header_flags = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry> extensions;
        extensions.reserve(8);
        for (std::size_t _i = 0; _i < 8; ++_i) {
            if (cursor.remaining() == 0) break;
            auto _elem = ::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry::decode(cursor);
            if (!_elem.has_value()) return std::nullopt;
            bool _continue = _elem->z();
            extensions.push_back(*_elem);
            if (!_continue) break;
        }
        return CodecZenohExtEnvelope{
            .header_flags = header_flags,
            .extensions = extensions,
        };
    }

    std::vector<uint8_t> encode() const {
        // RFC §5.B B2 encode: fixed prefix appends byte-by-byte;
        // repeat fields iterate the host vector and splice each
        // element's encode() into the parent buffer. Author keeps
        // count field == list length (trust contract).
        std::vector<uint8_t> r;
        r.reserve(345);
        r.push_back(header_flags);
        for (const auto& _e : extensions) {
            auto _sub = _e.encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        return r;
    }
};

}  // namespace SCE::Generated::CodecZenohExtEnvelope

#endif  // SCE_FORGE_CODEC_ZENOH_EXT_ENVELOPE_H
