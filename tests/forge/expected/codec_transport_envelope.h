// SCE-MAP: codec_transport_envelope:68

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_TRANSPORT_ENVELOPE_H
#define SCE_FORGE_CODEC_TRANSPORT_ENVELOPE_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <variant>

#include "sce/forge/codec.h"
#include "codec_zenoh_init_body.h"
#include "codec_zenoh_open_body.h"
#include "codec_zenoh_close.h"
#include "codec_zenoh_keep_alive.h"
#include "codec_zenoh_frame.h"
#include "codec_zenoh_fragment.h"
#include "codec_zenoh_join.h"

namespace SCE::Generated::CodecTransportEnvelope {

// RFC §5.B variant primitive: discriminated-union body for the
// codec's tag-field suffix. `std::variant` carries one of N arm bodies
// (each an imported codec type); the optional Default arm is a small
// struct that bundles the runtime tag value with the catch-all body.
struct CodecTransportEnvelopeDefault {
    uint8_t tag;
    ::SCE::Generated::CodecZenohClose::CodecZenohClose body;
};
using CodecTransportEnvelopeVariant = std::variant<
    ::SCE::Generated::CodecZenohInitBody::CodecZenohInitBody,
    ::SCE::Generated::CodecZenohOpenBody::CodecZenohOpenBody,
    ::SCE::Generated::CodecZenohClose::CodecZenohClose,
    ::SCE::Generated::CodecZenohKeepAlive::CodecZenohKeepAlive,
    ::SCE::Generated::CodecZenohFrame::CodecZenohFrame,
    ::SCE::Generated::CodecZenohFragment::CodecZenohFragment,
    ::SCE::Generated::CodecZenohJoin::CodecZenohJoin,
    CodecTransportEnvelopeDefault
>;

struct CodecTransportEnvelope {
    uint8_t header;
    // RFC variant-default-uniformity Atomic β-cpp: the
    // `std::in_place_index_t<N>{}` tag-type selects the arm marked
    // `<sce:arm default="true"/>` by index so a freshly-constructed
    // envelope holds that arm (not the first declared alternative
    // which `std::variant`'s default constructor would otherwise
    // pick), encoding its wire-MID for byte-exact round-trip.
    // (We construct the tag type explicitly — `std::in_place_index<N>`
    // is a variable template of type `std::in_place_index_t<N>` and
    // the brace-init form `std::in_place_index<N>{}` does not parse
    // in a member-init context.)
    CodecTransportEnvelopeVariant body{std::in_place_index_t<2>{}};

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecTransportEnvelope> decode(::SCE::Forge::SceCursor& cursor) {
        // Decode fixed prefix (RFC §5.B variant: fields before tag suffix).
        const std::uint8_t* raw = cursor.peek_slice(1);
        if (raw == nullptr) return std::nullopt;
        uint8_t header = raw[0];
        if (!cursor.advance(1)) return std::nullopt;
        // Dispatch on tag value into the matching arm body.
        CodecTransportEnvelopeVariant body;
        switch (static_cast<uint8_t>((header >> 0) & static_cast<uint8_t>(0x1F))) {
            case 1: {
                auto _arm = ::SCE::Generated::CodecZenohInitBody::CodecZenohInitBody::decode(cursor, static_cast<std::uint8_t>((header >> 6) & 0x1), static_cast<std::uint8_t>((header >> 5) & 0x1));
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 2: {
                auto _arm = ::SCE::Generated::CodecZenohOpenBody::CodecZenohOpenBody::decode(cursor, static_cast<std::uint8_t>((header >> 5) & 0x1));
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 3: {
                auto _arm = ::SCE::Generated::CodecZenohClose::CodecZenohClose::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 4: {
                auto _arm = ::SCE::Generated::CodecZenohKeepAlive::CodecZenohKeepAlive::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 5: {
                auto _arm = ::SCE::Generated::CodecZenohFrame::CodecZenohFrame::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 6: {
                auto _arm = ::SCE::Generated::CodecZenohFragment::CodecZenohFragment::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 7: {
                auto _arm = ::SCE::Generated::CodecZenohJoin::CodecZenohJoin::decode(cursor, static_cast<std::uint8_t>((header >> 6) & 0x1));
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            default: {
                auto _arm = ::SCE::Generated::CodecZenohClose::CodecZenohClose::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = CodecTransportEnvelopeDefault{
                    .tag = static_cast<uint8_t>((header >> 0) & static_cast<uint8_t>(0x1F)),
                    .body = *_arm,
                };
                break;
            }
        }
        return CodecTransportEnvelope{
            .header = header,
            .body = body,
        };
    }

    // RFC §5.B flags primitive: per-bit-range accessors.
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

    bool a() const noexcept {
        return (this->header & 0x20) != 0;
    }

    void set_a(bool v) noexcept {
        if (v) {
            this->header = static_cast<uint8_t>(this->header | 0x20);
        } else {
            this->header = static_cast<uint8_t>(this->header & static_cast<uint8_t>(~0x20));
        }
    }

    bool s() const noexcept {
        return (this->header & 0x40) != 0;
    }

    void set_s(bool v) noexcept {
        if (v) {
            this->header = static_cast<uint8_t>(this->header | 0x40);
        } else {
            this->header = static_cast<uint8_t>(this->header & static_cast<uint8_t>(~0x40));
        }
    }

    bool z() const noexcept {
        return (this->header & 0x80) != 0;
    }

    void set_z(bool v) noexcept {
        if (v) {
            this->header = static_cast<uint8_t>(this->header | 0x80);
        } else {
            this->header = static_cast<uint8_t>(this->header & static_cast<uint8_t>(~0x80));
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 65547;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable sinks
    /// (e.g. `VectorSink`) are effectively infallible.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w) const noexcept {
        // Encode fixed prefix (tag field bytes are part of the prefix).
        if (auto _e = w.write_u8(header); _e) return _e;
        // Append the active arm body's encoded bytes.
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohInitBody::CodecZenohInitBody>(&body)) {
            if (auto _e = _p->encode(w, static_cast<std::uint8_t>((this->header >> 6) & 0x1), static_cast<std::uint8_t>((this->header >> 5) & 0x1)); _e) return _e;
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohOpenBody::CodecZenohOpenBody>(&body)) {
            if (auto _e = _p->encode(w, static_cast<std::uint8_t>((this->header >> 5) & 0x1)); _e) return _e;
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohClose::CodecZenohClose>(&body)) {
            if (auto _e = _p->encode(w); _e) return _e;
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohKeepAlive::CodecZenohKeepAlive>(&body)) {
            if (auto _e = _p->encode(w); _e) return _e;
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohFrame::CodecZenohFrame>(&body)) {
            if (auto _e = _p->encode(w); _e) return _e;
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohFragment::CodecZenohFragment>(&body)) {
            if (auto _e = _p->encode(w); _e) return _e;
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohJoin::CodecZenohJoin>(&body)) {
            if (auto _e = _p->encode(w, static_cast<std::uint8_t>((this->header >> 6) & 0x1)); _e) return _e;
        }
        if (auto _p = std::get_if<CodecTransportEnvelopeDefault>(&body)) {
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

}  // namespace SCE::Generated::CodecTransportEnvelope

#endif  // SCE_FORGE_CODEC_TRANSPORT_ENVELOPE_H
