// SCE-MAP: codec_zenoh_undecl_subscriber:46

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_UNDECL_SUBSCRIBER_H
#define SCE_FORGE_CODEC_ZENOH_UNDECL_SUBSCRIBER_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"
#include "codec_zenoh_decl_ext_keyexpr.h"

namespace SCE::Generated::CodecZenohUndeclSubscriber {

struct CodecZenohUndeclSubscriber {
    uint32_t id;
    std::optional<::SCE::Generated::CodecZenohDeclExtKeyexpr::CodecZenohDeclExtKeyexpr> ext_keyexpr;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohUndeclSubscriber> decode(::SCE::Forge::SceCursor& cursor, std::uint8_t z) {
        // Declared-but-unconsumed flag inputs: defensive (void) suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly under -Wunused.
        (void)z;
        // RFC §5.B present-if: per-field cursor advance.
        // Gated fields hold std::optional<T>; gating extends to
        // Tail / LengthRef / Vle bit-sizes via dispatch inside
        // `present_if_decode_stmt`. Per-field `is_repeat` routes
        // Repeat fields to the dedicated helper. Branch fires before
        // has_vle_fields so a codec mixing VLE + present-if uses the
        // unified streaming path.
        auto id_opt = cursor.read_vle_u32();
        if (!id_opt.has_value()) return std::nullopt;
        auto id = static_cast<std::uint32_t>(*id_opt);
        std::optional<::SCE::Generated::CodecZenohDeclExtKeyexpr::CodecZenohDeclExtKeyexpr> ext_keyexpr;
        if ((z & 0x01) != 0) {
            auto _emb = ::SCE::Generated::CodecZenohDeclExtKeyexpr::CodecZenohDeclExtKeyexpr::decode(cursor);
            if (!_emb.has_value()) return std::nullopt;
            ext_keyexpr = std::move(*_emb);
        }
        return CodecZenohUndeclSubscriber{
            .id = id,
            .ext_keyexpr = ext_keyexpr,
        };
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 261;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable sinks
    /// (e.g. `VectorSink`) are effectively infallible.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w, std::uint8_t z) const noexcept {
        (void)z;
        // RFC §5.B present-if encode: per-field byte
        // append. Gated fields skip the append when the optional is
        // empty. Per-field `is_repeat` routes Repeat fields to the
        // dedicated helper. Branch fires before has_vle_fields so a
        // codec mixing VLE + present-if uses the unified encode path.
        {
            std::uint64_t _w = static_cast<std::uint64_t>(id);
            while (_w >= 0x80) {
                if (auto _e = w.write_u8(static_cast<std::uint8_t>((_w & 0x7F) | 0x80)); _e) return _e;
                _w >>= 7;
            }
            if (auto _e = w.write_u8(static_cast<std::uint8_t>(_w)); _e) return _e;
        }
        if (this->ext_keyexpr.has_value()) {
            if (auto _e = this->ext_keyexpr->encode(w); _e) return _e;
        }
        return std::nullopt;
    }

    /// Heap-backed convenience facade. Pre-reserves `MAX_ENCODED_BYTES`
    /// so the worst-case write path performs at most one allocation,
    /// then delegates to `encode` over a `VectorSink`. Returns the
    /// freshly-encoded byte vector. Callers targeting zero-alloc hot
    /// paths should call `encode` directly against a caller-owned sink.
    [[nodiscard]] std::vector<std::uint8_t> encode_to_vec(std::uint8_t z) const {
        std::vector<std::uint8_t> _sce_v;
        _sce_v.reserve(MAX_ENCODED_BYTES);
        ::SCE::Forge::VectorSink _sce_sink(_sce_v);
        (void)encode(_sce_sink, z);
        return _sce_v;
    }
};

}  // namespace SCE::Generated::CodecZenohUndeclSubscriber

#endif  // SCE_FORGE_CODEC_ZENOH_UNDECL_SUBSCRIBER_H
