// SCE-MAP: codec_init_cookie_body:36 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_INIT_COOKIE_BODY_H
#define SCE_FORGE_CODEC_INIT_COOKIE_BODY_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecInitCookieBody {

/// The elements a value of `CodecInitCookieBody` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_version {
void version() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.version())) -> decltype(s.version()) {
    return s.version();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(version(s))) -> decltype(version(s)) {
    return version(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_version
/// The `version` element of a source.
inline constexpr detail_version::Read version{};
namespace detail_cookie {
void cookie() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.cookie())) -> decltype(s.cookie()) {
    return s.cookie();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(cookie(s))) -> decltype(cookie(s)) {
    return cookie(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_cookie
/// The `cookie` element of a source.
inline constexpr detail_cookie::Read cookie{};
}  // namespace source

/// Whether `S` presents every element of `CodecInitCookieBody`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::version(std::declval<const S&>())), decltype(source::cookie(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::version(std::declval<const S&>())), uint8_t>, std::is_convertible<decltype(source::cookie(std::declval<const S&>())), std::optional<::SCE::Forge::Bytes<64>>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::version(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<std::optional<::SCE::Forge::Bytes<64>>>(source::cookie(std::declval<const S&>())))> {};

/// The encode of `CodecInitCookieBody` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecInitCookieBody: version, cookie");
        const auto& _r_cookie = source::cookie(src);
        const std::optional<::SCE::Forge::Bytes<64>> _e_cookie = _r_cookie;
        const std::uint8_t a = static_cast<std::uint8_t>(_e_cookie.has_value() ? 1 : 0);
        (void)a;
        const uint16_t _d_cookie_size = _e_cookie.has_value() ? static_cast<uint16_t>(_e_cookie->size()) : uint16_t{0};
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(source::version(src))); _e) return _e;
        if (((static_cast<std::uint64_t>(a) & 0x1ull) != 0)) {
            if (auto _e = w.write_vle_u16(static_cast<std::uint16_t>(_d_cookie_size)); _e) return _e;
        }
        if (_e_cookie) {
            if (auto _e = w.write_bytes(_e_cookie->data(), _e_cookie->size()); _e) return _e;
        }
        return std::nullopt;
    }
};

/// The flag input `A` a source of `CodecInitCookieBody` says, which its encode is not
/// handed (docs/adr/0065 §6 R4a).
template <class S>
[[nodiscard]] std::uint8_t input_a(const S& src) noexcept(is_nothrow_source<S>::value) {
    static_assert(is_source<S>::value, "S presents every element of CodecInitCookieBody");
    return static_cast<std::uint8_t>(static_cast<std::optional<::SCE::Forge::Bytes<64>>>(source::cookie(src)).has_value() ? 1 : 0);
}

struct CodecInitCookieBody {
    CodecInitCookieBody() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecInitCookieBody(uint8_t version, std::optional<::SCE::Forge::Bytes<64>> cookie)
        : version_(version), cookie_(cookie) {}
    /// The `version` element.
    [[nodiscard]] constexpr uint8_t version() const noexcept { return version_; }
    /// The `cookie` element, where a gate leaves it in.
    [[nodiscard]] std::optional<::SCE::Forge::Bytes<64>> cookie() const noexcept { if (!cookie_) { return std::nullopt; } return ::SCE::Forge::Bytes<64>(*cookie_); }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecInitCookieBody> decode(::SCE::Forge::SceCursor& cursor, std::uint8_t a) {
        // Declared-but-unconsumed flag inputs: defensive (void) suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly under -Wunused.
        (void)a;
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
        uint8_t version;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            version = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<uint16_t> cookie_size;
        if ((a & 0x01) != 0) {
            auto _v_opt = cursor.read_vle_u16();
        if (!_v_opt.has_value()) return std::nullopt;
        auto _v = static_cast<std::uint16_t>(*_v_opt);
            cookie_size = _v;
        }
        std::optional<std::vector<uint8_t>> cookie;
        if ((a & 0x01) != 0) {
            const auto _len = cookie_size.value();
            if (_len > 64) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange); return std::nullopt;
            }
            std::size_t _n = static_cast<std::size_t>(_len);
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            cookie.emplace(raw, raw + _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        std::optional<::SCE::Forge::Bytes<64>> _b_cookie;
        if (cookie) {
            _b_cookie = ::SCE::Forge::Bytes<64>::make(*cookie);
            if (!_b_cookie) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);
                return std::nullopt;
            }
        }
        (void)cookie_size;
        CodecInitCookieBody _decoded{
            /*version=*/version,
            /*cookie=*/_b_cookie,
        };
        return _decoded;
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 68;

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
    uint8_t version_{};
    std::optional<::SCE::Forge::Bounded<std::vector<std::uint8_t>, 64>> cookie_{};
};

}  // namespace SCE::Generated::CodecInitCookieBody

#endif  // SCE_FORGE_CODEC_INIT_COOKIE_BODY_H
