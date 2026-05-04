// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_NETWORK_ENVELOPE_H
#define SCE_FORGE_CODEC_ZENOH_NETWORK_ENVELOPE_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <variant>

#include "sce/forge/codec.h"
#include "codec_zenoh_interest.h"
#include "codec_zenoh_response_final.h"
#include "codec_zenoh_response.h"
#include "codec_zenoh_request.h"
#include "codec_zenoh_push.h"
#include "codec_zenoh_declare.h"
#include "codec_zenoh_oam.h"

namespace SCE::Generated::CodecZenohNetworkEnvelope {

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
// codec's tag-field suffix. `std::variant` carries one of N arm bodies
// (each an imported codec type); the optional Default arm is a small
// struct that bundles the runtime tag value with the catch-all body.
struct CodecZenohNetworkEnvelopeDefault {
    uint8_t tag;
    ::SCE::Generated::CodecZenohOam::CodecZenohOam body;
};
using CodecZenohNetworkEnvelopeVariant = std::variant<
    ::SCE::Generated::CodecZenohInterest::CodecZenohInterest,
    ::SCE::Generated::CodecZenohResponseFinal::CodecZenohResponseFinal,
    ::SCE::Generated::CodecZenohResponse::CodecZenohResponse,
    ::SCE::Generated::CodecZenohRequest::CodecZenohRequest,
    ::SCE::Generated::CodecZenohPush::CodecZenohPush,
    ::SCE::Generated::CodecZenohDeclare::CodecZenohDeclare,
    ::SCE::Generated::CodecZenohOam::CodecZenohOam,
    CodecZenohNetworkEnvelopeDefault
>;

struct CodecZenohNetworkEnvelope {
    CodecZenohNetworkEnvelopeVariant body;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohNetworkEnvelope> decode(::SCE::Forge::SceCursor& cursor) {
        // RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
        // streaming prefix decode (variable-length fields supported via
        // per-field present_if/tlv-chain/embed/repeat helpers). Peek-byte
        // mode additionally peeks the cursor's next byte for variant tag
        // without advancing — arm body decoder reads it as its own
        // header byte.
        const std::uint8_t* _peek_raw = cursor.peek_slice(1);
        if (_peek_raw == nullptr) return std::nullopt;
        const std::uint8_t _peek = _peek_raw[0];
        // Dispatch on tag value into the matching arm body.
        CodecZenohNetworkEnvelopeVariant body;
        switch (static_cast<uint8_t>((_peek >> 0) & static_cast<uint8_t>(0x1F))) {
            case 25: {
                auto _arm = ::SCE::Generated::CodecZenohInterest::CodecZenohInterest::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 26: {
                auto _arm = ::SCE::Generated::CodecZenohResponseFinal::CodecZenohResponseFinal::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 27: {
                auto _arm = ::SCE::Generated::CodecZenohResponse::CodecZenohResponse::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 28: {
                auto _arm = ::SCE::Generated::CodecZenohRequest::CodecZenohRequest::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 29: {
                auto _arm = ::SCE::Generated::CodecZenohPush::CodecZenohPush::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 30: {
                auto _arm = ::SCE::Generated::CodecZenohDeclare::CodecZenohDeclare::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 31: {
                auto _arm = ::SCE::Generated::CodecZenohOam::CodecZenohOam::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            default: {
                auto _arm = ::SCE::Generated::CodecZenohOam::CodecZenohOam::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = CodecZenohNetworkEnvelopeDefault{
                    .tag = static_cast<uint8_t>((_peek >> 0) & static_cast<uint8_t>(0x1F)),
                    .body = *_arm,
                };
                break;
            }
        }
        return CodecZenohNetworkEnvelope{
            .body = body,
        };
    }

    std::vector<uint8_t> encode() const {
        // RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
        // streaming prefix encode. Peek-byte mode: arm body's encode
        // prepends its own header byte (which the decoder peeked); no
        // separate tag byte here. Streaming-prefix mode (own-field):
        // carrier is part of the prefix fields and emits via the same
        // per-field path.
        std::vector<uint8_t> r;
        r.reserve(1218);
        // Append the active arm body's encoded bytes.
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohInterest::CodecZenohInterest>(&body)) {
            auto _sub = _p->encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohResponseFinal::CodecZenohResponseFinal>(&body)) {
            auto _sub = _p->encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohResponse::CodecZenohResponse>(&body)) {
            auto _sub = _p->encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohRequest::CodecZenohRequest>(&body)) {
            auto _sub = _p->encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohPush::CodecZenohPush>(&body)) {
            auto _sub = _p->encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohDeclare::CodecZenohDeclare>(&body)) {
            auto _sub = _p->encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohOam::CodecZenohOam>(&body)) {
            auto _sub = _p->encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        if (auto _p = std::get_if<CodecZenohNetworkEnvelopeDefault>(&body)) {
            auto _sub = _p->body.encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        return r;
    }
};

}  // namespace SCE::Generated::CodecZenohNetworkEnvelope

#endif  // SCE_FORGE_CODEC_ZENOH_NETWORK_ENVELOPE_H
