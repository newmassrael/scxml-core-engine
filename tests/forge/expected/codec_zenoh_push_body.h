// SCE-MAP: codec_zenoh_push_body:30 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_PUSH_BODY_H
#define SCE_FORGE_CODEC_ZENOH_PUSH_BODY_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <variant>

#include "sce/forge/codec.h"
#include "codec_zenoh_put.h"
#include "codec_zenoh_del.h"

namespace SCE::Generated::CodecZenohPushBody {

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. `std::variant` carries one of N arm bodies
// (each an imported codec type); the optional Default arm is a small
// struct that bundles the runtime tag value with the catch-all body.
struct CodecZenohPushBodyDefault {
    uint8_t tag;
    ::SCE::Generated::CodecZenohPut::CodecZenohPut body;
};
using CodecZenohPushBodyVariant = std::variant<
    ::SCE::Generated::CodecZenohPut::CodecZenohPut,
    ::SCE::Generated::CodecZenohDel::CodecZenohDel,
    CodecZenohPushBodyDefault
>;

struct CodecZenohPushBody {
    uint8_t header;
    // RFC variant-default-uniformity (Cpp): the
    // `std::in_place_index_t<N>{}` tag-type selects the arm marked
    // `<sce:arm default="true"/>` by index so a freshly-constructed
    // envelope holds that arm (not the first declared alternative
    // which `std::variant`'s default constructor would otherwise
    // pick), encoding its wire-MID for byte-exact round-trip.
    // (We construct the tag type explicitly — `std::in_place_index<N>`
    // is a variable template of type `std::in_place_index_t<N>` and
    // the brace-init form `std::in_place_index<N>{}` does not parse
    // in a member-init context.)
    CodecZenohPushBodyVariant body{std::in_place_index_t<0>{}};

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohPushBody> decode(::SCE::Forge::SceCursor& cursor) {
        // Decode fixed prefix (RFC §synth-5-B variant: fields before tag suffix).
        const std::uint8_t* raw = cursor.peek_slice(1);
        if (raw == nullptr) return std::nullopt;
        uint8_t header = raw[0];
        if (!cursor.advance(1)) return std::nullopt;
        // Dispatch on tag value into the matching arm body.
        CodecZenohPushBodyVariant body;
        switch (static_cast<uint8_t>((header >> 0) & static_cast<uint8_t>(0x1F))) {
            case 1: {
                auto _arm = ::SCE::Generated::CodecZenohPut::CodecZenohPut::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 2: {
                auto _arm = ::SCE::Generated::CodecZenohDel::CodecZenohDel::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            default: {
                auto _arm = ::SCE::Generated::CodecZenohPut::CodecZenohPut::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = CodecZenohPushBodyDefault{
                    .tag = static_cast<uint8_t>((header >> 0) & static_cast<uint8_t>(0x1F)),
                    .body = *_arm,
                };
                break;
            }
        }
        return CodecZenohPushBody{
            .header = header,
            .body = body,
        };
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    uint8_t mid() const noexcept {
        return static_cast<uint8_t>(
            (this->header >> 0) & static_cast<uint8_t>(0x1F)
        );
    }

    void set_mid(uint8_t v) noexcept {
        const uint8_t _shifted_mask =
            static_cast<uint8_t>(
                static_cast<uint8_t>(0x1F) << 0
            );
        const uint8_t _val =
            static_cast<uint8_t>(
                (static_cast<uint8_t>(v) & static_cast<uint8_t>(0x1F)) << 0
            );
        this->header = static_cast<uint8_t>(
            (this->header & static_cast<uint8_t>(~_shifted_mask)) | _val
        );
    }

    uint8_t rest() const noexcept {
        return static_cast<uint8_t>(
            (this->header >> 5) & static_cast<uint8_t>(0x07)
        );
    }

    void set_rest(uint8_t v) noexcept {
        const uint8_t _shifted_mask =
            static_cast<uint8_t>(
                static_cast<uint8_t>(0x07) << 5
            );
        const uint8_t _val =
            static_cast<uint8_t>(
                (static_cast<uint8_t>(v) & static_cast<uint8_t>(0x07)) << 5
            );
        this->header = static_cast<uint8_t>(
            (this->header & static_cast<uint8_t>(~_shifted_mask)) | _val
        );
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 2;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable sinks
    /// (e.g. `VectorSink`) are effectively infallible.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w) const noexcept {
        // Encode fixed prefix (tag field bytes are part of the prefix).
        if (auto _e = w.write_u8(header); _e) return _e;
        // Append the active arm body's encoded bytes.
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohPut::CodecZenohPut>(&body)) {
            if (auto _e = _p->encode(w); _e) return _e;
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohDel::CodecZenohDel>(&body)) {
            if (auto _e = _p->encode(w); _e) return _e;
        }
        if (auto _p = std::get_if<CodecZenohPushBodyDefault>(&body)) {
            if (auto _e = _p->body.encode(w); _e) return _e;
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

}  // namespace SCE::Generated::CodecZenohPushBody

#endif  // SCE_FORGE_CODEC_ZENOH_PUSH_BODY_H
