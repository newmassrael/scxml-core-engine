// SCE-MAP: codec_repeat_present_if_basic:37 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_REPEAT_PRESENT_IF_BASIC_H
#define SCE_FORGE_CODEC_REPEAT_PRESENT_IF_BASIC_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_repeat_elem.h"

namespace SCE::Generated::CodecRepeatPresentIfBasic {

/// The elements a value of `CodecRepeatPresentIfBasic` presents, each read by name: the
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
namespace detail_elems {
void elems() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.elems())) -> decltype(s.elems()) {
    return s.elems();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(elems(s))) -> decltype(elems(s)) {
    return elems(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_elems
/// The `elems` element of a source.
inline constexpr detail_elems::Read elems{};
}  // namespace source

/// Whether `R` is a list a source may present for `elems`: held to bounds
/// within its own, of any type, each entry a source of the entry codec.
template <class R, class = void>
struct is_elems_list : std::false_type {};
template <class R>
struct is_elems_list<R, std::enable_if_t<::SCE::Forge::Detail::IsBoundedWithin<R, 32>::value>>
    : ::SCE::Generated::CodecRepeatElem::is_source<::SCE::Forge::Detail::ElementOf<R>> {};

/// Whether `R` is what a source may present for `elems`, a list a gate
/// decides: a `std::optional` of a list `is_elems_list` accepts.
template <class R>
struct is_elems_present_list : std::false_type {};
template <class L>
struct is_elems_present_list<std::optional<L>> : is_elems_list<L> {};

/// Whether `S` presents every element of `CodecRepeatPresentIfBasic`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::carrier(std::declval<const S&>())), decltype(source::elems(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::carrier(std::declval<const S&>())), uint8_t>, is_elems_present_list<std::decay_t<decltype(source::elems(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::carrier(std::declval<const S&>()))) && noexcept(source::elems(std::declval<const S&>())) && ::SCE::Generated::CodecRepeatElem::is_nothrow_source<::SCE::Forge::Detail::ElementOf<typename std::decay_t<decltype(source::elems(std::declval<const S&>()))>::value_type>>::value> {};

/// The encode of `CodecRepeatPresentIfBasic` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecRepeatPresentIfBasic: carrier, elems");
        const auto& _r_elems = source::elems(src);
        const uint8_t _d_num_elems = _r_elems.has_value() ? static_cast<uint8_t>((*_r_elems).size()) : uint8_t{0};
        std::uint64_t _f_carrier = static_cast<std::uint64_t>(source::carrier(src));
        _f_carrier = (_f_carrier & ~0x1ull) | static_cast<std::uint64_t>(_r_elems.has_value());
        const uint8_t _c_carrier = static_cast<uint8_t>(_f_carrier);
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_c_carrier)); _e) return _e;
        if (((static_cast<std::uint64_t>(_c_carrier) & 0x1ull) != 0)) {
            if (auto _e = w.write_u8(static_cast<std::uint8_t>(_d_num_elems)); _e) return _e;
        }
        if (_r_elems) {
            for (const auto& _x : (*_r_elems)) {
                if (auto _e = ::SCE::Generated::CodecRepeatElem::Wire::encode_source(_x, w); _e) return _e;
            }
        }
        return std::nullopt;
    }
};

struct CodecRepeatPresentIfBasic {
    CodecRepeatPresentIfBasic() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecRepeatPresentIfBasic(uint8_t carrier, std::optional<::SCE::Forge::List<::SCE::Generated::CodecRepeatElem::CodecRepeatElem, 32>> elems)
        : carrier_(carrier), elems_(elems) {}
    /// The `carrier` element.
    [[nodiscard]] constexpr uint8_t carrier() const noexcept { return carrier_; }
    /// The `elems` element, the entries it holds where a gate leaves them in.
    [[nodiscard]] std::optional<::SCE::Forge::List<::SCE::Generated::CodecRepeatElem::CodecRepeatElem, 32>> elems() const noexcept { if (!elems_) { return std::nullopt; } return ::SCE::Forge::List<::SCE::Generated::CodecRepeatElem::CodecRepeatElem, 32>(*elems_); }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecRepeatPresentIfBasic> decode(::SCE::Forge::SceCursor& cursor) {
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
        std::optional<uint8_t> num_elems;
        if ((carrier & 0x01) != 0) {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            num_elems = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<std::vector<::SCE::Generated::CodecRepeatElem::CodecRepeatElem>> elems;
        if ((carrier & 0x01) != 0) {
            auto _n = num_elems.value();
            if (_n > 32) {
                cursor.refuse(::SCE::Forge::CodecError::TooManyElements); return std::nullopt;
            }
            std::vector<::SCE::Generated::CodecRepeatElem::CodecRepeatElem> _list;
            _list.reserve(cursor.entries_backed(_n));
            for (auto _i = decltype(_n){0}; _i < _n; ++_i) {
                auto _elem = ::SCE::Generated::CodecRepeatElem::CodecRepeatElem::decode(cursor);
                if (!_elem.has_value()) return std::nullopt;
                _list.push_back(*_elem);
            }
            elems = std::move(_list);
        }
        std::optional<::SCE::Forge::List<::SCE::Generated::CodecRepeatElem::CodecRepeatElem, 32>> _b_elems;
        if (elems) {
            _b_elems = ::SCE::Forge::List<::SCE::Generated::CodecRepeatElem::CodecRepeatElem, 32>::make(*elems);
            if (!_b_elems) {
                cursor.refuse(::SCE::Forge::CodecError::TooManyElements);
                return std::nullopt;
            }
        }
        (void)num_elems;
        CodecRepeatPresentIfBasic _decoded{
            /*carrier=*/carrier,
            /*elems=*/_b_elems,
        };
        return _decoded;
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool has_list() const noexcept {
        return (this->carrier_ & 0x01) != 0;
    }

    void set_has_list(bool v) noexcept {
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
    static constexpr std::size_t MAX_ENCODED_BYTES = 66;

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
    std::optional<::SCE::Forge::Bounded<std::vector<::SCE::Generated::CodecRepeatElem::CodecRepeatElem>, 32>> elems_{};
};

}  // namespace SCE::Generated::CodecRepeatPresentIfBasic

#endif  // SCE_FORGE_CODEC_REPEAT_PRESENT_IF_BASIC_H
