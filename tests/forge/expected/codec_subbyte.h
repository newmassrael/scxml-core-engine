// SCE-MAP: codec_subbyte:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_SUBBYTE_H
#define SCE_FORGE_CODEC_SUBBYTE_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecSubbyte {

/// The elements a value of `CodecSubbyte` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_priority {
void priority() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.priority())) -> decltype(s.priority()) {
    return s.priority();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(priority(s))) -> decltype(priority(s)) {
    return priority(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_priority
/// The `priority` element of a source.
inline constexpr detail_priority::Read priority{};
namespace detail_channel {
void channel() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.channel())) -> decltype(s.channel()) {
    return s.channel();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(channel(s))) -> decltype(channel(s)) {
    return channel(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_channel
/// The `channel` element of a source.
inline constexpr detail_channel::Read channel{};
namespace detail_direction {
void direction() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.direction())) -> decltype(s.direction()) {
    return s.direction();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(direction(s))) -> decltype(direction(s)) {
    return direction(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_direction
/// The `direction` element of a source.
inline constexpr detail_direction::Read direction{};
}  // namespace source

/// Whether `S` presents every element of `CodecSubbyte`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::priority(std::declval<const S&>())), decltype(source::channel(std::declval<const S&>())), decltype(source::direction(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::priority(std::declval<const S&>())), uint8_t>, std::is_convertible<decltype(source::channel(std::declval<const S&>())), uint8_t>, std::is_convertible<decltype(source::direction(std::declval<const S&>())), uint8_t>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::priority(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::channel(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::direction(std::declval<const S&>())))> {};

/// The encode of `CodecSubbyte` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecSubbyte: priority, channel, direction");
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(((static_cast<std::uint32_t>(source::priority(src)) & 0x7u) << 5) | ((static_cast<std::uint32_t>(source::channel(src)) & 0x7u) << 2) | ((static_cast<std::uint32_t>(source::direction(src)) & 0x3u) << 0))); _e) return _e;
        return std::nullopt;
    }
};

struct CodecSubbyte {
    CodecSubbyte() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    constexpr CodecSubbyte(uint8_t priority, uint8_t channel, uint8_t direction) noexcept
        : priority_(priority), channel_(channel), direction_(direction) {}
    /// The `priority` element.
    [[nodiscard]] constexpr uint8_t priority() const noexcept { return priority_; }
    /// The `channel` element.
    [[nodiscard]] constexpr uint8_t channel() const noexcept { return channel_; }
    /// The `direction` element.
    [[nodiscard]] constexpr uint8_t direction() const noexcept { return direction_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecSubbyte> decode(::SCE::Forge::SceCursor& cursor) {
        const std::uint8_t* raw = cursor.peek_slice(1);
        if (raw == nullptr) return std::nullopt;
        uint8_t priority = static_cast<uint8_t>((raw[0] >> 5) & 0x07);
        uint8_t channel = static_cast<uint8_t>((raw[0] >> 2) & 0x07);
        uint8_t direction = static_cast<uint8_t>((raw[0] >> 0) & 0x03);
        CodecSubbyte _decoded{
            /*priority=*/priority,
            /*channel=*/channel,
            /*direction=*/direction,
        };
        if (!cursor.advance(1)) return std::nullopt;
        return _decoded;
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 1;

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
    uint8_t priority_{};
    uint8_t channel_{};
    uint8_t direction_{};
};

}  // namespace SCE::Generated::CodecSubbyte

#endif  // SCE_FORGE_CODEC_SUBBYTE_H
