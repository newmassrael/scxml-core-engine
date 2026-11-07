// SCE-MAP: codec_variant_peek_basic:29 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_VARIANT_PEEK_BASIC_H
#define SCE_FORGE_CODEC_VARIANT_PEEK_BASIC_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <variant>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_peek_arm_a.h"
#include "codec_peek_arm_b.h"

namespace SCE::Generated::CodecVariantPeekBasic {

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. `std::variant` carries one of N arm bodies
// (each an imported codec type); the optional Default arm is a small
// struct that bundles the runtime tag value with the catch-all body.
using CodecVariantPeekBasicVariant = std::variant<
    ::SCE::Generated::CodecPeekArmA::CodecPeekArmA,
    ::SCE::Generated::CodecPeekArmB::CodecPeekArmB
>;

/// The elements a value of `CodecVariantPeekBasic` presents, each read by name: the
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

/// Whether `R` is a body a source may present: the arm it holds, of a
/// `std::variant` of a source of each arm's codec in turn.
template <class R, class = void>
struct is_body : std::false_type {};
template <class R>
struct is_body<R, std::enable_if_t<::SCE::Forge::Detail::IsVariantOf<R, 2>::value>>
    : std::conjunction<::SCE::Generated::CodecPeekArmA::is_source<std::variant_alternative_t<0, R>>, ::SCE::Generated::CodecPeekArmB::is_source<std::variant_alternative_t<1, R>>> {};

/// Whether `S` presents every element of `CodecVariantPeekBasic`, each convertible to
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
    : std::bool_constant<noexcept(source::body(std::declval<const S&>())) && ::SCE::Generated::CodecPeekArmA::is_nothrow_source<std::variant_alternative_t<0, std::decay_t<decltype(source::body(std::declval<const S&>()))>>>::value && ::SCE::Generated::CodecPeekArmB::is_nothrow_source<std::variant_alternative_t<1, std::decay_t<decltype(source::body(std::declval<const S&>()))>>>::value> {};

/// The encode of `CodecVariantPeekBasic` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecVariantPeekBasic: body");
        const auto& _r_body = source::body(src);
        switch (_r_body.index()) {
            case 0:
                if (auto _e = ::SCE::Generated::CodecPeekArmA::Wire::encode_source(*std::get_if<0>(&_r_body), w); _e) return _e;
                break;
            case 1:
                if (auto _e = ::SCE::Generated::CodecPeekArmB::Wire::encode_source(*std::get_if<1>(&_r_body), w); _e) return _e;
                break;
            default: break;
        }
        return std::nullopt;
    }
};

struct CodecVariantPeekBasic {
    CodecVariantPeekBasic() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecVariantPeekBasic(CodecVariantPeekBasicVariant body)
        : body_(std::move(body)) {}
    /// The arm the value holds, each arm's body the value of its codec.
    [[nodiscard]] const CodecVariantPeekBasicVariant& body() const noexcept { return body_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecVariantPeekBasic> decode(::SCE::Forge::SceCursor& cursor) {
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
        CodecVariantPeekBasicVariant body;
        switch (static_cast<uint8_t>((_peek >> 0) & static_cast<uint8_t>(0x01))) {
            case 0: {
                auto _arm = ::SCE::Generated::CodecPeekArmA::CodecPeekArmA::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 1: {
                auto _arm = ::SCE::Generated::CodecPeekArmB::CodecPeekArmB::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            default: {
                // codec/variant-arm-unreachable rejected this case at parse time.
                return std::nullopt;
            }
        }
        CodecVariantPeekBasic _decoded{
            /*body=*/std::move(body),
        };
        return _decoded;
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 3;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable sinks
    /// (e.g. `VectorSink`) are effectively infallible.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w) const noexcept {
        // The encode over any source of the codec, this value one of them,
        // handed the inputs the caller hands this encode.
        return Wire::encode_source(*this, w);
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

private:
    CodecVariantPeekBasicVariant body_{std::in_place_index_t<0>{}};
};

}  // namespace SCE::Generated::CodecVariantPeekBasic

#endif  // SCE_FORGE_CODEC_VARIANT_PEEK_BASIC_H
