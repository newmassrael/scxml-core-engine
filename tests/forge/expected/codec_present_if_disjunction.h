// SCE-MAP: codec_present_if_disjunction:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_PRESENT_IF_DISJUNCTION_H
#define SCE_FORGE_CODEC_PRESENT_IF_DISJUNCTION_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecPresentIfDisjunction {

/// The elements a value of `CodecPresentIfDisjunction` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_flags {
void flags() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.flags())) -> decltype(s.flags()) {
    return s.flags();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(flags(s))) -> decltype(flags(s)) {
    return flags(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_flags
/// The `flags` element of a source.
inline constexpr detail_flags::Read flags{};
namespace detail_flags_wants_a_or_wants_b {
void flags_wants_a_or_wants_b() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.flags_wants_a_or_wants_b())) -> decltype(s.flags_wants_a_or_wants_b()) {
    return s.flags_wants_a_or_wants_b();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(flags_wants_a_or_wants_b(s))) -> decltype(flags_wants_a_or_wants_b(s)) {
    return flags_wants_a_or_wants_b(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_flags_wants_a_or_wants_b
/// The `flags_wants_a_or_wants_b` element of a source.
inline constexpr detail_flags_wants_a_or_wants_b::Read flags_wants_a_or_wants_b{};
namespace detail_seq {
void seq() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.seq())) -> decltype(s.seq()) {
    return s.seq();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(seq(s))) -> decltype(seq(s)) {
    return seq(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_seq
/// The `seq` element of a source.
inline constexpr detail_seq::Read seq{};
}  // namespace source

/// The fields `flags.wants_a || flags.wants_b` decides (docs/adr/0065 §6 R4b, R4c) present while `flags.wants_a` holds and `flags.wants_b` does not hold.
class CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsA {
public:
    CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsA() = default;
    /// The value of each field, in declaration order.
    constexpr CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsA(uint16_t seq) noexcept
        : seq_(seq) {}
    /// The `seq` field.
    [[nodiscard]] constexpr uint16_t seq() const noexcept { return seq_; }

private:
    uint16_t seq_{};
};

/// The fields `flags.wants_a || flags.wants_b` decides (docs/adr/0065 §6 R4b, R4c) present while `flags.wants_a` does not hold and `flags.wants_b` holds.
class CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsB {
public:
    CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsB() = default;
    /// The value of each field, in declaration order.
    constexpr CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsB(uint16_t seq) noexcept
        : seq_(seq) {}
    /// The `seq` field.
    [[nodiscard]] constexpr uint16_t seq() const noexcept { return seq_; }

private:
    uint16_t seq_{};
};

/// The fields `flags.wants_a || flags.wants_b` decides (docs/adr/0065 §6 R4b, R4c) present while `flags.wants_a` holds and `flags.wants_b` holds.
class CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsAWantsB {
public:
    CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsAWantsB() = default;
    /// The value of each field, in declaration order.
    constexpr CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsAWantsB(uint16_t seq) noexcept
        : seq_(seq) {}
    /// The `seq` field.
    [[nodiscard]] constexpr uint16_t seq() const noexcept { return seq_; }

private:
    uint16_t seq_{};
};

/// The fields `flags.wants_a || flags.wants_b` decides (docs/adr/0065 §6 R4b, R4c): a value holds one
/// arm or none of them, and its encode derives the gates from which.
using CodecPresentIfDisjunctionFlagsWantsAOrWantsB = std::variant<CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsA, CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsB, CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsAWantsB>;

/// Whether `G` presents every field of `CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsA`, each convertible to its type.
template <class G, class = void>
struct is_flags_wants_a_or_wants_b_wants_a_members : std::false_type {};
template <class G>
struct is_flags_wants_a_or_wants_b_wants_a_members<G, std::void_t<decltype(source::seq(std::declval<const G&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::seq(std::declval<const G&>())), uint16_t>> {};
/// Whether `G` presents every field of `CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsB`, each convertible to its type.
template <class G, class = void>
struct is_flags_wants_a_or_wants_b_wants_b_members : std::false_type {};
template <class G>
struct is_flags_wants_a_or_wants_b_wants_b_members<G, std::void_t<decltype(source::seq(std::declval<const G&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::seq(std::declval<const G&>())), uint16_t>> {};
/// Whether `G` presents every field of `CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsAWantsB`, each convertible to its type.
template <class G, class = void>
struct is_flags_wants_a_or_wants_b_wants_awants_b_members : std::false_type {};
template <class G>
struct is_flags_wants_a_or_wants_b_wants_awants_b_members<G, std::void_t<decltype(source::seq(std::declval<const G&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::seq(std::declval<const G&>())), uint16_t>> {};
/// Whether `R` is what a source may present for `flags_wants_a_or_wants_b`, the fields `flags.wants_a || flags.wants_b`
/// decides: a `std::optional` of a `std::variant` of a type for each arm, in order, that presents its fields.
template <class R>
struct is_flags_wants_a_or_wants_b_group : std::false_type {};
template <class G0, class G1, class G2>
struct is_flags_wants_a_or_wants_b_group<std::optional<std::variant<G0, G1, G2>>>
    : std::conjunction<is_flags_wants_a_or_wants_b_wants_a_members<G0>, is_flags_wants_a_or_wants_b_wants_b_members<G1>, is_flags_wants_a_or_wants_b_wants_awants_b_members<G2>> {};

/// Whether `S` presents every element of `CodecPresentIfDisjunction`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::flags(std::declval<const S&>())), decltype(source::flags_wants_a_or_wants_b(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::flags(std::declval<const S&>())), uint8_t>, is_flags_wants_a_or_wants_b_group<std::decay_t<decltype(source::flags_wants_a_or_wants_b(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::flags(std::declval<const S&>()))) && noexcept(source::flags_wants_a_or_wants_b(std::declval<const S&>())) && noexcept(::SCE::Forge::Detail::converts_to<uint16_t>(source::seq(std::declval<const std::variant_alternative_t<0, typename std::decay_t<decltype(source::flags_wants_a_or_wants_b(std::declval<const S&>()))>::value_type>&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint16_t>(source::seq(std::declval<const std::variant_alternative_t<1, typename std::decay_t<decltype(source::flags_wants_a_or_wants_b(std::declval<const S&>()))>::value_type>&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint16_t>(source::seq(std::declval<const std::variant_alternative_t<2, typename std::decay_t<decltype(source::flags_wants_a_or_wants_b(std::declval<const S&>()))>::value_type>&>())))> {};

/// The encode of `CodecPresentIfDisjunction` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecPresentIfDisjunction: flags, flags_wants_a_or_wants_b");
        const auto& _r_flags_wants_a_or_wants_b = source::flags_wants_a_or_wants_b(src);
        std::optional<uint16_t> _e_seq;
        if (_r_flags_wants_a_or_wants_b) {
            if (const auto* _a = std::get_if<0>(&*_r_flags_wants_a_or_wants_b)) {
                _e_seq = static_cast<uint16_t>(source::seq(*_a));
            }
            if (const auto* _a = std::get_if<1>(&*_r_flags_wants_a_or_wants_b)) {
                _e_seq = static_cast<uint16_t>(source::seq(*_a));
            }
            if (const auto* _a = std::get_if<2>(&*_r_flags_wants_a_or_wants_b)) {
                _e_seq = static_cast<uint16_t>(source::seq(*_a));
            }
        }
        std::uint64_t _f_flags = static_cast<std::uint64_t>(source::flags(src));
        _f_flags = (_f_flags & ~0x1ull) | static_cast<std::uint64_t>(static_cast<std::uint8_t>(!_r_flags_wants_a_or_wants_b.has_value() ? 0 : ((_r_flags_wants_a_or_wants_b->index() == 0u || _r_flags_wants_a_or_wants_b->index() == 2u) ? 1 : 0)));
        _f_flags = (_f_flags & ~0x2ull) | (static_cast<std::uint64_t>(static_cast<std::uint8_t>(!_r_flags_wants_a_or_wants_b.has_value() ? 0 : ((_r_flags_wants_a_or_wants_b->index() == 1u || _r_flags_wants_a_or_wants_b->index() == 2u) ? 1 : 0))) << 1);
        const uint8_t _c_flags = static_cast<uint8_t>(_f_flags);
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_c_flags)); _e) return _e;
        if (_e_seq) {
            if (auto _e = w.write_u16_be(static_cast<std::uint16_t>((*_e_seq))); _e) return _e;
        }
        return std::nullopt;
    }
};

struct CodecPresentIfDisjunction {
    CodecPresentIfDisjunction() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    constexpr CodecPresentIfDisjunction(uint8_t flags, std::optional<CodecPresentIfDisjunctionFlagsWantsAOrWantsB> flags_wants_a_or_wants_b) noexcept
        : flags_(flags), flags_wants_a_or_wants_b_(flags_wants_a_or_wants_b) {}
    /// The `flags` element.
    [[nodiscard]] constexpr uint8_t flags() const noexcept { return flags_; }
    /// The fields `flags.wants_a || flags.wants_b` decides, where its gate leaves them in.
    [[nodiscard]] constexpr const std::optional<CodecPresentIfDisjunctionFlagsWantsAOrWantsB>& flags_wants_a_or_wants_b() const noexcept { return flags_wants_a_or_wants_b_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecPresentIfDisjunction> decode(::SCE::Forge::SceCursor& cursor) {
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
        uint8_t flags;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            flags = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<uint16_t> seq;
        if ((flags & 0x01) != 0 || (flags & 0x02) != 0) {
            const std::uint8_t* raw = cursor.peek_slice(2);
            if (raw == nullptr) return std::nullopt;
            seq = static_cast<uint16_t>(static_cast<uint16_t>((static_cast<uint16_t>(raw[0]) << 8) | raw[1]));
            if (!cursor.advance(2)) return std::nullopt;
        }
        std::optional<CodecPresentIfDisjunctionFlagsWantsAOrWantsB> _g_flags_wants_a_or_wants_b;
        if ((((flags >> 0) & 0x1) != 0) && (((flags >> 1) & 0x1) == 0)) {
            _g_flags_wants_a_or_wants_b.emplace(std::in_place_index<0>, *seq);
        } else if ((((flags >> 0) & 0x1) == 0) && (((flags >> 1) & 0x1) != 0)) {
            _g_flags_wants_a_or_wants_b.emplace(std::in_place_index<1>, *seq);
        } else if ((((flags >> 0) & 0x1) != 0) && (((flags >> 1) & 0x1) != 0)) {
            _g_flags_wants_a_or_wants_b.emplace(std::in_place_index<2>, *seq);
        }
        CodecPresentIfDisjunction _decoded{
            /*flags=*/flags,
            /*flags_wants_a_or_wants_b=*/_g_flags_wants_a_or_wants_b,
        };
        return _decoded;
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool wants_a() const noexcept {
        return (this->flags_ & 0x01) != 0;
    }

    void set_wants_a(bool v) noexcept {
        if (v) {
            this->flags_ = static_cast<uint8_t>(this->flags_ | 0x01);
        } else {
            this->flags_ = static_cast<uint8_t>(this->flags_ & static_cast<uint8_t>(~0x01));
        }
    }

    bool wants_b() const noexcept {
        return (this->flags_ & 0x02) != 0;
    }

    void set_wants_b(bool v) noexcept {
        if (v) {
            this->flags_ = static_cast<uint8_t>(this->flags_ | 0x02);
        } else {
            this->flags_ = static_cast<uint8_t>(this->flags_ & static_cast<uint8_t>(~0x02));
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 3;

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
    uint8_t flags_{};
    std::optional<CodecPresentIfDisjunctionFlagsWantsAOrWantsB> flags_wants_a_or_wants_b_{};
};

}  // namespace SCE::Generated::CodecPresentIfDisjunction

#endif  // SCE_FORGE_CODEC_PRESENT_IF_DISJUNCTION_H
