// SCE-MAP: codec_variant_dispatch:8

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_VARIANT_DISPATCH_H
#define SCE_FORGE_CODEC_VARIANT_DISPATCH_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <variant>

#include "sce/forge/codec.h"
#include "codec_variant_session_open.h"
#include "codec_variant_session_close.h"

namespace SCE::Generated::CodecVariantDispatch {

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
// codec's tag-field suffix. `std::variant` carries one of N arm bodies
// (each an imported codec type); the optional Default arm is a small
// struct that bundles the runtime tag value with the catch-all body.
struct CodecVariantDispatchDefault {
    uint8_t tag;
    ::SCE::Generated::CodecVariantSessionClose::CodecVariantSessionClose body;
};
using CodecVariantDispatchVariant = std::variant<
    ::SCE::Generated::CodecVariantSessionOpen::CodecVariantSessionOpen,
    ::SCE::Generated::CodecVariantSessionClose::CodecVariantSessionClose,
    CodecVariantDispatchDefault
>;

struct CodecVariantDispatch {
    uint8_t msg_id;
    // RFC variant-default-uniformity Atomic β-cpp: the
    // `std::in_place_index<N>{}` tag selects the arm marked
    // `<sce:arm default="true"/>` by index so a freshly-constructed
    // envelope holds that arm (not the first declared alternative
    // which `std::variant`'s default constructor would otherwise
    // pick), encoding its wire-MID for byte-exact round-trip.
    CodecVariantDispatchVariant body{std::in_place_index<1>{}};

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecVariantDispatch> decode(::SCE::Forge::SceCursor& cursor) {
        // Decode fixed prefix (RFC §5.B variant B1-β: fields before tag suffix).
        const std::uint8_t* raw = cursor.peek_slice(1);
        if (raw == nullptr) return std::nullopt;
        uint8_t msg_id = raw[0];
        if (!cursor.advance(1)) return std::nullopt;
        // Dispatch on tag value into the matching arm body.
        CodecVariantDispatchVariant body;
        switch (msg_id) {
            case 1: {
                auto _arm = ::SCE::Generated::CodecVariantSessionOpen::CodecVariantSessionOpen::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 2: {
                auto _arm = ::SCE::Generated::CodecVariantSessionClose::CodecVariantSessionClose::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            default: {
                auto _arm = ::SCE::Generated::CodecVariantSessionClose::CodecVariantSessionClose::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = CodecVariantDispatchDefault{
                    .tag = msg_id,
                    .body = *_arm,
                };
                break;
            }
        }
        return CodecVariantDispatch{
            .msg_id = msg_id,
            .body = body,
        };
    }

    std::vector<uint8_t> encode() const {
        // Encode fixed prefix (tag field bytes are part of the prefix).
        std::vector<uint8_t> r;
        r.reserve(3);
        r.push_back(msg_id);
        // Append the active arm body's encoded bytes.
        if (auto _p = std::get_if<::SCE::Generated::CodecVariantSessionOpen::CodecVariantSessionOpen>(&body)) {
            auto _sub = _p->encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecVariantSessionClose::CodecVariantSessionClose>(&body)) {
            auto _sub = _p->encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        if (auto _p = std::get_if<CodecVariantDispatchDefault>(&body)) {
            auto _sub = _p->body.encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        return r;
    }
};

}  // namespace SCE::Generated::CodecVariantDispatch

#endif  // SCE_FORGE_CODEC_VARIANT_DISPATCH_H
