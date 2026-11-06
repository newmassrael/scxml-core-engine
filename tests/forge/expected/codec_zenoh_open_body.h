// SCE-MAP: codec_zenoh_open_body:42 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_OPEN_BODY_H
#define SCE_FORGE_CODEC_ZENOH_OPEN_BODY_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecZenohOpenBody {

/// The elements a value of `CodecZenohOpenBody` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_lease {
void lease() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.lease())) -> decltype(s.lease()) {
    return s.lease();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(lease(s))) -> decltype(lease(s)) {
    return lease(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_lease
/// The `lease` element of a source.
inline constexpr detail_lease::Read lease{};
namespace detail_initial_sn {
void initial_sn() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.initial_sn())) -> decltype(s.initial_sn()) {
    return s.initial_sn();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(initial_sn(s))) -> decltype(initial_sn(s)) {
    return initial_sn(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_initial_sn
/// The `initial_sn` element of a source.
inline constexpr detail_initial_sn::Read initial_sn{};
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

/// Whether `S` presents every element of `CodecZenohOpenBody`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::lease(std::declval<const S&>())), decltype(source::initial_sn(std::declval<const S&>())), decltype(source::cookie(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::lease(std::declval<const S&>())), uint64_t>, std::is_convertible<decltype(source::initial_sn(std::declval<const S&>())), uint64_t>, std::is_convertible<decltype(source::cookie(std::declval<const S&>())), std::optional<::SCE::Forge::Bytes<::SCE::Forge::kUnbounded>>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint64_t>(source::lease(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint64_t>(source::initial_sn(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<std::optional<::SCE::Forge::Bytes<::SCE::Forge::kUnbounded>>>(source::cookie(std::declval<const S&>())))> {};

/// The encode of `CodecZenohOpenBody` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecZenohOpenBody: lease, initial_sn, cookie");
        const auto& _r_cookie = source::cookie(src);
        const std::optional<::SCE::Forge::Bytes<::SCE::Forge::kUnbounded>> _e_cookie = _r_cookie;
        const std::uint8_t a = static_cast<std::uint8_t>(_e_cookie.has_value() ? 0 : 1);
        (void)a;
        const uint64_t _d_cookie_len = _e_cookie.has_value() ? static_cast<uint64_t>(_e_cookie->size()) : uint64_t{0};
        if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(source::lease(src))); _e) return _e;
        if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(source::initial_sn(src))); _e) return _e;
        if (((static_cast<std::uint64_t>(a) & 0x1ull) == 0)) {
            if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(_d_cookie_len)); _e) return _e;
        }
        if (_e_cookie) {
            if (auto _e = w.write_bytes(_e_cookie->data(), _e_cookie->size()); _e) return _e;
        }
        return std::nullopt;
    }
};

/// The flag input `A` a source of `CodecZenohOpenBody` says, which its encode is not
/// handed (docs/adr/0065 §6 R4a).
template <class S>
[[nodiscard]] std::uint8_t input_a(const S& src) noexcept(is_nothrow_source<S>::value) {
    static_assert(is_source<S>::value, "S presents every element of CodecZenohOpenBody");
    return static_cast<std::uint8_t>(static_cast<std::optional<::SCE::Forge::Bytes<::SCE::Forge::kUnbounded>>>(source::cookie(src)).has_value() ? 0 : 1);
}

struct CodecZenohOpenBody {
    CodecZenohOpenBody() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecZenohOpenBody(uint64_t lease, uint64_t initial_sn, std::optional<::SCE::Forge::Bytes<::SCE::Forge::kUnbounded>> cookie)
        : lease_(lease), initial_sn_(initial_sn), cookie_(cookie) {}
    /// The `lease` element.
    [[nodiscard]] constexpr uint64_t lease() const noexcept { return lease_; }
    /// The `initial_sn` element.
    [[nodiscard]] constexpr uint64_t initial_sn() const noexcept { return initial_sn_; }
    /// The `cookie` element, where a gate leaves it in.
    [[nodiscard]] std::optional<::SCE::Forge::Bytes<::SCE::Forge::kUnbounded>> cookie() const noexcept { if (!cookie_) { return std::nullopt; } return ::SCE::Forge::Bytes<::SCE::Forge::kUnbounded>(*cookie_); }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohOpenBody> decode(::SCE::Forge::SceCursor& cursor, std::uint8_t a) {
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
        auto lease_opt = cursor.read_vle_u64();
        if (!lease_opt.has_value()) return std::nullopt;
        auto lease = static_cast<std::uint64_t>(*lease_opt);
        auto initial_sn_opt = cursor.read_vle_u64();
        if (!initial_sn_opt.has_value()) return std::nullopt;
        auto initial_sn = static_cast<std::uint64_t>(*initial_sn_opt);
        std::optional<uint64_t> cookie_len;
        if ((a & 0x01) == 0) {
            auto _v_opt = cursor.read_vle_u64();
        if (!_v_opt.has_value()) return std::nullopt;
        auto _v = static_cast<std::uint64_t>(*_v_opt);
            cookie_len = _v;
        }
        std::optional<std::vector<uint8_t>> cookie;
        if ((a & 0x01) == 0) {
            const auto _len = cookie_len.value();
            if (static_cast<std::uint64_t>(_len) > static_cast<std::uint64_t>(SIZE_MAX) - 0u) return std::nullopt;
            std::size_t _n = static_cast<std::size_t>(_len);
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            cookie.emplace(raw, raw + _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        std::optional<::SCE::Forge::Bytes<::SCE::Forge::kUnbounded>> _b_cookie;
        if (cookie) {
            _b_cookie = ::SCE::Forge::Bytes<::SCE::Forge::kUnbounded>::make(*cookie);
            if (!_b_cookie) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);
                return std::nullopt;
            }
        }
        (void)cookie_len;
        CodecZenohOpenBody _decoded{
            /*lease=*/lease,
            /*initial_sn=*/initial_sn,
            /*cookie=*/_b_cookie,
        };
        return _decoded;
    }


    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable sinks
    /// (e.g. `VectorSink`) are effectively infallible.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w) const noexcept {
        // The encode over any source of the codec, this value one of them,
        // handed the inputs the caller hands this encode.
        return Wire::encode_source(*this, w);
    }

    /// Heap-backed convenience facade. It reserves nothing, since the
    /// largest frame passes `u32` (docs/adr/0077 §7), and delegates to
    /// `encode` over a `VectorSink`. Returns the
    /// freshly-encoded byte vector, or `std::nullopt` when `encode`
    /// refuses the value before its first byte: a gated field and the
    /// `sce:present-if` that gates it contradict each other
    /// (`CodecError::PresentIfMismatch`), or a byte string, text or list
    /// lies outside its bound (`CodecError::LengthOutOfRange`,
    /// `CodecError::TooManyElements`). Callers that need the reason, and
    /// zero-alloc hot paths, call `encode` directly against a caller-owned
    /// sink.
    [[nodiscard]] std::optional<std::vector<std::uint8_t>> encode_to_vec() const {
        std::vector<std::uint8_t> _sce_v;
        ::SCE::Forge::VectorSink _sce_sink(_sce_v);
        if (encode(_sce_sink)) {
            return std::nullopt;
        }
        return _sce_v;
    }

private:
    uint64_t lease_{};
    uint64_t initial_sn_{};
    std::optional<::SCE::Forge::Bounded<std::vector<std::uint8_t>, ::SCE::Forge::kUnbounded>> cookie_{};
};

}  // namespace SCE::Generated::CodecZenohOpenBody

#endif  // SCE_FORGE_CODEC_ZENOH_OPEN_BODY_H
