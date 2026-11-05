// SCE-MAP: codec_chain_has_slice:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_CHAIN_HAS_SLICE_H
#define SCE_FORGE_CODEC_CHAIN_HAS_SLICE_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecChainHasSlice {

/// The elements a value of `CodecChainHasSlice` presents, each read by name: the
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
namespace detail_bytes {
void bytes() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.bytes())) -> decltype(s.bytes()) {
    return s.bytes();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(bytes(s))) -> decltype(bytes(s)) {
    return bytes(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_bytes
/// The `bytes` element of a source.
inline constexpr detail_bytes::Read bytes{};
}  // namespace source

/// Whether `S` presents every element of `CodecChainHasSlice`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::kind(std::declval<const S&>())), decltype(source::bytes(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::kind(std::declval<const S&>())), uint16_t>, std::is_convertible<decltype(source::bytes(std::declval<const S&>())), ::SCE::Forge::Bytes<256>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint16_t>(source::kind(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<::SCE::Forge::Bytes<256>>(source::bytes(std::declval<const S&>())))> {};

/// The encode of `CodecChainHasSlice` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecChainHasSlice: kind, bytes");
        const auto& _r_bytes = source::bytes(src);
        const ::SCE::Forge::Bytes<256> _e_bytes = _r_bytes;
        const uint64_t _d_len = static_cast<uint64_t>(_e_bytes.size());
        if (auto _e = w.write_vle_u16(static_cast<std::uint16_t>(source::kind(src))); _e) return _e;
        if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(_d_len)); _e) return _e;
        if (auto _e = w.write_bytes(_e_bytes.data(), _e_bytes.size()); _e) return _e;
        return std::nullopt;
    }
};

struct CodecChainHasSlice {
    CodecChainHasSlice() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecChainHasSlice(uint16_t kind, ::SCE::Forge::Bytes<256> bytes)
        : kind_(kind), bytes_(bytes) {}
    /// The `kind` element.
    [[nodiscard]] constexpr uint16_t kind() const noexcept { return kind_; }
    /// The `bytes` element.
    [[nodiscard]] ::SCE::Forge::Bytes<256> bytes() const noexcept { return bytes_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecChainHasSlice> decode(::SCE::Forge::SceCursor& cursor) {
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
        auto kind_opt = cursor.read_vle_u16();
        if (!kind_opt.has_value()) return std::nullopt;
        auto kind = static_cast<std::uint16_t>(*kind_opt);
        auto len_opt = cursor.read_vle_u64();
        if (!len_opt.has_value()) return std::nullopt;
        auto len = static_cast<std::uint64_t>(*len_opt);
        std::vector<uint8_t> bytes;
        {
            const auto _len = len;
            if (_len > 256) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange); return std::nullopt;
            }
            std::size_t _n = static_cast<std::size_t>(_len);
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            bytes.assign(raw, raw + _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        const auto _b_bytes = ::SCE::Forge::Bytes<256>::make(bytes);
        if (!_b_bytes) {
            cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);
            return std::nullopt;
        }
        (void)len;
        CodecChainHasSlice _decoded{
            /*kind=*/kind,
            /*bytes=*/*_b_bytes,
        };
        return _decoded;
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 268;

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
    uint16_t kind_{};
    ::SCE::Forge::Bounded<std::vector<std::uint8_t>, 256> bytes_{};
};

}  // namespace SCE::Generated::CodecChainHasSlice

#endif  // SCE_FORGE_CODEC_CHAIN_HAS_SLICE_H
