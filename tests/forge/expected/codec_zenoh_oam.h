// SCE-MAP: codec_zenoh_oam:56 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_OAM_H
#define SCE_FORGE_CODEC_ZENOH_OAM_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <variant>

#include "sce/forge/codec.h"
#include "codec_zenoh_ext_entry.h"
#include "codec_zenoh_ext_unit.h"
#include "codec_zenoh_ext_zint.h"
#include "codec_zenoh_ext_zbuf.h"

namespace SCE::Generated::CodecZenohOam {

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. `std::variant` carries one of N arm bodies
// (each an imported codec type); the optional Default arm is a small
// struct that bundles the runtime tag value with the catch-all body.
struct CodecZenohOamDefault {
    uint8_t tag;
    ::SCE::Generated::CodecZenohExtUnit::CodecZenohExtUnit body;
};
using CodecZenohOamVariant = std::variant<
    ::SCE::Generated::CodecZenohExtUnit::CodecZenohExtUnit,
    ::SCE::Generated::CodecZenohExtZint::CodecZenohExtZint,
    ::SCE::Generated::CodecZenohExtZbuf::CodecZenohExtZbuf,
    CodecZenohOamDefault
>;

struct CodecZenohOam {
    uint8_t header{0x1fu};
    uint16_t id;
    std::optional<std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>> extensions;
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
    CodecZenohOamVariant body{std::in_place_index_t<0>{}};

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohOam> decode(::SCE::Forge::SceCursor& cursor) {
        // RFC §synth-5-B peek-byte / streaming-prefix:
        // streaming prefix decode (variable-length fields supported via
        // per-field present_if/tlv-chain/embed/repeat helpers). Peek-byte
        // mode additionally peeks the cursor's next byte for variant tag
        // without advancing — arm body decoder reads it as its own
        // header byte.
        uint8_t header;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            header = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        auto id_opt = cursor.read_vle_u16();
        if (!id_opt.has_value()) return std::nullopt;
        auto id = static_cast<std::uint16_t>(*id_opt);
        std::optional<std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>> extensions;
        if ((header & 0x80) != 0) {
            std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry> _list;
            _list.reserve(4);
            bool _more = false;
            for (std::size_t _i = 0; _i < 4; ++_i) {
                if (cursor.remaining() == 0) break;
                auto _elem = ::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry::decode(cursor);
                if (!_elem.has_value()) return std::nullopt;
                _more = _elem->z();
                _list.push_back(*_elem);
                if (!_more) break;
            }
            if (_more) return std::nullopt;
            extensions = std::move(_list);
        }
        // Dispatch on tag value into the matching arm body.
        CodecZenohOamVariant body;
        switch (static_cast<uint8_t>((header >> 5) & static_cast<uint8_t>(0x03))) {
            case 0: {
                auto _arm = ::SCE::Generated::CodecZenohExtUnit::CodecZenohExtUnit::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 1: {
                auto _arm = ::SCE::Generated::CodecZenohExtZint::CodecZenohExtZint::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 2: {
                auto _arm = ::SCE::Generated::CodecZenohExtZbuf::CodecZenohExtZbuf::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            default: {
                auto _arm = ::SCE::Generated::CodecZenohExtUnit::CodecZenohExtUnit::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = CodecZenohOamDefault{
                    .tag = static_cast<uint8_t>((header >> 5) & static_cast<uint8_t>(0x03)),
                    .body = *_arm,
                };
                break;
            }
        }
        return CodecZenohOam{
            .header = header,
            .id = id,
            .extensions = extensions,
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

    uint8_t enc() const noexcept {
        return static_cast<uint8_t>(
            (this->header >> 5) & static_cast<uint8_t>(0x03)
        );
    }

    void set_enc(uint8_t v) noexcept {
        const uint8_t _shifted_mask =
            static_cast<uint8_t>(
                static_cast<uint8_t>(0x03) << 5
            );
        const uint8_t _val =
            static_cast<uint8_t>(
                (static_cast<uint8_t>(v) & static_cast<uint8_t>(0x03)) << 5
            );
        this->header = static_cast<uint8_t>(
            (this->header & static_cast<uint8_t>(~_shifted_mask)) | _val
        );
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
    static constexpr std::size_t MAX_ENCODED_BYTES = 45;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable sinks
    /// (e.g. `VectorSink`) are effectively infallible.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w) const noexcept {
        // RFC §synth-5-B peek-byte / streaming-prefix:
        // streaming prefix encode. Peek-byte mode: arm body's encode
        // prepends its own header byte (which the decoder peeked); no
        // separate tag byte here. Streaming-prefix mode (own-field):
        // carrier is part of the prefix fields and emits via the same
        // per-field path.
        if (auto _e = w.write_u8(header); _e) return _e;
        if (auto _e = w.write_vle_u16(static_cast<std::uint16_t>(id)); _e) return _e;
        if (this->extensions.has_value()) {
            for (const auto& _e : *this->extensions) {
                if (auto _se = _e.encode(w); _se) return _se;
            }
        }
        // Append the active arm body's encoded bytes.
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohExtUnit::CodecZenohExtUnit>(&body)) {
            if (auto _e = _p->encode(w); _e) return _e;
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohExtZint::CodecZenohExtZint>(&body)) {
            if (auto _e = _p->encode(w); _e) return _e;
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohExtZbuf::CodecZenohExtZbuf>(&body)) {
            if (auto _e = _p->encode(w); _e) return _e;
        }
        if (auto _p = std::get_if<CodecZenohOamDefault>(&body)) {
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

}  // namespace SCE::Generated::CodecZenohOam

#endif  // SCE_FORGE_CODEC_ZENOH_OAM_H
