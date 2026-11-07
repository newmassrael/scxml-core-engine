// SCE-MAP: codec_zenoh_network_envelope:60 :: _forge_body

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
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_zenoh_interest.h"
#include "codec_zenoh_response_final.h"
#include "codec_zenoh_response.h"
#include "codec_zenoh_request.h"
#include "codec_zenoh_push.h"
#include "codec_zenoh_declare.h"
#include "codec_zenoh_oam.h"

namespace SCE::Generated::CodecZenohNetworkEnvelope {

// RFC §synth-5-B variant primitive: discriminated-union body for the
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

/// The elements a value of `CodecZenohNetworkEnvelope` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_body {
void body() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.body())) -> decltype(s.body()) {
    return s.body();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(body(s))) -> decltype(body(s)) {
    return body(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_body
/// The `body` element of a source.
inline constexpr detail_body::Read body{};
}  // namespace source

/// Whether `D` is the catch-all arm of a source's body: the tag it holds,
/// and its body, a source of the catch-all's codec.
template <class D, class = void>
struct is_default_arm : std::false_type {};
template <class D>
struct is_default_arm<D, std::void_t<decltype(std::declval<const D&>().tag), decltype(std::declval<const D&>().body)>>
    : std::conjunction<std::is_convertible<decltype(std::declval<const D&>().tag), std::uint64_t>,
                       ::SCE::Generated::CodecZenohOam::is_source<std::decay_t<decltype(std::declval<const D&>().body)>>> {};

/// Whether `R` is a body a source may present: the arm it holds, of a
/// `std::variant` of a source of each arm's codec in turn.
template <class R, class = void>
struct is_body : std::false_type {};
template <class R>
struct is_body<R, std::enable_if_t<::SCE::Forge::Detail::IsVariantOf<R, 8>::value>>
    : std::conjunction<::SCE::Generated::CodecZenohInterest::is_source<std::variant_alternative_t<0, R>>, ::SCE::Generated::CodecZenohResponseFinal::is_source<std::variant_alternative_t<1, R>>, ::SCE::Generated::CodecZenohResponse::is_source<std::variant_alternative_t<2, R>>, ::SCE::Generated::CodecZenohRequest::is_source<std::variant_alternative_t<3, R>>, ::SCE::Generated::CodecZenohPush::is_source<std::variant_alternative_t<4, R>>, ::SCE::Generated::CodecZenohDeclare::is_source<std::variant_alternative_t<5, R>>, ::SCE::Generated::CodecZenohOam::is_source<std::variant_alternative_t<6, R>>, is_default_arm<std::variant_alternative_t<7, R>>> {};

/// Whether `S` presents every element of `CodecZenohNetworkEnvelope`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::body(std::declval<const S&>()))>>
    : std::conjunction<is_body<std::decay_t<decltype(source::body(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(source::body(std::declval<const S&>())) && ::SCE::Generated::CodecZenohInterest::is_nothrow_source<std::variant_alternative_t<0, std::decay_t<decltype(source::body(std::declval<const S&>()))>>>::value && ::SCE::Generated::CodecZenohResponseFinal::is_nothrow_source<std::variant_alternative_t<1, std::decay_t<decltype(source::body(std::declval<const S&>()))>>>::value && ::SCE::Generated::CodecZenohResponse::is_nothrow_source<std::variant_alternative_t<2, std::decay_t<decltype(source::body(std::declval<const S&>()))>>>::value && ::SCE::Generated::CodecZenohRequest::is_nothrow_source<std::variant_alternative_t<3, std::decay_t<decltype(source::body(std::declval<const S&>()))>>>::value && ::SCE::Generated::CodecZenohPush::is_nothrow_source<std::variant_alternative_t<4, std::decay_t<decltype(source::body(std::declval<const S&>()))>>>::value && ::SCE::Generated::CodecZenohDeclare::is_nothrow_source<std::variant_alternative_t<5, std::decay_t<decltype(source::body(std::declval<const S&>()))>>>::value && ::SCE::Generated::CodecZenohOam::is_nothrow_source<std::variant_alternative_t<6, std::decay_t<decltype(source::body(std::declval<const S&>()))>>>::value && ::SCE::Generated::CodecZenohOam::is_nothrow_source<std::decay_t<decltype(std::declval<const std::variant_alternative_t<7, std::decay_t<decltype(source::body(std::declval<const S&>()))>>&>().body)>>::value> {};

/// The encode of `CodecZenohNetworkEnvelope` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecZenohNetworkEnvelope: body");
        const auto& _r_body = source::body(src);
        switch (_r_body.index()) {
            case 0:
                if (auto _e = ::SCE::Generated::CodecZenohInterest::Wire::encode_source(*std::get_if<0>(&_r_body), w); _e) return _e;
                break;
            case 1:
                if (auto _e = ::SCE::Generated::CodecZenohResponseFinal::Wire::encode_source(*std::get_if<1>(&_r_body), w); _e) return _e;
                break;
            case 2:
                if (auto _e = ::SCE::Generated::CodecZenohResponse::Wire::encode_source(*std::get_if<2>(&_r_body), w); _e) return _e;
                break;
            case 3:
                if (auto _e = ::SCE::Generated::CodecZenohRequest::Wire::encode_source(*std::get_if<3>(&_r_body), w); _e) return _e;
                break;
            case 4:
                if (auto _e = ::SCE::Generated::CodecZenohPush::Wire::encode_source(*std::get_if<4>(&_r_body), w); _e) return _e;
                break;
            case 5:
                if (auto _e = ::SCE::Generated::CodecZenohDeclare::Wire::encode_source(*std::get_if<5>(&_r_body), w); _e) return _e;
                break;
            case 6:
                if (auto _e = ::SCE::Generated::CodecZenohOam::Wire::encode_source(*std::get_if<6>(&_r_body), w); _e) return _e;
                break;
            case 7:
                if (auto _e = ::SCE::Generated::CodecZenohOam::Wire::encode_source(std::get_if<7>(&_r_body)->body, w); _e) return _e;
                break;
            default: break;
        }
        return std::nullopt;
    }
};

struct CodecZenohNetworkEnvelope {
    CodecZenohNetworkEnvelope() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecZenohNetworkEnvelope(CodecZenohNetworkEnvelopeVariant body)
        : body_(std::move(body)) {}
    /// The arm the value holds, each arm's body the value of its codec.
    [[nodiscard]] const CodecZenohNetworkEnvelopeVariant& body() const noexcept { return body_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohNetworkEnvelope> decode(::SCE::Forge::SceCursor& cursor) {
        // RFC §synth-5-B peek-byte / streaming-prefix:
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
                    /*tag=*/static_cast<uint8_t>((_peek >> 0) & static_cast<uint8_t>(0x1F)),
                    /*body=*/*_arm,
                };
                break;
            }
        }
        CodecZenohNetworkEnvelope _decoded{
            /*body=*/std::move(body),
        };
        return _decoded;
    }


    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable sinks
    /// (e.g. `VectorSink`) are effectively infallible.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w) const noexcept {
        // The encode over any source of the codec, this value one of them,
        // handed the inputs the caller hands this encode.
        return Wire::encode_source(*this, w);
    }

    /// Heap-backed convenience facade. It reserves nothing, since the
    /// largest frame passes `u32` (docs/adr/0077 §7), and delegates to
    /// `encode` over a `VectorSink`. Returns the
    /// freshly-encoded byte vector. Callers targeting zero-alloc hot
    /// paths should call `encode` directly against a caller-owned sink.
    [[nodiscard]] std::vector<std::uint8_t> encode_to_vec() const {
        std::vector<std::uint8_t> _sce_v;
        ::SCE::Forge::VectorSink _sce_sink(_sce_v);
        (void)encode(_sce_sink);
        return _sce_v;
    }

private:
    CodecZenohNetworkEnvelopeVariant body_{std::in_place_index_t<6>{}};
};

}  // namespace SCE::Generated::CodecZenohNetworkEnvelope

#endif  // SCE_FORGE_CODEC_ZENOH_NETWORK_ENVELOPE_H
