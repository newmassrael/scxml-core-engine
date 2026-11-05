// SCE-MAP: codec_present_if_string:47 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_PRESENT_IF_STRING_H
#define SCE_FORGE_CODEC_PRESENT_IF_STRING_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <string>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecPresentIfString {

/// The elements a value of `CodecPresentIfString` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_carrier {
void carrier() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.carrier())) -> decltype(s.carrier()) {
    return s.carrier();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(carrier(s))) -> decltype(carrier(s)) {
    return carrier(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_carrier
/// The `carrier` element of a source.
inline constexpr detail_carrier::Read carrier{};
namespace detail_text {
void text() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.text())) -> decltype(s.text()) {
    return s.text();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(text(s))) -> decltype(text(s)) {
    return text(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_text
/// The `text` element of a source.
inline constexpr detail_text::Read text{};
}  // namespace source

/// Whether `S` presents every element of `CodecPresentIfString`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::carrier(std::declval<const S&>())), decltype(source::text(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::carrier(std::declval<const S&>())), uint8_t>, std::is_convertible<decltype(source::text(std::declval<const S&>())), std::optional<::SCE::Forge::Text<32>>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::carrier(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<std::optional<::SCE::Forge::Text<32>>>(source::text(std::declval<const S&>())))> {};

/// The encode of `CodecPresentIfString` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecPresentIfString: carrier, text");
        const auto& _r_text = source::text(src);
        const std::optional<::SCE::Forge::Text<32>> _e_text = _r_text;
        const uint8_t _d_text_len = _e_text.has_value() ? static_cast<uint8_t>(_e_text->size()) : uint8_t{0};
        std::uint64_t _f_carrier = static_cast<std::uint64_t>(source::carrier(src));
        _f_carrier = (_f_carrier & ~0x1ull) | static_cast<std::uint64_t>(_e_text.has_value());
        const uint8_t _c_carrier = static_cast<uint8_t>(_f_carrier);
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_c_carrier)); _e) return _e;
        if (((static_cast<std::uint64_t>(_c_carrier) & 0x1ull) != 0)) {
            if (auto _e = w.write_u8(static_cast<std::uint8_t>(_d_text_len)); _e) return _e;
        }
        if (_e_text) {
            if (auto _e = w.write_bytes(reinterpret_cast<const std::uint8_t*>(_e_text->data()), _e_text->size()); _e) return _e;
        }
        return std::nullopt;
    }
};

struct CodecPresentIfString {
    CodecPresentIfString() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecPresentIfString(uint8_t carrier, std::optional<::SCE::Forge::Text<32>> text)
        : carrier_(carrier), text_(text) {}
    /// The `carrier` element.
    [[nodiscard]] constexpr uint8_t carrier() const noexcept { return carrier_; }
    /// The `text` element, where a gate leaves it in.
    [[nodiscard]] std::optional<::SCE::Forge::Text<32>> text() const noexcept { if (!text_) { return std::nullopt; } return ::SCE::Forge::Text<32>(*text_); }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecPresentIfString> decode(::SCE::Forge::SceCursor& cursor) {
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
        uint8_t carrier;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            carrier = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<uint8_t> text_len;
        if ((carrier & 0x01) != 0) {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            text_len = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<std::string> text;
        if ((carrier & 0x01) != 0) {
            const auto _len = text_len.value();
            if (_len > 32) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange); return std::nullopt;
            }
            std::size_t _n = static_cast<std::size_t>(_len);
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            if (!::SCE::Forge::is_valid_utf8(raw, _n)) return std::nullopt;
            text.emplace(reinterpret_cast<const char*>(raw), _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        std::optional<::SCE::Forge::Text<32>> _b_text;
        if (text) {
            _b_text = ::SCE::Forge::Text<32>::make(*text);
            if (!_b_text) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);
                return std::nullopt;
            }
        }
        (void)text_len;
        CodecPresentIfString _decoded{
            /*carrier=*/carrier,
            /*text=*/_b_text,
        };
        return _decoded;
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool has_text() const noexcept {
        return (this->carrier_ & 0x01) != 0;
    }

    void set_has_text(bool v) noexcept {
        if (v) {
            this->carrier_ = static_cast<uint8_t>(this->carrier_ | 0x01);
        } else {
            this->carrier_ = static_cast<uint8_t>(this->carrier_ & static_cast<uint8_t>(~0x01));
        }
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
    uint8_t carrier_{};
    std::optional<::SCE::Forge::Bounded<std::string, 32>> text_{};
};

}  // namespace SCE::Generated::CodecPresentIfString

#endif  // SCE_FORGE_CODEC_PRESENT_IF_STRING_H
