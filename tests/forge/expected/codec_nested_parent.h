// SCE-MAP: codec_nested_parent:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_NESTED_PARENT_H
#define SCE_FORGE_CODEC_NESTED_PARENT_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_nested_body.h"

namespace SCE::Generated::CodecNestedParent {

/// The elements a value of `CodecNestedParent` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_hdr {
void hdr() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.hdr())) -> decltype(s.hdr()) {
    return s.hdr();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(hdr(s))) -> decltype(hdr(s)) {
    return hdr(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_hdr
/// The `hdr` element of a source.
inline constexpr detail_hdr::Read hdr{};
namespace detail_required_body {
void required_body() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.required_body())) -> decltype(s.required_body()) {
    return s.required_body();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(required_body(s))) -> decltype(required_body(s)) {
    return required_body(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_required_body
/// The `required_body` element of a source.
inline constexpr detail_required_body::Read required_body{};
namespace detail_optional_body {
void optional_body() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.optional_body())) -> decltype(s.optional_body()) {
    return s.optional_body();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(optional_body(s))) -> decltype(optional_body(s)) {
    return optional_body(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_optional_body
/// The `optional_body` element of a source.
inline constexpr detail_optional_body::Read optional_body{};
namespace detail_body_list {
void body_list() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.body_list())) -> decltype(s.body_list()) {
    return s.body_list();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(body_list(s))) -> decltype(body_list(s)) {
    return body_list(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_body_list
/// The `body_list` element of a source.
inline constexpr detail_body_list::Read body_list{};
}  // namespace source

/// Whether `R` is what a source may present for `optional_body`, a body a gate
/// decides: a `std::optional` of a source of the body's codec.
template <class R>
struct is_optional_body_body : std::false_type {};
template <class B>
struct is_optional_body_body<std::optional<B>> : ::SCE::Generated::CodecNestedBody::is_source<B> {};

/// Whether `R` is a list a source may present for `body_list`: held to bounds
/// within its own, of any type, each entry a source of the entry codec.
template <class R, class = void>
struct is_body_list_list : std::false_type {};
template <class R>
struct is_body_list_list<R, std::enable_if_t<::SCE::Forge::Detail::IsBoundedWithin<R, 4>::value>>
    : ::SCE::Generated::CodecNestedBody::is_source<::SCE::Forge::Detail::ElementOf<R>> {};

/// Whether `S` presents every element of `CodecNestedParent`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::hdr(std::declval<const S&>())), decltype(source::required_body(std::declval<const S&>())), decltype(source::optional_body(std::declval<const S&>())), decltype(source::body_list(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::hdr(std::declval<const S&>())), uint8_t>, ::SCE::Generated::CodecNestedBody::is_source<std::decay_t<decltype(source::required_body(std::declval<const S&>()))>>, is_optional_body_body<std::decay_t<decltype(source::optional_body(std::declval<const S&>()))>>, is_body_list_list<std::decay_t<decltype(source::body_list(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::hdr(std::declval<const S&>()))) && noexcept(source::required_body(std::declval<const S&>())) && ::SCE::Generated::CodecNestedBody::is_nothrow_source<std::decay_t<decltype(source::required_body(std::declval<const S&>()))>>::value && noexcept(source::optional_body(std::declval<const S&>())) && ::SCE::Generated::CodecNestedBody::is_nothrow_source<typename std::decay_t<decltype(source::optional_body(std::declval<const S&>()))>::value_type>::value && noexcept(source::body_list(std::declval<const S&>())) && ::SCE::Generated::CodecNestedBody::is_nothrow_source<::SCE::Forge::Detail::ElementOf<std::decay_t<decltype(source::body_list(std::declval<const S&>()))>>>::value> {};

/// The encode of `CodecNestedParent` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecNestedParent: hdr, required_body, optional_body, body_list");
        const auto& _r_required_body = source::required_body(src);
        const auto& _r_optional_body = source::optional_body(src);
        const auto& _r_body_list = source::body_list(src);
        const uint8_t _d_m = static_cast<uint8_t>(_r_body_list.size());
        std::uint64_t _f_hdr = static_cast<std::uint64_t>(source::hdr(src));
        _f_hdr = (_f_hdr & ~0x1ull) | static_cast<std::uint64_t>(_r_optional_body.has_value());
        const uint8_t _c_hdr = static_cast<uint8_t>(_f_hdr);
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_c_hdr)); _e) return _e;
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_d_m)); _e) return _e;
        if (auto _e = ::SCE::Generated::CodecNestedBody::Wire::encode_source(_r_required_body, w); _e) return _e;
        if (_r_optional_body) {
            if (auto _e = ::SCE::Generated::CodecNestedBody::Wire::encode_source(*_r_optional_body, w); _e) return _e;
        }
        for (const auto& _x : _r_body_list) {
            if (auto _e = ::SCE::Generated::CodecNestedBody::Wire::encode_source(_x, w); _e) return _e;
        }
        return std::nullopt;
    }
};

