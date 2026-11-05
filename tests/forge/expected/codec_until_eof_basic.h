// SCE-MAP: codec_until_eof_basic:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_UNTIL_EOF_BASIC_H
#define SCE_FORGE_CODEC_UNTIL_EOF_BASIC_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_repeat_elem.h"

namespace SCE::Generated::CodecUntilEofBasic {

/// The elements a value of `CodecUntilEofBasic` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_msgs {
void msgs() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.msgs())) -> decltype(s.msgs()) {
    return s.msgs();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(msgs(s))) -> decltype(msgs(s)) {
    return msgs(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_msgs
/// The `msgs` element of a source.
inline constexpr detail_msgs::Read msgs{};
}  // namespace source

/// Whether `R` is a list a source may present for `msgs`: held to bounds
/// within its own, of any type, each entry a source of the entry codec.
template <class R, class = void>
struct is_msgs_list : std::false_type {};
template <class R>
struct is_msgs_list<R, std::enable_if_t<::SCE::Forge::Detail::IsBoundedWithin<R, 64>::value>>
    : ::SCE::Generated::CodecRepeatElem::is_source<::SCE::Forge::Detail::ElementOf<R>> {};

/// Whether `S` presents every element of `CodecUntilEofBasic`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::msgs(std::declval<const S&>()))>>
    : std::conjunction<is_msgs_list<std::decay_t<decltype(source::msgs(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(source::msgs(std::declval<const S&>())) && ::SCE::Generated::CodecRepeatElem::is_nothrow_source<::SCE::Forge::Detail::ElementOf<std::decay_t<decltype(source::msgs(std::declval<const S&>()))>>>::value> {};

/// The encode of `CodecUntilEofBasic` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecUntilEofBasic: msgs");
        const auto& _r_msgs = source::msgs(src);
        for (const auto& _x : _r_msgs) {
            if (auto _e = ::SCE::Generated::CodecRepeatElem::Wire::encode_source(_x, w); _e) return _e;
        }
        return std::nullopt;
    }
};

struct CodecUntilEofBasic {
    CodecUntilEofBasic() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecUntilEofBasic(::SCE::Forge::List<::SCE::Generated::CodecRepeatElem::CodecRepeatElem, 64> msgs)
        : msgs_(msgs) {}
    /// The `msgs` element, the entries it holds.
    [[nodiscard]] ::SCE::Forge::List<::SCE::Generated::CodecRepeatElem::CodecRepeatElem, 64> msgs() const noexcept { return msgs_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecUntilEofBasic> decode(::SCE::Forge::SceCursor& cursor) {
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
        std::vector<::SCE::Generated::CodecRepeatElem::CodecRepeatElem> msgs;
        while (cursor.remaining() > 0) {
            if (msgs.size() > 63) {
                cursor.refuse(::SCE::Forge::CodecError::TooManyElements); return std::nullopt;
            }
            auto _elem = ::SCE::Generated::CodecRepeatElem::CodecRepeatElem::decode(cursor);
            if (!_elem.has_value()) return std::nullopt;
            msgs.push_back(*_elem);
        }
        const auto _b_msgs = ::SCE::Forge::List<::SCE::Generated::CodecRepeatElem::CodecRepeatElem, 64>::make(msgs);
        if (!_b_msgs) {
            cursor.refuse(::SCE::Forge::CodecError::TooManyElements);
            return std::nullopt;
        }
        CodecUntilEofBasic _decoded{
            /*msgs=*/*_b_msgs,
        };
        return _decoded;
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 128;

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
    ::SCE::Forge::Bounded<std::vector<::SCE::Generated::CodecRepeatElem::CodecRepeatElem>, 64> msgs_{};
};

}  // namespace SCE::Generated::CodecUntilEofBasic

#endif  // SCE_FORGE_CODEC_UNTIL_EOF_BASIC_H
