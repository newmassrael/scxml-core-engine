// SCE-MAP: codec_repeat_present_if_basic:37

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_REPEAT_PRESENT_IF_BASIC_H
#define SCE_FORGE_CODEC_REPEAT_PRESENT_IF_BASIC_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"
#include "codec_repeat_elem.h"

namespace SCE::Generated::CodecRepeatPresentIfBasic {

struct CodecRepeatPresentIfBasic {
    uint8_t carrier;
    std::optional<uint8_t> num_elems;
    std::optional<std::vector<::SCE::Generated::CodecRepeatElem::CodecRepeatElem>> elems;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecRepeatPresentIfBasic> decode(::SCE::Forge::SceCursor& cursor) {
        // RFC §5.B present-if: per-field cursor advance.
        // Gated fields hold std::optional<T>; gating extends to
        // Tail / LengthRef / Vle bit-sizes via dispatch inside
        // `present_if_decode_stmt`. Per-field `is_repeat` routes
        // Repeat fields to the dedicated helper. Branch fires before
        // has_vle_fields so a codec mixing VLE + present-if uses the
        // unified streaming path.
        uint8_t carrier;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            carrier = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<uint8_t> num_elems;
        if ((carrier & 0x01) != 0) {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            num_elems = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<std::vector<::SCE::Generated::CodecRepeatElem::CodecRepeatElem>> elems;
        if ((carrier & 0x01) != 0) {
            auto _n = num_elems.value();
            std::vector<::SCE::Generated::CodecRepeatElem::CodecRepeatElem> _list;
            _list.reserve(_n);
            for (auto _i = decltype(_n){0}; _i < _n; ++_i) {
                auto _elem = ::SCE::Generated::CodecRepeatElem::CodecRepeatElem::decode(cursor);
                if (!_elem.has_value()) return std::nullopt;
                _list.push_back(*_elem);
            }
            elems = std::move(_list);
        }
        return CodecRepeatPresentIfBasic{
            .carrier = carrier,
            .num_elems = num_elems,
            .elems = elems,
        };
    }

    // RFC §5.B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool has_list() const noexcept {
        return (this->carrier & 0x01) != 0;
    }

    void set_has_list(bool v) noexcept {
        if (v) {
            this->carrier = static_cast<uint8_t>(this->carrier | 0x01);
        } else {
            this->carrier = static_cast<uint8_t>(this->carrier & static_cast<uint8_t>(~0x01));
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 66;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable sinks
    /// (e.g. `VectorSink`) are effectively infallible.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w) const noexcept {
        // RFC §5.B present-if encode: per-field byte
        // append. Gated fields skip the append when the optional is
        // empty. Per-field `is_repeat` routes Repeat fields to the
        // dedicated helper. Branch fires before has_vle_fields so a
        // codec mixing VLE + present-if uses the unified encode path.
        if (auto _e = w.write_u8(carrier); _e) return _e;
        if (num_elems.has_value()) {
            auto _v = *num_elems;
            if (auto _e = w.write_u8(_v); _e) return _e;
        }
        if (this->elems.has_value()) {
            for (const auto& _e : *this->elems) {
                if (auto _se = _e.encode(w); _se) return _se;
            }
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

}  // namespace SCE::Generated::CodecRepeatPresentIfBasic

#endif  // SCE_FORGE_CODEC_REPEAT_PRESENT_IF_BASIC_H