struct CodecNestedParent {
    CodecNestedParent() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecNestedParent(uint8_t hdr, ::SCE::Generated::CodecNestedBody::CodecNestedBody required_body, std::optional<::SCE::Generated::CodecNestedBody::CodecNestedBody> optional_body, ::SCE::Forge::List<::SCE::Generated::CodecNestedBody::CodecNestedBody, 4> body_list)
        : hdr_(hdr), required_body_(std::move(required_body)), optional_body_(std::move(optional_body)), body_list_(body_list) {}
    /// The `hdr` element.
    [[nodiscard]] constexpr uint8_t hdr() const noexcept { return hdr_; }
    /// The `required_body` element, the body it holds.
    [[nodiscard]] const ::SCE::Generated::CodecNestedBody::CodecNestedBody& required_body() const noexcept { return required_body_; }
    /// The `optional_body` element, the body it holds where a gate leaves it in.
    [[nodiscard]] const std::optional<::SCE::Generated::CodecNestedBody::CodecNestedBody>& optional_body() const noexcept { return optional_body_; }
    /// The `body_list` element, the entries it holds.
    [[nodiscard]] ::SCE::Forge::List<::SCE::Generated::CodecNestedBody::CodecNestedBody, 4> body_list() const noexcept { return body_list_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecNestedParent> decode(::SCE::Forge::SceCursor& cursor) {
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
        uint8_t hdr;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            hdr = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        uint8_t m;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            m = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        auto _emb_required_body = ::SCE::Generated::CodecNestedBody::CodecNestedBody::decode(cursor);
        if (!_emb_required_body.has_value()) return std::nullopt;
        auto required_body = std::move(*_emb_required_body);
        std::optional<::SCE::Generated::CodecNestedBody::CodecNestedBody> optional_body;
        if ((hdr & 0x01) != 0) {
            auto _emb = ::SCE::Generated::CodecNestedBody::CodecNestedBody::decode(cursor);
            if (!_emb.has_value()) return std::nullopt;
            optional_body = std::move(*_emb);
        }
        std::vector<::SCE::Generated::CodecNestedBody::CodecNestedBody> body_list;
        if (m > 4) {
            cursor.refuse(::SCE::Forge::CodecError::TooManyElements); return std::nullopt;
        }
        body_list.reserve(cursor.entries_backed(m));
        for (auto _i = decltype(m){0}; _i < m; ++_i) {
            auto _elem = ::SCE::Generated::CodecNestedBody::CodecNestedBody::decode(cursor);
            if (!_elem.has_value()) return std::nullopt;
            body_list.push_back(*_elem);
        }
        const auto _b_body_list = ::SCE::Forge::List<::SCE::Generated::CodecNestedBody::CodecNestedBody, 4>::make(body_list);
        if (!_b_body_list) {
            cursor.refuse(::SCE::Forge::CodecError::TooManyElements);
            return std::nullopt;
        }
        (void)m;
        CodecNestedParent _decoded{
            /*hdr=*/hdr,
            /*required_body=*/std::move(required_body),
            /*optional_body=*/std::move(optional_body),
            /*body_list=*/*_b_body_list,
        };
        return _decoded;
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool has_opt() const noexcept {
        return (this->hdr_ & 0x01) != 0;
    }

    void set_has_opt(bool v) noexcept {
        if (v) {
            this->hdr_ = static_cast<uint8_t>(this->hdr_ | 0x01);
        } else {
            this->hdr_ = static_cast<uint8_t>(this->hdr_ & static_cast<uint8_t>(~0x01));
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 6344;

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
    uint8_t hdr_{};
    ::SCE::Generated::CodecNestedBody::CodecNestedBody required_body_{};
    std::optional<::SCE::Generated::CodecNestedBody::CodecNestedBody> optional_body_{};
    ::SCE::Forge::Bounded<std::vector<::SCE::Generated::CodecNestedBody::CodecNestedBody>, 4> body_list_{};
};

}  // namespace SCE::Generated::CodecNestedParent

#endif  // SCE_FORGE_CODEC_NESTED_PARENT_H
