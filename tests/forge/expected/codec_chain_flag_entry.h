// SCE-MAP: codec_chain_flag_entry:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_CHAIN_FLAG_ENTRY_H
#define SCE_FORGE_CODEC_CHAIN_FLAG_ENTRY_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecChainFlagEntry {

/// The elements a value of `CodecChainFlagEntry` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_header {
void header() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.header())) -> decltype(s.header()) {
    return s.header();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(header(s))) -> decltype(header(s)) {
    return header(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_header
/// The `header` element of a source.
inline constexpr detail_header::Read header{};
namespace detail_wide {
void wide() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.wide())) -> decltype(s.wide()) {
    return s.wide();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(wide(s))) -> decltype(wide(s)) {
    return wide(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_wide
/// The `wide` element of a source.
inline constexpr detail_wide::Read wide{};
namespace detail_short_value {
void short_value() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.short_value())) -> decltype(s.short_value()) {
    return s.short_value();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(short_value(s))) -> decltype(short_value(s)) {
    return short_value(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_short_value
/// The `short_value` element of a source.
inline constexpr detail_short_value::Read short_value{};
namespace detail_long_value {
void long_value() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.long_value())) -> decltype(s.long_value()) {
    return s.long_value();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(long_value(s))) -> decltype(long_value(s)) {
    return long_value(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_long_value
/// The `long_value` element of a source.
inline constexpr detail_long_value::Read long_value{};
}  // namespace source

/// The fields `wide` decides (docs/adr/0065 §6 R4b, R4c) present while `wide` holds.
class CodecChainFlagEntryWideWide {
public:
    CodecChainFlagEntryWideWide() = default;
    /// The value of each field, in declaration order.
    constexpr CodecChainFlagEntryWideWide(uint16_t long_value) noexcept
        : long_value_(long_value) {}
    /// The `long_value` field.
    [[nodiscard]] constexpr uint16_t long_value() const noexcept { return long_value_; }

private:
    uint16_t long_value_{};
};

/// The fields `wide` decides (docs/adr/0065 §6 R4b, R4c) present while `wide` does not hold.
class CodecChainFlagEntryWideNotWide {
public:
    CodecChainFlagEntryWideNotWide() = default;
    /// The value of each field, in declaration order.
    constexpr CodecChainFlagEntryWideNotWide(uint8_t short_value) noexcept
        : short_value_(short_value) {}
    /// The `short_value` field.
    [[nodiscard]] constexpr uint8_t short_value() const noexcept { return short_value_; }

private:
    uint8_t short_value_{};
};

/// The fields `wide` decides (docs/adr/0065 §6 R4b, R4c): a value holds one
/// arm, and its encode derives the gates from which.
using CodecChainFlagEntryWide = std::variant<CodecChainFlagEntryWideWide, CodecChainFlagEntryWideNotWide>;

/// Whether `G` presents every field of `CodecChainFlagEntryWideWide`, each convertible to its type.
template <class G, class = void>
struct is_wide_wide_members : std::false_type {};
template <class G>
struct is_wide_wide_members<G, std::void_t<decltype(source::long_value(std::declval<const G&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::long_value(std::declval<const G&>())), uint16_t>> {};
/// Whether `G` presents every field of `CodecChainFlagEntryWideNotWide`, each convertible to its type.
template <class G, class = void>
struct is_wide_not_wide_members : std::false_type {};
template <class G>
struct is_wide_not_wide_members<G, std::void_t<decltype(source::short_value(std::declval<const G&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::short_value(std::declval<const G&>())), uint8_t>> {};
/// Whether `R` is what a source may present for `wide`, the fields `wide`
/// decides: a `std::variant` of a type for each arm, in order, that presents its fields.
template <class R>
struct is_wide_group : std::false_type {};
template <class G0, class G1>
struct is_wide_group<std::variant<G0, G1>>
    : std::conjunction<is_wide_wide_members<G0>, is_wide_not_wide_members<G1>> {};

/// Whether `S` presents every element of `CodecChainFlagEntry`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::header(std::declval<const S&>())), decltype(source::wide(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::header(std::declval<const S&>())), uint8_t>, is_wide_group<std::decay_t<decltype(source::wide(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::header(std::declval<const S&>()))) && noexcept(source::wide(std::declval<const S&>())) && noexcept(::SCE::Forge::Detail::converts_to<uint16_t>(source::long_value(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(source::wide(std::declval<const S&>()))>>&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::short_value(std::declval<const std::variant_alternative_t<1, std::decay_t<decltype(source::wide(std::declval<const S&>()))>>&>())))> {};

/// One-bit flags of `CodecChainFlagEntry` a parent may set in place of the ones a value
/// presents, such as the flag a chain whose entries are `CodecChainFlagEntry` ends them
/// on, which the chain derives from where each entry stands.
struct Overrides {
    /// The `Z` flag of `header`, where a parent sets it.
    std::optional<bool> header_z;
};

/// The encode of `CodecChainFlagEntry` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w, const Overrides& overrides = Overrides{}) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecChainFlagEntry: header, wide");
        const auto& _r_wide = source::wide(src);
        std::optional<uint8_t> _e_short_value;
        std::optional<uint16_t> _e_long_value;
        if (const auto* _a = std::get_if<0>(&_r_wide)) {
            _e_long_value = static_cast<uint16_t>(source::long_value(*_a));
        }
        if (const auto* _a = std::get_if<1>(&_r_wide)) {
            _e_short_value = static_cast<uint8_t>(source::short_value(*_a));
        }
        std::uint64_t _f_header = static_cast<std::uint64_t>(source::header(src));
        if (overrides.header_z.has_value()) {
            _f_header = (_f_header & ~0x80ull) | (static_cast<std::uint64_t>(*overrides.header_z) << 7);
        }
        const uint8_t _c_header = static_cast<uint8_t>(_f_header);
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_c_header)); _e) return _e;
        if (_e_short_value) {
            if (auto _e = w.write_u8(static_cast<std::uint8_t>((*_e_short_value))); _e) return _e;
        }
        if (_e_long_value) {
            if (auto _e = w.write_u16_be(static_cast<std::uint16_t>((*_e_long_value))); _e) return _e;
        }
        return std::nullopt;
    }
};

/// The flag input `wide` a source of `CodecChainFlagEntry` says, which its encode is not
/// handed (docs/adr/0065 §6 R4a).
template <class S>
[[nodiscard]] std::uint8_t input_wide(const S& src) noexcept(is_nothrow_source<S>::value) {
    static_assert(is_source<S>::value, "S presents every element of CodecChainFlagEntry");
    return static_cast<std::uint8_t>((source::wide(src).index() == 0u) ? 1 : 0);
}

struct CodecChainFlagEntry {
    CodecChainFlagEntry() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    constexpr CodecChainFlagEntry(uint8_t header, CodecChainFlagEntryWide wide) noexcept
        : header_(header), wide_(wide) {}
    /// The `header` element.
    [[nodiscard]] constexpr uint8_t header() const noexcept { return header_; }
    /// The fields `wide` decides, as the arm the value holds.
    [[nodiscard]] constexpr const CodecChainFlagEntryWide& wide() const noexcept { return wide_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecChainFlagEntry> decode(::SCE::Forge::SceCursor& cursor, std::uint8_t wide) {
        // Declared-but-unconsumed flag inputs: defensive (void) suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly under -Wunused.
        (void)wide;
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
        uint8_t header;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            header = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<uint8_t> short_value;
        if ((wide & 0x01) == 0) {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            short_value = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<uint16_t> long_value;
        if ((wide & 0x01) != 0) {
            const std::uint8_t* raw = cursor.peek_slice(2);
            if (raw == nullptr) return std::nullopt;
            long_value = static_cast<uint16_t>(static_cast<uint16_t>((static_cast<uint16_t>(raw[0]) << 8) | raw[1]));
            if (!cursor.advance(2)) return std::nullopt;
        }
        CodecChainFlagEntryWide _g_wide;
        if (((wide & 0x01) != 0)) {
            _g_wide.emplace<0>(*long_value);
        } else if (((wide & 0x01) == 0)) {
            _g_wide.emplace<1>(*short_value);
        }
        CodecChainFlagEntry _decoded{
            /*header=*/header,
            /*wide=*/_g_wide,
        };
        return _decoded;
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    uint8_t kind() const noexcept {
        return static_cast<uint8_t>(
            (this->header_ >> 0) & static_cast<uint8_t>(0x0F)
        );
    }

    void set_kind(uint8_t v) noexcept {
        const uint8_t _shifted_mask =
            static_cast<uint8_t>(
                static_cast<uint8_t>(0x0F) << 0
            );
        const uint8_t _val =
            static_cast<uint8_t>(
                (static_cast<uint8_t>(v) & static_cast<uint8_t>(0x0F)) << 0
            );
        this->header_ = static_cast<uint8_t>(
            (this->header_ & static_cast<uint8_t>(~_shifted_mask)) | _val
        );
    }

    bool z() const noexcept {
        return (this->header_ & 0x80) != 0;
    }

    void set_z(bool v) noexcept {
        if (v) {
            this->header_ = static_cast<uint8_t>(this->header_ | 0x80);
        } else {
            this->header_ = static_cast<uint8_t>(this->header_ & static_cast<uint8_t>(~0x80));
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 4;

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
    uint8_t header_{};
    CodecChainFlagEntryWide wide_{std::in_place_index_t<1>{}};
};

}  // namespace SCE::Generated::CodecChainFlagEntry

#endif  // SCE_FORGE_CODEC_CHAIN_FLAG_ENTRY_H
