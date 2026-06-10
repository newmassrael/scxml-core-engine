// SCE-MAP: codec_zenoh_ext_entry:52

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_EXT_ENTRY_H
#define SCE_FORGE_CODEC_ZENOH_EXT_ENTRY_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <variant>

#include "sce/forge/codec.h"
#include "codec_zenoh_ext_unit.h"
#include "codec_zenoh_ext_zint.h"
#include "codec_zenoh_ext_zbuf.h"

namespace SCE::Generated::CodecZenohExtEntry {

// RFC §5.B variant primitive: discriminated-union body for the
// codec's tag-field suffix. `std::variant` carries one of N arm bodies
// (each an imported codec type); the optional Default arm is a small
// struct that bundles the runtime tag value with the catch-all body.
struct CodecZenohExtEntryDefault {
    uint8_t tag;
    ::SCE::Generated::CodecZenohExtUnit::CodecZenohExtUnit body;
};
using CodecZenohExtEntryVariant = std::variant<
    ::SCE::Generated::CodecZenohExtUnit::CodecZenohExtUnit,
    ::SCE::Generated::CodecZenohExtZint::CodecZenohExtZint,
    ::SCE::Generated::CodecZenohExtZbuf::CodecZenohExtZbuf,
    CodecZenohExtEntryDefault
>;

struct CodecZenohExtEntry {
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
    CodecZenohExtEntryVariant body{std::in_place_index_t<0>{}};

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohExtEntry> decode(::SCE::Forge::SceCursor& cursor) {
        // Decode fixed prefix (RFC §5.B variant: fields before tag suffix).
        const std::uint8_t* raw = cursor.peek_slice(1);
        if (raw == nullptr) return std::nullopt;
        uint8_t header = raw[0];
        if (!cursor.advance(1)) return std::nullopt;
        // Dispatch on tag value into the matching arm body.
        CodecZenohExtEntryVariant body;
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
                body = CodecZenohExtEntryDefault{
                    .tag = static_cast<uint8_t>((header >> 5) & static_cast<uint8_t>(0x03)),
                    .body = *_arm,
                };
                break;
            }
        }
        return CodecZenohExtEntry{
            .header = header,
            .body = body,
        };
    }

    // RFC §5.B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    uint8_t ext_id() const noexcept {
        return static_cast<uint8_t>(
            (this->header >> 0) & static_cast<uint8_t>(0x0F)
        );
    }

    void set_ext_id(uint8_t v) noexcept {
        const uint8_t _shifted_mask =
            static_cast<uint8_t>(
                static_cast<uint8_t>(0x0F) << 0
            );
        const uint8_t _val =
            static_cast<uint8_t>(
                (static_cast<uint8_t>(v) & static_cast<uint8_t>(0x0F)) << 0
            );
        this->header = static_cast<uint8_t>(
            (this->header & static_cast<uint8_t>(~_shifted_mask)) | _val
        );
    }

    bool m() const noexcept {
        return (this->header & 0x10) != 0;
    }

    void set_m(bool v) noexcept {
        if (v) {
            this->header = static_cast<uint8_t>(this->header | 0x10);
        } else {
            this->header = static_cast<uint8_t>(this->header & static_cast<uint8_t>(~0x10));
        }
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
    static constexpr std::size_t MAX_ENCODED_BYTES = 43;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable sinks
    /// (e.g. `VectorSink`) are effectively infallible.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w) const noexcept {
        // Encode fixed prefix (tag field bytes are part of the prefix).
        if (auto _e = w.write_u8(header); _e) return _e;
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
        if (auto _p = std::get_if<CodecZenohExtEntryDefault>(&body)) {
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

}  // namespace SCE::Generated::CodecZenohExtEntry

#endif  // SCE_FORGE_CODEC_ZENOH_EXT_ENTRY_H
