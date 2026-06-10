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
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecRepeatBasic> decode(::SCE::Forge::SceCursor& cursor) {
        // RFC §synth-5-B B2 repeat (trunk): per-field cursor advance. Fixed
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

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 65;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable sinks
    /// (e.g. `VectorSink`) are effectively infallible.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w) const noexcept {
        // RFC §synth-5-B B2 encode: fixed prefix appends byte-by-byte;
        // repeat fields iterate the host vector and splice each
        // element's encode() into the parent buffer. Author keeps
        // count field == list length (trust contract).
        if (auto _e = w.write_u8(num_frags); _e) return _e;
        for (const auto& _e : frags) {
            if (auto _se = _e.encode(w); _se) return _se;
        }
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

}  // namespace SCE::Generated::CodecRepeatBasic

#endif  // SCE_FORGE_CODEC_REPEAT_BASIC_H
