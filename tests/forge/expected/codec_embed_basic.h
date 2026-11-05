// SCE-MAP: codec_embed_basic:43 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_EMBED_BASIC_H
#define SCE_FORGE_CODEC_EMBED_BASIC_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_zenoh_locator.h"

namespace SCE::Generated::CodecEmbedBasic {

/// The elements a value of `CodecEmbedBasic` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_tag {
void tag() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.tag())) -> decltype(s.tag()) {
    return s.tag();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(tag(s))) -> decltype(tag(s)) {
    return tag(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_tag
/// The `tag` element of a source.
inline constexpr detail_tag::Read tag{};
namespace detail_locator {
void locator() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.locator())) -> decltype(s.locator()) {
    return s.locator();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(locator(s))) -> decltype(locator(s)) {
    return locator(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_locator
/// The `locator` element of a source.
inline constexpr detail_locator::Read locator{};
}  // namespace source

/// Whether `S` presents every element of `CodecEmbedBasic`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::tag(std::declval<const S&>())), decltype(source::locator(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::tag(std::declval<const S&>())), uint8_t>, ::SCE::Generated::CodecZenohLocator::is_source<std::decay_t<decltype(source::locator(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::tag(std::declval<const S&>()))) && noexcept(source::locator(std::declval<const S&>())) && ::SCE::Generated::CodecZenohLocator::is_nothrow_source<std::decay_t<decltype(source::locator(std::declval<const S&>()))>>::value> {};

/// The encode of `CodecEmbedBasic` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecEmbedBasic: tag, locator");
        const auto& _r_locator = source::locator(src);
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(source::tag(src))); _e) return _e;
        if (auto _e = ::SCE::Generated::CodecZenohLocator::Wire::encode_source(_r_locator, w); _e) return _e;
        return std::nullopt;
    }
};

struct CodecEmbedBasic {
    CodecEmbedBasic() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecEmbedBasic(uint8_t tag, ::SCE::Generated::CodecZenohLocator::CodecZenohLocator locator)
        : tag_(tag), locator_(std::move(locator)) {}
    /// The `tag` element.
    [[nodiscard]] constexpr uint8_t tag() const noexcept { return tag_; }
    /// The `locator` element, the body it holds.
    [[nodiscard]] const ::SCE::Generated::CodecZenohLocator::CodecZenohLocator& locator() const noexcept { return locator_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecEmbedBasic> decode(::SCE::Forge::SceCursor& cursor) {
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
        uint8_t tag;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            tag = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        auto _emb_locator = ::SCE::Generated::CodecZenohLocator::CodecZenohLocator::decode(cursor);
        if (!_emb_locator.has_value()) return std::nullopt;
        auto locator = std::move(*_emb_locator);
        CodecEmbedBasic _decoded{
            /*tag=*/tag,
            /*locator=*/std::move(locator),
        };
        return _decoded;
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 265;

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
    uint8_t tag_{};
    ::SCE::Generated::CodecZenohLocator::CodecZenohLocator locator_{};
};

}  // namespace SCE::Generated::CodecEmbedBasic

#endif  // SCE_FORGE_CODEC_EMBED_BASIC_H
