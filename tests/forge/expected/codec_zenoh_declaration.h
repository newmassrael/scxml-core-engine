// SCE-MAP: codec_zenoh_declaration:54

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_DECLARATION_H
#define SCE_FORGE_CODEC_ZENOH_DECLARATION_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <variant>

#include "sce/forge/codec.h"
#include "codec_zenoh_decl_kexpr.h"
#include "codec_zenoh_undecl_kexpr.h"
#include "codec_zenoh_decl_subscriber.h"
#include "codec_zenoh_undecl_subscriber.h"
#include "codec_zenoh_decl_queryable.h"
#include "codec_zenoh_undecl_queryable.h"
#include "codec_zenoh_decl_token.h"
#include "codec_zenoh_undecl_token.h"
#include "codec_zenoh_decl_final.h"

namespace SCE::Generated::CodecZenohDeclaration {

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
// codec's tag-field suffix. `std::variant` carries one of N arm bodies
// (each an imported codec type); the optional Default arm is a small
// struct that bundles the runtime tag value with the catch-all body.
struct CodecZenohDeclarationDefault {
    uint8_t tag;
    ::SCE::Generated::CodecZenohDeclFinal::CodecZenohDeclFinal body;
};
using CodecZenohDeclarationVariant = std::variant<
    ::SCE::Generated::CodecZenohDeclKexpr::CodecZenohDeclKexpr,
    ::SCE::Generated::CodecZenohUndeclKexpr::CodecZenohUndeclKexpr,
    ::SCE::Generated::CodecZenohDeclSubscriber::CodecZenohDeclSubscriber,
    ::SCE::Generated::CodecZenohUndeclSubscriber::CodecZenohUndeclSubscriber,
    ::SCE::Generated::CodecZenohDeclQueryable::CodecZenohDeclQueryable,
    ::SCE::Generated::CodecZenohUndeclQueryable::CodecZenohUndeclQueryable,
    ::SCE::Generated::CodecZenohDeclToken::CodecZenohDeclToken,
    ::SCE::Generated::CodecZenohUndeclToken::CodecZenohUndeclToken,
    ::SCE::Generated::CodecZenohDeclFinal::CodecZenohDeclFinal,
    CodecZenohDeclarationDefault
>;

struct CodecZenohDeclaration {
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
    CodecZenohDeclarationVariant body{std::in_place_index_t<8>{}};

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohDeclaration> decode(::SCE::Forge::SceCursor& cursor) {
        // Decode fixed prefix (RFC §5.B variant B1-β: fields before tag suffix).
        const std::uint8_t* raw = cursor.peek_slice(1);
        if (raw == nullptr) return std::nullopt;
        uint8_t header = raw[0];
        if (!cursor.advance(1)) return std::nullopt;
        // Dispatch on tag value into the matching arm body.
        CodecZenohDeclarationVariant body;
        switch (static_cast<uint8_t>((header >> 0) & static_cast<uint8_t>(0x1F))) {
            case 0: {
                auto _arm = ::SCE::Generated::CodecZenohDeclKexpr::CodecZenohDeclKexpr::decode(cursor, header);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 1: {
                auto _arm = ::SCE::Generated::CodecZenohUndeclKexpr::CodecZenohUndeclKexpr::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 2: {
                auto _arm = ::SCE::Generated::CodecZenohDeclSubscriber::CodecZenohDeclSubscriber::decode(cursor, header);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 3: {
                auto _arm = ::SCE::Generated::CodecZenohUndeclSubscriber::CodecZenohUndeclSubscriber::decode(cursor, header);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 4: {
                auto _arm = ::SCE::Generated::CodecZenohDeclQueryable::CodecZenohDeclQueryable::decode(cursor, header);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 5: {
                auto _arm = ::SCE::Generated::CodecZenohUndeclQueryable::CodecZenohUndeclQueryable::decode(cursor, header);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 6: {
                auto _arm = ::SCE::Generated::CodecZenohDeclToken::CodecZenohDeclToken::decode(cursor, header);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 7: {
                auto _arm = ::SCE::Generated::CodecZenohUndeclToken::CodecZenohUndeclToken::decode(cursor, header);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 26: {
                auto _arm = ::SCE::Generated::CodecZenohDeclFinal::CodecZenohDeclFinal::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            default: {
                auto _arm = ::SCE::Generated::CodecZenohDeclFinal::CodecZenohDeclFinal::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = CodecZenohDeclarationDefault{
                    .tag = static_cast<uint8_t>((header >> 0) & static_cast<uint8_t>(0x1F)),
                    .body = *_arm,
                };
                break;
            }
        }
        return CodecZenohDeclaration{
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

    bool n() const noexcept {
        return (this->header & 0x20) != 0;
    }

    void set_n(bool v) noexcept {
        if (v) {
            this->header = static_cast<uint8_t>(this->header | 0x20);
        } else {
            this->header = static_cast<uint8_t>(this->header & static_cast<uint8_t>(~0x20));
        }
    }

    bool m() const noexcept {
        return (this->header & 0x40) != 0;
    }

    void set_m(bool v) noexcept {
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

    std::vector<uint8_t> encode() const {
        // Encode fixed prefix (tag field bytes are part of the prefix).
        std::vector<uint8_t> r;
        r.reserve(275);
        r.push_back(header);
        // Append the active arm body's encoded bytes.
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohDeclKexpr::CodecZenohDeclKexpr>(&body)) {
            auto _sub = _p->encode(header);
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohUndeclKexpr::CodecZenohUndeclKexpr>(&body)) {
            auto _sub = _p->encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohDeclSubscriber::CodecZenohDeclSubscriber>(&body)) {
            auto _sub = _p->encode(header);
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohUndeclSubscriber::CodecZenohUndeclSubscriber>(&body)) {
            auto _sub = _p->encode(header);
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohDeclQueryable::CodecZenohDeclQueryable>(&body)) {
            auto _sub = _p->encode(header);
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohUndeclQueryable::CodecZenohUndeclQueryable>(&body)) {
            auto _sub = _p->encode(header);
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohDeclToken::CodecZenohDeclToken>(&body)) {
            auto _sub = _p->encode(header);
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohUndeclToken::CodecZenohUndeclToken>(&body)) {
            auto _sub = _p->encode(header);
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        if (auto _p = std::get_if<::SCE::Generated::CodecZenohDeclFinal::CodecZenohDeclFinal>(&body)) {
            auto _sub = _p->encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        if (auto _p = std::get_if<CodecZenohDeclarationDefault>(&body)) {
            auto _sub = _p->body.encode();
            r.insert(r.end(), _sub.begin(), _sub.end());
        }
        return r;
    }
};

}  // namespace SCE::Generated::CodecZenohDeclaration

#endif  // SCE_FORGE_CODEC_ZENOH_DECLARATION_H
