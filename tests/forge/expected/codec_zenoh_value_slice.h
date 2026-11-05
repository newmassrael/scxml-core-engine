// SCE-MAP: codec_zenoh_value_slice:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_VALUE_SLICE_H
#define SCE_FORGE_CODEC_ZENOH_VALUE_SLICE_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecZenohValueSlice {

/// The elements a value of `CodecZenohValueSlice` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_kind {
void kind() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.kind())) -> decltype(s.kind()) {
    return s.kind();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(kind(s))) -> decltype(kind(s)) {
    return kind(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_kind
/// The `kind` element of a source.
inline constexpr detail_kind::Read kind{};
namespace detail_data {
void data() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.data())) -> decltype(s.data()) {
    return s.data();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(data(s))) -> decltype(data(s)) {
    return data(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_data
/// The `data` element of a source.
inline constexpr detail_data::Read data{};
}  // namespace source

/// Whether `S` presents every element of `CodecZenohValueSlice`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::kind(std::declval<const S&>())), decltype(source::data(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::kind(std::declval<const S&>())), uint8_t>, std::is_convertible<decltype(source::data(std::declval<const S&>())), ::SCE::Forge::Bytes<4294967295>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::kind(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<::SCE::Forge::Bytes<4294967295>>(source::data(std::declval<const S&>())))> {};

/// The encode of `CodecZenohValueSlice` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecZenohValueSlice: kind, data");
        const auto& _r_data = source::data(src);
        const ::SCE::Forge::Bytes<4294967295> _e_data = _r_data;
        const uint64_t _d_len = static_cast<uint64_t>(_e_data.size());
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(source::kind(src))); _e) return _e;
        if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(_d_len)); _e) return _e;
        if (auto _e = w.write_bytes(_e_data.data(), _e_data.size()); _e) return _e;
        return std::nullopt;
    }
};

struct CodecZenohValueSlice {
    CodecZenohValueSlice() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecZenohValueSlice(uint8_t kind, ::SCE::Forge::Bytes<4294967295> data)
        : kind_(kind), data_(data) {}
    /// The `kind` element.
    [[nodiscard]] constexpr uint8_t kind() const noexcept { return kind_; }
    /// The `data` element.
    [[nodiscard]] ::SCE::Forge::Bytes<4294967295> data() const noexcept { return data_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohValueSlice> decode(::SCE::Forge::SceCursor& cursor) {
        // Streaming cursor decode (SSOT selection: `needs_streaming`).
        // The positional `raw[byte_off]` path is valid only when every
        // field's absolute offset is fixed at codegen time; this branch
        // handles every codec where it is not — present-if-gated fields
        // (runtime presence), VLE / repeat / TLV-chain / embed fields
        // (runtime width), string fields (UTF-8 decode), and a fixed field
        // after a variable-length payload (offset depends on the payload
        // length). Each field reads its own bytes from the cursor and
        // advances past exactly what it consumed. Per-field `is_repeat` /
        // `is_tlv_chain` / `is_embed` route to their dedicated helpers;
        // every other field flows through `present_if_decode_stmt`.
        uint8_t kind;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            kind = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        auto len_opt = cursor.read_vle_u64();
        if (!len_opt.has_value()) return std::nullopt;
        auto len = static_cast<std::uint64_t>(*len_opt);
        std::vector<uint8_t> data;
        {
            const auto _len = len;
            if (_len > 4294967295ULL) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange); return std::nullopt;
            }
            if (static_cast<std::uint64_t>(_len) > static_cast<std::uint64_t>(SIZE_MAX) - 0u) return std::nullopt;
            std::size_t _n = static_cast<std::size_t>(_len);
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            data.assign(raw, raw + _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        const auto _b_data = ::SCE::Forge::Bytes<4294967295>::make(data);
        if (!_b_data) {
            cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);
            return std::nullopt;
        }
        (void)len;
        CodecZenohValueSlice _decoded{
            /*kind=*/kind,
            /*data=*/*_b_data,
        };
        return _decoded;
    }


    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable sinks
    /// (e.g. `VectorSink`) are effectively infallible.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w) const noexcept {
        // The encode over any source of the codec, this value one of them.
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
    uint8_t kind_{};
    ::SCE::Forge::Bounded<std::vector<std::uint8_t>, 4294967295> data_{};
};

}  // namespace SCE::Generated::CodecZenohValueSlice

#endif  // SCE_FORGE_CODEC_ZENOH_VALUE_SLICE_H
