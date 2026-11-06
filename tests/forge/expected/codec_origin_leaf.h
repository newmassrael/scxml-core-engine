// SCE-MAP: codec_origin_leaf:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ORIGIN_LEAF_H
#define SCE_FORGE_CODEC_ORIGIN_LEAF_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecOriginLeaf {

/// The elements a value of `CodecOriginLeaf` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
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

/// Whether `S` presents every element of `CodecOriginLeaf`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::data(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::data(std::declval<const S&>())), ::SCE::Forge::Bytes<16>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<::SCE::Forge::Bytes<16>>(source::data(std::declval<const S&>())))> {};

/// The encode of `CodecOriginLeaf` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecOriginLeaf: data");
        const auto& _r_data = source::data(src);
        const ::SCE::Forge::Bytes<16> _e_data = _r_data;
        const uint8_t _d_len = static_cast<uint8_t>(_e_data.size());
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_d_len)); _e) return _e;
        if (auto _e = w.write_bytes(_e_data.data(), _e_data.size()); _e) return _e;
        return std::nullopt;
    }
};

struct CodecOriginLeaf {
    CodecOriginLeaf() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecOriginLeaf(::SCE::Forge::Bytes<16> data)
        : data_(data) {}
    /// The `data` element.
    [[nodiscard]] ::SCE::Forge::Bytes<16> data() const noexcept { return data_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecOriginLeaf> decode(::SCE::Forge::SceCursor& cursor) {
        std::size_t _frame_len = cursor.remaining();
        if (_frame_len < 1) return std::nullopt;
        const std::uint8_t* raw = cursor.peek_slice(_frame_len);
        if (raw == nullptr) return std::nullopt;
        uint8_t len = raw[0];
        if (len > 16) {
            cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange); return std::nullopt;
        }
        if (_frame_len < static_cast<std::size_t>(1 + len)) return std::nullopt;
        std::vector<uint8_t> data = std::vector<uint8_t>(raw + 1, raw + 1 + len);
        const auto _b_data = ::SCE::Forge::Bytes<16>::make(data);
        if (!_b_data) {
            cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);
            return std::nullopt;
        }
        (void)len;
        CodecOriginLeaf _decoded{
            /*data=*/*_b_data,
        };
        // The bytes it read and no more (RFC §synth-5-B B3, as the Rust decode does): the
        // end of the last length-ref field, at least the fixed fields' bytes, so that a
        // codec holding this one as a body, an entry or an arm decodes on after it.
        std::size_t _consumed = 1;
        if (static_cast<std::size_t>(1 + len) > _consumed) {
            _consumed = static_cast<std::size_t>(1 + len);
        }
        if (!cursor.advance(_consumed)) return std::nullopt;
        return _decoded;
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 17;

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
    ::SCE::Forge::Bounded<std::vector<std::uint8_t>, 16> data_{};
};

}  // namespace SCE::Generated::CodecOriginLeaf

#endif  // SCE_FORGE_CODEC_ORIGIN_LEAF_H
