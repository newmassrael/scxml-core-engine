// SCE-MAP: codec_zenoh_locator:26 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_LOCATOR_H
#define SCE_FORGE_CODEC_ZENOH_LOCATOR_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <string>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecZenohLocator {

/// The elements a value of `CodecZenohLocator` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
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

/// Whether `S` presents every element of `CodecZenohLocator`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::locator(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::locator(std::declval<const S&>())), ::SCE::Forge::Text<255>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<::SCE::Forge::Text<255>>(source::locator(std::declval<const S&>())))> {};

/// The encode of `CodecZenohLocator` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecZenohLocator: locator");
        const auto& _r_locator = source::locator(src);
        const ::SCE::Forge::Text<255> _e_locator = _r_locator;
        const uint64_t _d_locator_len = static_cast<uint64_t>(_e_locator.size());
        if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(_d_locator_len)); _e) return _e;
        if (auto _e = w.write_bytes(reinterpret_cast<const std::uint8_t*>(_e_locator.data()), _e_locator.size()); _e) return _e;
        return std::nullopt;
    }
};

struct CodecZenohLocator {
    CodecZenohLocator() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecZenohLocator(::SCE::Forge::Text<255> locator)
        : locator_(locator) {}
    /// The `locator` element.
    [[nodiscard]] ::SCE::Forge::Text<255> locator() const noexcept { return locator_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohLocator> decode(::SCE::Forge::SceCursor& cursor) {
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
        auto locator_len_opt = cursor.read_vle_u64();
        if (!locator_len_opt.has_value()) return std::nullopt;
        auto locator_len = static_cast<std::uint64_t>(*locator_len_opt);
        std::string locator;
        {
            const auto _len = locator_len;
            if (_len > 255) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange); return std::nullopt;
            }
            std::size_t _n = static_cast<std::size_t>(_len);
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            if (!::SCE::Forge::is_valid_utf8(raw, _n)) return std::nullopt;
            locator.assign(reinterpret_cast<const char*>(raw), _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        const auto _b_locator = ::SCE::Forge::Text<255>::make(locator);
        if (!_b_locator) {
            cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);
            return std::nullopt;
        }
        (void)locator_len;
        CodecZenohLocator _decoded{
            /*locator=*/*_b_locator,
        };
        return _decoded;
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 264;

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
    ::SCE::Forge::Bounded<std::string, 255> locator_{};
};

}  // namespace SCE::Generated::CodecZenohLocator

#endif  // SCE_FORGE_CODEC_ZENOH_LOCATOR_H
