// SCE-MAP: codec_length_ref_uint16_be:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_LENGTH_REF_UINT16_BE_H
#define SCE_FORGE_CODEC_LENGTH_REF_UINT16_BE_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecLengthRefUint16Be {

/// The elements a value of `CodecLengthRefUint16Be` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_payload {
void payload() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.payload())) -> decltype(s.payload()) {
    return s.payload();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(payload(s))) -> decltype(payload(s)) {
    return payload(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_payload
/// The `payload` element of a source.
inline constexpr detail_payload::Read payload{};
}  // namespace source

/// Whether `S` presents every element of `CodecLengthRefUint16Be`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::payload(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::payload(std::declval<const S&>())), ::SCE::Forge::Bytes<1024>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<::SCE::Forge::Bytes<1024>>(source::payload(std::declval<const S&>())))> {};

/// The encode of `CodecLengthRefUint16Be` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecLengthRefUint16Be: payload");
        const auto& _r_payload = source::payload(src);
        const ::SCE::Forge::Bytes<1024> _e_payload = _r_payload;
        const uint16_t _d_payload_len = static_cast<uint16_t>(_e_payload.size());
        if (auto _e = w.write_u16_be(static_cast<std::uint16_t>(_d_payload_len)); _e) return _e;
        if (auto _e = w.write_bytes(_e_payload.data(), _e_payload.size()); _e) return _e;
        return std::nullopt;
    }
};

struct CodecLengthRefUint16Be {
    CodecLengthRefUint16Be() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecLengthRefUint16Be(::SCE::Forge::Bytes<1024> payload)
        : payload_(payload) {}
    /// The `payload` element.
    [[nodiscard]] ::SCE::Forge::Bytes<1024> payload() const noexcept { return payload_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecLengthRefUint16Be> decode(::SCE::Forge::SceCursor& cursor) {
        std::size_t _frame_len = cursor.remaining();
        if (_frame_len < 2) return std::nullopt;
        const std::uint8_t* raw = cursor.peek_slice(_frame_len);
        if (raw == nullptr) return std::nullopt;
        uint16_t payload_len = static_cast<uint16_t>((static_cast<uint16_t>(raw[0]) << 8) | raw[1]);
        if (payload_len > 1024) {
            cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange); return std::nullopt;
        }
        if (_frame_len < static_cast<std::size_t>(2 + payload_len)) return std::nullopt;
        std::vector<uint8_t> payload = std::vector<uint8_t>(raw + 2, raw + 2 + payload_len);
        const auto _b_payload = ::SCE::Forge::Bytes<1024>::make(payload);
        if (!_b_payload) {
            cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);
            return std::nullopt;
        }
        (void)payload_len;
        CodecLengthRefUint16Be _decoded{
            /*payload=*/*_b_payload,
        };
        if (!cursor.advance(_frame_len)) return std::nullopt;
        return _decoded;
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 1026;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable sinks
    /// (e.g. `VectorSink`) are effectively infallible.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w) const noexcept {
        // The encode over any source of the codec, this value one of them.
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
    ::SCE::Forge::Bounded<std::vector<std::uint8_t>, 1024> payload_{};
};

}  // namespace SCE::Generated::CodecLengthRefUint16Be

#endif  // SCE_FORGE_CODEC_LENGTH_REF_UINT16_BE_H
