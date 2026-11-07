// SCE-MAP: codec_zenoh_value_entry:14 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_VALUE_ENTRY_H
#define SCE_FORGE_CODEC_ZENOH_VALUE_ENTRY_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <variant>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_zenoh_ext_unit.h"
#include "codec_zenoh_ext_zint.h"
#include "codec_zenoh_value_zbuf.h"

namespace SCE::Generated::CodecZenohValueEntry {

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. `std::variant` carries one of N arm bodies
// (each an imported codec type); the optional Default arm is a small
// struct that bundles the runtime tag value with the catch-all body.
struct CodecZenohValueEntryDefault {
    uint8_t tag;
    ::SCE::Generated::CodecZenohExtUnit::CodecZenohExtUnit body;
};
using CodecZenohValueEntryVariant = std::variant<
    ::SCE::Generated::CodecZenohExtUnit::CodecZenohExtUnit,
    ::SCE::Generated::CodecZenohExtZint::CodecZenohExtZint,
    ::SCE::Generated::CodecZenohValueZbuf::CodecZenohValueZbuf,
    CodecZenohValueEntryDefault
>;

/// The elements a value of `CodecZenohValueEntry` presents, each read by name: the
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
namespace detail_body {
void body() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.body())) -> decltype(s.body()) {
    return s.body();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(body(s))) -> decltype(body(s)) {
    return body(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_body
/// The `body` element of a source.
inline constexpr detail_body::Read body{};
}  // namespace source

/// Whether `D` is the catch-all arm of a source's body: the tag it holds,
/// and its body, a source of the catch-all's codec.
template <class D, class = void>
struct is_default_arm : std::false_type {};
template <class D>
struct is_default_arm<D, std::void_t<decltype(std::declval<const D&>().tag), decltype(std::declval<const D&>().body)>>
    : std::conjunction<std::is_convertible<decltype(std::declval<const D&>().tag), std::uint64_t>,
                       ::SCE::Generated::CodecZenohExtUnit::is_source<std::decay_t<decltype(std::declval<const D&>().body)>>> {};

/// Whether `R` is a body a source may present: the arm it holds, of a
/// `std::variant` of a source of each arm's codec in turn.
template <class R, class = void>
struct is_body : std::false_type {};
template <class R>
struct is_body<R, std::enable_if_t<::SCE::Forge::Detail::IsVariantOf<R, 4>::value>>
    : std::conjunction<::SCE::Generated::CodecZenohExtUnit::is_source<std::variant_alternative_t<0, R>>, ::SCE::Generated::CodecZenohExtZint::is_source<std::variant_alternative_t<1, R>>, ::SCE::Generated::CodecZenohValueZbuf::is_source<std::variant_alternative_t<2, R>>, is_default_arm<std::variant_alternative_t<3, R>>> {};

/// Whether `S` presents every element of `CodecZenohValueEntry`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::header(std::declval<const S&>())), decltype(source::body(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::header(std::declval<const S&>())), uint8_t>, is_body<std::decay_t<decltype(source::body(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::header(std::declval<const S&>()))) && noexcept(source::body(std::declval<const S&>())) && ::SCE::Generated::CodecZenohExtUnit::is_nothrow_source<std::variant_alternative_t<0, std::decay_t<decltype(source::body(std::declval<const S&>()))>>>::value && ::SCE::Generated::CodecZenohExtZint::is_nothrow_source<std::variant_alternative_t<1, std::decay_t<decltype(source::body(std::declval<const S&>()))>>>::value && ::SCE::Generated::CodecZenohValueZbuf::is_nothrow_source<std::variant_alternative_t<2, std::decay_t<decltype(source::body(std::declval<const S&>()))>>>::value && ::SCE::Generated::CodecZenohExtUnit::is_nothrow_source<std::decay_t<decltype(std::declval<const std::variant_alternative_t<3, std::decay_t<decltype(source::body(std::declval<const S&>()))>>&>().body)>>::value> {};

/// One-bit flags of `CodecZenohValueEntry` a parent may set in place of the ones a value
/// presents, such as the flag a chain whose entries are `CodecZenohValueEntry` ends them
/// on, which the chain derives from where each entry stands.
struct Overrides {
    /// The `M` flag of `header`, where a parent sets it.
    std::optional<bool> header_m;
    /// The `Z` flag of `header`, where a parent sets it.
    std::optional<bool> header_z;
};

/// The encode of `CodecZenohValueEntry` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w, const Overrides& overrides = Overrides{}) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecZenohValueEntry: header, body");
        const auto& _r_body = source::body(src);
        std::uint64_t _t_body = 0;
        switch (_r_body.index()) {
            case 0: _t_body = 0ull; break;
            case 1: _t_body = 1ull; break;
            case 2: _t_body = 2ull; break;
            case 3: _t_body = static_cast<std::uint64_t>(std::get_if<3>(&_r_body)->tag); break;
            default: break;
        }
        std::uint64_t _f_header = static_cast<std::uint64_t>(source::header(src));
        _f_header = (_f_header & ~0x60ull) | ((_t_body & 0x3ull) << 5);
        if (overrides.header_m.has_value()) {
            _f_header = (_f_header & ~0x10ull) | (static_cast<std::uint64_t>(*overrides.header_m) << 4);
        }
        if (overrides.header_z.has_value()) {
            _f_header = (_f_header & ~0x80ull) | (static_cast<std::uint64_t>(*overrides.header_z) << 7);
        }
        const uint8_t _c_header = static_cast<uint8_t>(_f_header);
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_c_header)); _e) return _e;
        switch (_r_body.index()) {
            case 0:
                if (auto _e = ::SCE::Generated::CodecZenohExtUnit::Wire::encode_source(*std::get_if<0>(&_r_body), w); _e) return _e;
                break;
            case 1:
                if (auto _e = ::SCE::Generated::CodecZenohExtZint::Wire::encode_source(*std::get_if<1>(&_r_body), w); _e) return _e;
                break;
            case 2:
                if (auto _e = ::SCE::Generated::CodecZenohValueZbuf::Wire::encode_source(*std::get_if<2>(&_r_body), w); _e) return _e;
                break;
            case 3:
                if (auto _e = ::SCE::Generated::CodecZenohExtUnit::Wire::encode_source(std::get_if<3>(&_r_body)->body, w); _e) return _e;
                break;
            default: break;
        }
        return std::nullopt;
    }
};

