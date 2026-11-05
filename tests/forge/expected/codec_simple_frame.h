// SCE-MAP: codec_simple_frame:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_SIMPLE_FRAME_H
#define SCE_FORGE_CODEC_SIMPLE_FRAME_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecSimpleFrame {

/// The elements a value of `CodecSimpleFrame` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_msgId {
void msgId() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.msgId())) -> decltype(s.msgId()) {
    return s.msgId();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(msgId(s))) -> decltype(msgId(s)) {
    return msgId(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_msgId
/// The `msgId` element of a source.
inline constexpr detail_msgId::Read msgId{};
namespace detail_length {
void length() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.length())) -> decltype(s.length()) {
    return s.length();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(length(s))) -> decltype(length(s)) {
    return length(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_length
/// The `length` element of a source.
inline constexpr detail_length::Read length{};
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

/// Whether `S` presents every element of `CodecSimpleFrame`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::msgId(std::declval<const S&>())), decltype(source::length(std::declval<const S&>())), decltype(source::payload(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::msgId(std::declval<const S&>())), uint8_t>, std::is_convertible<decltype(source::length(std::declval<const S&>())), uint8_t>, std::is_convertible<decltype(source::payload(std::declval<const S&>())), uint16_t>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::msgId(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::length(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint16_t>(source::payload(std::declval<const S&>())))> {};

/// The encode of `CodecSimpleFrame` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecSimpleFrame: msgId, length, payload");
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(source::msgId(src))); _e) return _e;
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(source::length(src))); _e) return _e;
        if (auto _e = w.write_u16_be(static_cast<std::uint16_t>(source::payload(src))); _e) return _e;
        return std::nullopt;
    }
};

struct CodecSimpleFrame {
    CodecSimpleFrame() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    constexpr CodecSimpleFrame(uint8_t msgId, uint8_t length, uint16_t payload) noexcept
        : msgId_(msgId), length_(length), payload_(payload) {}
    /// The `msgId` element.
    [[nodiscard]] constexpr uint8_t msgId() const noexcept { return msgId_; }
    /// The `length` element.
    [[nodiscard]] constexpr uint8_t length() const noexcept { return length_; }
    /// The `payload` element.
    [[nodiscard]] constexpr uint16_t payload() const noexcept { return payload_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecSimpleFrame> decode(::SCE::Forge::SceCursor& cursor) {
        const std::uint8_t* raw = cursor.peek_slice(4);
        if (raw == nullptr) return std::nullopt;
        uint8_t msgId = raw[0];
        uint8_t length = raw[1];
        uint16_t payload = static_cast<uint16_t>((static_cast<uint16_t>(raw[2]) << 8) | raw[3]);
        CodecSimpleFrame _decoded{
            /*msgId=*/msgId,
            /*length=*/length,
            /*payload=*/payload,
        };
        if (!cursor.advance(4)) return std::nullopt;
        return _decoded;
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 4;

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
    uint8_t msgId_{};
    uint8_t length_{};
    uint16_t payload_{};
};

}  // namespace SCE::Generated::CodecSimpleFrame

#endif  // SCE_FORGE_CODEC_SIMPLE_FRAME_H
