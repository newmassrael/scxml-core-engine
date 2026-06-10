// SCE-MAP: codec_embed_basic:43

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
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecEmbedBasic> decode(::SCE::Forge::SceCursor& cursor) {
        // RFC §synth-5-B B2 repeat (trunk): per-field cursor advance. Fixed
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

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 257;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable sinks
    /// (e.g. `VectorSink`) are effectively infallible.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w) const noexcept {
        // RFC §synth-5-B B2 encode: fixed prefix appends byte-by-byte;
        // repeat fields iterate the host vector and splice each
        // element's encode() into the parent buffer. Author keeps
        // count field == list length (trust contract).
        if (auto _e = w.write_u8(tag); _e) return _e;
        if (auto _e = locator.encode(w); _e) return _e;
        return std::nullopt;
    }

    /// Heap-backed convenience facade. Pre-reserves `MAX_ENCODED_BYTES`
    /// so the worst-case write path performs at most one allocation,
    /// then delegates to `encode` over a `VectorSink`. Returns the
    /// freshly-encoded byte vector. Callers targeting zero-alloc hot
    /// paths should call `encode` directly against a caller-owned sink.
    [[nodiscard]] std::vector<std::uint8_t> encode_to_vec() const {
        std::vector<std::uint8_t> _sce_v;
        _sce_v.reserve(MAX_ENCODED_BYTES);
        ::SCE::Forge::VectorSink _sce_sink(_sce_v);
        (void)encode(_sce_sink);
        return _sce_v;
    }
};

}  // namespace SCE::Generated::CodecEmbedBasic

#endif  // SCE_FORGE_CODEC_EMBED_BASIC_H
