// SCE-MAP: codec_chain_has_tagged_entry:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_CHAIN_HAS_TAGGED_ENTRY_H
#define SCE_FORGE_CODEC_CHAIN_HAS_TAGGED_ENTRY_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecChainHasTaggedEntry {

/// The elements a value of `CodecChainHasTaggedEntry` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_entry_type {
void entry_type() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.entry_type())) -> decltype(s.entry_type()) {
    return s.entry_type();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(entry_type(s))) -> decltype(entry_type(s)) {
    return entry_type(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_entry_type
/// The `entry_type` element of a source.
inline constexpr detail_entry_type::Read entry_type{};
namespace detail_ctl {
void ctl() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.ctl())) -> decltype(s.ctl()) {
    return s.ctl();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(ctl(s))) -> decltype(ctl(s)) {
    return ctl(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_ctl
/// The `ctl` element of a source.
inline constexpr detail_ctl::Read ctl{};
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

/// Whether `S` presents every element of `CodecChainHasTaggedEntry`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::entry_type(std::declval<const S&>())), decltype(source::ctl(std::declval<const S&>())), decltype(source::body(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::entry_type(std::declval<const S&>())), uint8_t>, std::is_convertible<decltype(source::ctl(std::declval<const S&>())), uint8_t>, std::is_convertible<decltype(source::body(std::declval<const S&>())), uint8_t>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::entry_type(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::ctl(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::body(std::declval<const S&>())))> {};

/// The encode of `CodecChainHasTaggedEntry` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecChainHasTaggedEntry: entry_type, ctl, body");
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(source::entry_type(src))); _e) return _e;
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(source::ctl(src))); _e) return _e;
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(source::body(src))); _e) return _e;
        return std::nullopt;
    }
};

struct CodecChainHasTaggedEntry {
    CodecChainHasTaggedEntry() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    constexpr CodecChainHasTaggedEntry(uint8_t entry_type, uint8_t ctl, uint8_t body) noexcept
        : entry_type_(entry_type), ctl_(ctl), body_(body) {}
    /// The `entry_type` element.
    [[nodiscard]] constexpr uint8_t entry_type() const noexcept { return entry_type_; }
    /// The `ctl` element.
    [[nodiscard]] constexpr uint8_t ctl() const noexcept { return ctl_; }
    /// The `body` element.
    [[nodiscard]] constexpr uint8_t body() const noexcept { return body_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecChainHasTaggedEntry> decode(::SCE::Forge::SceCursor& cursor) {
        const std::uint8_t* raw = cursor.peek_slice(3);
        if (raw == nullptr) return std::nullopt;
        uint8_t entry_type = raw[0];
        uint8_t ctl = raw[1];
        uint8_t body = raw[2];
        CodecChainHasTaggedEntry _decoded{
            /*entry_type=*/entry_type,
            /*ctl=*/ctl,
            /*body=*/body,
        };
        if (!cursor.advance(3)) return std::nullopt;
        return _decoded;
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool more() const noexcept {
        return (this->ctl_ & 0x01) != 0;
    }

    void set_more(bool v) noexcept {
        if (v) {
            this->ctl_ = static_cast<uint8_t>(this->ctl_ | 0x01);
        } else {
            this->ctl_ = static_cast<uint8_t>(this->ctl_ & static_cast<uint8_t>(~0x01));
        }
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
    uint8_t entry_type_{};
    uint8_t ctl_{};
    uint8_t body_{};
};

}  // namespace SCE::Generated::CodecChainHasTaggedEntry

#endif  // SCE_FORGE_CODEC_CHAIN_HAS_TAGGED_ENTRY_H
