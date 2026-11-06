// SCE-MAP: codec_ext_attachment:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_EXT_ATTACHMENT_H
#define SCE_FORGE_CODEC_EXT_ATTACHMENT_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecExtAttachment {

/// The elements a value of `CodecExtAttachment` presents, each read by name: the
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

/// Whether `S` presents every element of `CodecExtAttachment`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::body(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::body(std::declval<const S&>())), ::SCE::Forge::Bytes<64>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<::SCE::Forge::Bytes<64>>(source::body(std::declval<const S&>())))> {};

/// The encode of `CodecExtAttachment` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecExtAttachment: body");
        const auto& _r_body = source::body(src);
        const ::SCE::Forge::Bytes<64> _e_body = _r_body;
        const uint8_t _d_length = static_cast<uint8_t>(_e_body.size());
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_d_length)); _e) return _e;
        if (auto _e = w.write_bytes(_e_body.data(), _e_body.size()); _e) return _e;
        return std::nullopt;
    }
};

struct CodecExtAttachment {
    CodecExtAttachment() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecExtAttachment(::SCE::Forge::Bytes<64> body)
        : body_(body) {}
    /// The `body` element.
    [[nodiscard]] ::SCE::Forge::Bytes<64> body() const noexcept { return body_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecExtAttachment> decode(::SCE::Forge::SceCursor& cursor) {
        std::size_t _frame_len = cursor.remaining();
        if (_frame_len < 1) return std::nullopt;
        const std::uint8_t* raw = cursor.peek_slice(_frame_len);
        if (raw == nullptr) return std::nullopt;
        uint8_t length = raw[0];
        if (length > 64) {
            cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange); return std::nullopt;
        }
        if (_frame_len < static_cast<std::size_t>(1 + length)) return std::nullopt;
        std::vector<uint8_t> body = std::vector<uint8_t>(raw + 1, raw + 1 + length);
        const auto _b_body = ::SCE::Forge::Bytes<64>::make(body);
        if (!_b_body) {
            cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);
            return std::nullopt;
        }
        (void)length;
        CodecExtAttachment _decoded{
            /*body=*/*_b_body,
        };
        // The bytes it read and no more (RFC §synth-5-B B3, as the Rust decode does): the
        // end of the last length-ref field, at least the fixed fields' bytes, so that a
        // codec holding this one as a body, an entry or an arm decodes on after it.
        std::size_t _consumed = 1;
        if (static_cast<std::size_t>(1 + length) > _consumed) {
            _consumed = static_cast<std::size_t>(1 + length);
        }
        if (!cursor.advance(_consumed)) return std::nullopt;
        return _decoded;
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 65;

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
    ::SCE::Forge::Bounded<std::vector<std::uint8_t>, 64> body_{};
};

}  // namespace SCE::Generated::CodecExtAttachment

#endif  // SCE_FORGE_CODEC_EXT_ATTACHMENT_H
