// SCE-MAP: codec_repeat_basic:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_REPEAT_BASIC_H
#define SCE_FORGE_CODEC_REPEAT_BASIC_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_repeat_elem.h"

namespace SCE::Generated::CodecRepeatBasic {

/// The elements a value of `CodecRepeatBasic` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_frags {
void frags() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.frags())) -> decltype(s.frags()) {
    return s.frags();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(frags(s))) -> decltype(frags(s)) {
    return frags(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_frags
/// The `frags` element of a source.
inline constexpr detail_frags::Read frags{};
}  // namespace source

/// Whether `R` is a list a source may present for `frags`: held to bounds
/// within its own, of any type, each entry a source of the entry codec.
template <class R, class = void>
struct is_frags_list : std::false_type {};
template <class R>
struct is_frags_list<R, std::enable_if_t<::SCE::Forge::Detail::IsBoundedWithin<R, 32>::value>>
    : ::SCE::Generated::CodecRepeatElem::is_source<::SCE::Forge::Detail::ElementOf<R>> {};

/// Whether `S` presents every element of `CodecRepeatBasic`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::frags(std::declval<const S&>()))>>
    : std::conjunction<is_frags_list<std::decay_t<decltype(source::frags(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(source::frags(std::declval<const S&>())) && ::SCE::Generated::CodecRepeatElem::is_nothrow_source<::SCE::Forge::Detail::ElementOf<std::decay_t<decltype(source::frags(std::declval<const S&>()))>>>::value> {};

/// The encode of `CodecRepeatBasic` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecRepeatBasic: frags");
        const auto& _r_frags = source::frags(src);
        const uint8_t _d_num_frags = static_cast<uint8_t>(_r_frags.size());
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_d_num_frags)); _e) return _e;
        for (const auto& _x : _r_frags) {
            if (auto _e = ::SCE::Generated::CodecRepeatElem::Wire::encode_source(_x, w); _e) return _e;
        }
        return std::nullopt;
    }
};

struct CodecRepeatBasic {
    CodecRepeatBasic() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecRepeatBasic(::SCE::Forge::List<::SCE::Generated::CodecRepeatElem::CodecRepeatElem, 32> frags)
        : frags_(frags) {}
    /// The `frags` element, the entries it holds.
    [[nodiscard]] ::SCE::Forge::List<::SCE::Generated::CodecRepeatElem::CodecRepeatElem, 32> frags() const noexcept { return frags_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecRepeatBasic> decode(::SCE::Forge::SceCursor& cursor) {
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
        uint8_t num_frags;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            num_frags = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::vector<::SCE::Generated::CodecRepeatElem::CodecRepeatElem> frags;
        if (num_frags > 32) {
            cursor.refuse(::SCE::Forge::CodecError::TooManyElements); return std::nullopt;
        }
        frags.reserve(cursor.entries_backed(num_frags));
        for (auto _i = decltype(num_frags){0}; _i < num_frags; ++_i) {
            auto _elem = ::SCE::Generated::CodecRepeatElem::CodecRepeatElem::decode(cursor);
            if (!_elem.has_value()) return std::nullopt;
            frags.push_back(*_elem);
        }
        const auto _b_frags = ::SCE::Forge::List<::SCE::Generated::CodecRepeatElem::CodecRepeatElem, 32>::make(frags);
        if (!_b_frags) {
            cursor.refuse(::SCE::Forge::CodecError::TooManyElements);
            return std::nullopt;
        }
        (void)num_frags;
        CodecRepeatBasic _decoded{
            /*frags=*/*_b_frags,
        };
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
    ::SCE::Forge::Bounded<std::vector<::SCE::Generated::CodecRepeatElem::CodecRepeatElem>, 32> frags_{};
};

}  // namespace SCE::Generated::CodecRepeatBasic

#endif  // SCE_FORGE_CODEC_REPEAT_BASIC_H