/// The flag input `after_shm` a source of `CodecZenohValueEntry` says, which its encode is not
/// handed (docs/adr/0065 §6 R4a), and nothing where the value says nothing of it.
template <class S>
[[nodiscard]] std::optional<std::uint8_t> input_after_shm(const S& src) noexcept(is_nothrow_source<S>::value) {
    static_assert(is_source<S>::value, "S presents every element of CodecZenohValueEntry");
    const auto& _r_body = source::body(src);
    return (_r_body.index() == 2 ? std::optional<std::uint8_t>(::SCE::Generated::CodecZenohValueZbuf::input_after_shm(*std::get_if<2>(&_r_body))) : std::optional<std::uint8_t>{});
}

struct CodecZenohValueEntry {
    CodecZenohValueEntry() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecZenohValueEntry(uint8_t header, CodecZenohValueEntryVariant body)
        : header_(header), body_(std::move(body)) {}
    /// The `header` element.
    [[nodiscard]] constexpr uint8_t header() const noexcept { return header_; }
    /// The arm the value holds, each arm's body the value of its codec.
    [[nodiscard]] const CodecZenohValueEntryVariant& body() const noexcept { return body_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohValueEntry> decode(::SCE::Forge::SceCursor& cursor, std::uint8_t after_shm) {
        // Declared-but-unconsumed flag inputs: defensive (void) suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly under -Wunused.
        (void)after_shm;
        // Decode fixed prefix (RFC §synth-5-B variant: fields before tag suffix).
        const std::uint8_t* raw = cursor.peek_slice(1);
        if (raw == nullptr) return std::nullopt;
        uint8_t header = raw[0];
        if (!cursor.advance(1)) return std::nullopt;
        // Dispatch on tag value into the matching arm body.
        CodecZenohValueEntryVariant body;
        switch (static_cast<uint8_t>((header >> 5) & static_cast<uint8_t>(0x03))) {
            case 0: {
                auto _arm = ::SCE::Generated::CodecZenohExtUnit::CodecZenohExtUnit::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 1: {
                auto _arm = ::SCE::Generated::CodecZenohExtZint::CodecZenohExtZint::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 2: {
                auto _arm = ::SCE::Generated::CodecZenohValueZbuf::CodecZenohValueZbuf::decode(cursor, after_shm);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            default: {
                auto _arm = ::SCE::Generated::CodecZenohExtUnit::CodecZenohExtUnit::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = CodecZenohValueEntryDefault{
                    /*tag=*/static_cast<uint8_t>((header >> 5) & static_cast<uint8_t>(0x03)),
                    /*body=*/*_arm,
                };
                break;
            }
        }
        CodecZenohValueEntry _decoded{
            /*header=*/header,
            /*body=*/std::move(body),
        };
        return _decoded;
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    uint8_t ext_id() const noexcept {
        return static_cast<uint8_t>(
            (this->header_ >> 0) & static_cast<uint8_t>(0x0F)
        );
    }

    void set_ext_id(uint8_t v) noexcept {
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

    bool m() const noexcept {
        return (this->header_ & 0x10) != 0;
    }

    void set_m(bool v) noexcept {
        if (v) {
            this->header_ = static_cast<uint8_t>(this->header_ | 0x10);
        } else {
            this->header_ = static_cast<uint8_t>(this->header_ & static_cast<uint8_t>(~0x10));
        }
    }

    uint8_t enc() const noexcept {
        return static_cast<uint8_t>(
            (this->header_ >> 5) & static_cast<uint8_t>(0x03)
        );
    }

    void set_enc(uint8_t v) noexcept {
        const uint8_t _shifted_mask =
            static_cast<uint8_t>(
                static_cast<uint8_t>(0x03) << 5
            );
        const uint8_t _val =
            static_cast<uint8_t>(
                (static_cast<uint8_t>(v) & static_cast<uint8_t>(0x03)) << 5
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
    /// freshly-encoded byte vector. Callers targeting zero-alloc hot
    /// paths should call `encode` directly against a caller-owned sink.
    [[nodiscard]] std::vector<std::uint8_t> encode_to_vec() const {
        std::vector<std::uint8_t> _sce_v;
        ::SCE::Forge::VectorSink _sce_sink(_sce_v);
        (void)encode(_sce_sink);
        return _sce_v;
    }

private:
    uint8_t header_{};
    CodecZenohValueEntryVariant body_{std::in_place_index_t<0>{}};
};

}  // namespace SCE::Generated::CodecZenohValueEntry

#endif  // SCE_FORGE_CODEC_ZENOH_VALUE_ENTRY_H
