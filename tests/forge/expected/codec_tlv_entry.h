// SCE-MAP: codec_tlv_entry:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_TLV_ENTRY_H
#define SCE_FORGE_CODEC_TLV_ENTRY_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecTlvEntry {

/// The elements a value of `CodecTlvEntry` presents, each read by name: the
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
namespace detail_entry_body {
void entry_body() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.entry_body())) -> decltype(s.entry_body()) {
    return s.entry_body();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(entry_body(s))) -> decltype(entry_body(s)) {
    return entry_body(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_entry_body
/// The `entry_body` element of a source.
inline constexpr detail_entry_body::Read entry_body{};
}  // namespace source

/// Whether `S` presents every element of `CodecTlvEntry`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::entry_type(std::declval<const S&>())), decltype(source::entry_body(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::entry_type(std::declval<const S&>())), uint8_t>, std::is_convertible<decltype(source::entry_body(std::declval<const S&>())), ::SCE::Forge::Bytes<32>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::entry_type(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<::SCE::Forge::Bytes<32>>(source::entry_body(std::declval<const S&>())))> {};

/// The encode of `CodecTlvEntry` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecTlvEntry: entry_type, entry_body");
        const auto& _r_entry_body = source::entry_body(src);
        const ::SCE::Forge::Bytes<32> _e_entry_body = _r_entry_body;
        const uint8_t _d_entry_len = static_cast<uint8_t>(_e_entry_body.size());
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(source::entry_type(src))); _e) return _e;
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_d_entry_len)); _e) return _e;
        if (auto _e = w.write_bytes(_e_entry_body.data(), _e_entry_body.size()); _e) return _e;
        return std::nullopt;
    }
};

struct CodecTlvEntry {
    CodecTlvEntry() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecTlvEntry(uint8_t entry_type, ::SCE::Forge::Bytes<32> entry_body)
        : entry_type_(entry_type), entry_body_(entry_body) {}
    /// The `entry_type` element.
    [[nodiscard]] constexpr uint8_t entry_type() const noexcept { return entry_type_; }
    /// The `entry_body` element.
    [[nodiscard]] ::SCE::Forge::Bytes<32> entry_body() const noexcept { return entry_body_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecTlvEntry> decode(::SCE::Forge::SceCursor& cursor) {
        std::size_t _frame_len = cursor.remaining();
        if (_frame_len < 2) return std::nullopt;
        const std::uint8_t* raw = cursor.peek_slice(_frame_len);
        if (raw == nullptr) return std::nullopt;
        uint8_t entry_type = raw[0];
        uint8_t entry_len = raw[1];
        if (entry_len > 32) {
            cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange); return std::nullopt;
        }
        if (_frame_len < static_cast<std::size_t>(2 + entry_len)) return std::nullopt;
        std::vector<uint8_t> entry_body = std::vector<uint8_t>(raw + 2, raw + 2 + entry_len);
        const auto _b_entry_body = ::SCE::Forge::Bytes<32>::make(entry_body);
        if (!_b_entry_body) {
            cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);
            return std::nullopt;
        }
        (void)entry_len;
        CodecTlvEntry _decoded{
            /*entry_type=*/entry_type,
            /*entry_body=*/*_b_entry_body,
        };
        if (!cursor.advance(_frame_len)) return std::nullopt;
        return _decoded;
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 34;

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
    uint8_t entry_type_{};
    ::SCE::Forge::Bounded<std::vector<std::uint8_t>, 32> entry_body_{};
};

}  // namespace SCE::Generated::CodecTlvEntry

#endif  // SCE_FORGE_CODEC_TLV_ENTRY_H
