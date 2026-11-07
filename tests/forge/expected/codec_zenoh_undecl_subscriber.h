// SCE-MAP: codec_zenoh_undecl_subscriber:46 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_UNDECL_SUBSCRIBER_H
#define SCE_FORGE_CODEC_ZENOH_UNDECL_SUBSCRIBER_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_zenoh_decl_ext_keyexpr.h"

namespace SCE::Generated::CodecZenohUndeclSubscriber {

/// The elements a value of `CodecZenohUndeclSubscriber` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_id {
void id() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.id())) -> decltype(s.id()) {
    return s.id();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(id(s))) -> decltype(id(s)) {
    return id(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_id
/// The `id` element of a source.
inline constexpr detail_id::Read id{};
namespace detail_ext_keyexpr {
void ext_keyexpr() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.ext_keyexpr())) -> decltype(s.ext_keyexpr()) {
    return s.ext_keyexpr();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(ext_keyexpr(s))) -> decltype(ext_keyexpr(s)) {
    return ext_keyexpr(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_ext_keyexpr
/// The `ext_keyexpr` element of a source.
inline constexpr detail_ext_keyexpr::Read ext_keyexpr{};
}  // namespace source

/// Whether `R` is what a source may present for `ext_keyexpr`, a body a gate
/// decides: a `std::optional` of a source of the body's codec.
template <class R>
struct is_ext_keyexpr_body : std::false_type {};
template <class B>
struct is_ext_keyexpr_body<std::optional<B>> : ::SCE::Generated::CodecZenohDeclExtKeyexpr::is_source<B> {};

/// Whether `S` presents every element of `CodecZenohUndeclSubscriber`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::id(std::declval<const S&>())), decltype(source::ext_keyexpr(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::id(std::declval<const S&>())), uint32_t>, is_ext_keyexpr_body<std::decay_t<decltype(source::ext_keyexpr(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint32_t>(source::id(std::declval<const S&>()))) && noexcept(source::ext_keyexpr(std::declval<const S&>())) && ::SCE::Generated::CodecZenohDeclExtKeyexpr::is_nothrow_source<typename std::decay_t<decltype(source::ext_keyexpr(std::declval<const S&>()))>::value_type>::value> {};

/// The encode of `CodecZenohUndeclSubscriber` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecZenohUndeclSubscriber: id, ext_keyexpr");
        const auto& _r_ext_keyexpr = source::ext_keyexpr(src);
        if (auto _e = w.write_vle_u32(static_cast<std::uint32_t>(source::id(src))); _e) return _e;
        if (_r_ext_keyexpr) {
            if (auto _e = ::SCE::Generated::CodecZenohDeclExtKeyexpr::Wire::encode_source(*_r_ext_keyexpr, w); _e) return _e;
        }
        return std::nullopt;
    }
};

/// The flag input `Z` a source of `CodecZenohUndeclSubscriber` says, which its encode is not
/// handed (docs/adr/0065 §6 R4a).
template <class S>
[[nodiscard]] std::uint8_t input_z(const S& src) noexcept(is_nothrow_source<S>::value) {
    static_assert(is_source<S>::value, "S presents every element of CodecZenohUndeclSubscriber");
    return static_cast<std::uint8_t>(source::ext_keyexpr(src).has_value() ? 1 : 0);
}

struct CodecZenohUndeclSubscriber {
    CodecZenohUndeclSubscriber() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecZenohUndeclSubscriber(uint32_t id, std::optional<::SCE::Generated::CodecZenohDeclExtKeyexpr::CodecZenohDeclExtKeyexpr> ext_keyexpr)
        : id_(id), ext_keyexpr_(std::move(ext_keyexpr)) {}
    /// The `id` element.
    [[nodiscard]] constexpr uint32_t id() const noexcept { return id_; }
    /// The `ext_keyexpr` element, the body it holds where a gate leaves it in.
    [[nodiscard]] const std::optional<::SCE::Generated::CodecZenohDeclExtKeyexpr::CodecZenohDeclExtKeyexpr>& ext_keyexpr() const noexcept { return ext_keyexpr_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohUndeclSubscriber> decode(::SCE::Forge::SceCursor& cursor, std::uint8_t z) {
        // Declared-but-unconsumed flag inputs: defensive (void) suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly under -Wunused.
        (void)z;
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
        auto id_opt = cursor.read_vle_u32();
        if (!id_opt.has_value()) return std::nullopt;
        auto id = static_cast<std::uint32_t>(*id_opt);
        std::optional<::SCE::Generated::CodecZenohDeclExtKeyexpr::CodecZenohDeclExtKeyexpr> ext_keyexpr;
        if ((z & 0x01) != 0) {
            auto _emb = ::SCE::Generated::CodecZenohDeclExtKeyexpr::CodecZenohDeclExtKeyexpr::decode(cursor);
            if (!_emb.has_value()) return std::nullopt;
            ext_keyexpr = std::move(*_emb);
        }
        CodecZenohUndeclSubscriber _decoded{
            /*id=*/id,
            /*ext_keyexpr=*/std::move(ext_keyexpr),
        };
        return _decoded;
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 65560;

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
        _sce_v.reserve(MAX_ENCODED_BYTES);
        ::SCE::Forge::VectorSink _sce_sink(_sce_v);
        if (encode(_sce_sink)) {
            return std::nullopt;
        }
        return _sce_v;
    }

private:
    uint32_t id_{};
    std::optional<::SCE::Generated::CodecZenohDeclExtKeyexpr::CodecZenohDeclExtKeyexpr> ext_keyexpr_{};
};

}  // namespace SCE::Generated::CodecZenohUndeclSubscriber

#endif  // SCE_FORGE_CODEC_ZENOH_UNDECL_SUBSCRIBER_H
