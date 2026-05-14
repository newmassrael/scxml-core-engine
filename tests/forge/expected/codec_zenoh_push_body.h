// SCE-MAP: codec_zenoh_push_body:30

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

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
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
    CodecZenohPushBodyVariant body;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohPushBody> decode(::SCE::Forge::SceCursor& cursor) {
        // Decode fixed prefix (RFC §5.B variant B1-β: fields before tag suffix).
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

    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors.
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

    std::vector<uint8_t> encode() const {
        // Encode fixed prefix (tag field bytes are part of the prefix).
        std::vector<uint8_t> r;
        r.reserve(2);
        r.push_back(header);
        // Append the active arm body's encoded bytes.
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohPut::CodecZenohPut>(&body)) {
            auto _sub = _p->encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohDel::CodecZenohDel>(&body)) {
            auto _sub = _p->encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        if (auto _p = std::get_if<CodecZenohPushBodyDefault>(&body)) {
            auto _sub = _p->body.encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        return r;
    }
};

}  // namespace SCE::Generated::CodecZenohPushBody

#endif  // SCE_FORGE_CODEC_ZENOH_PUSH_BODY_H
