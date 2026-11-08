// SCE-MAP: codec_chain_has_marker:28 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_CHAIN_HAS_MARKER_H
#define SCE_FORGE_CODEC_CHAIN_HAS_MARKER_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_zenoh_ext_entry.h"
#include "codec_chain_has_marker_slice.h"

namespace SCE::Generated::CodecChainHasMarker {

/// The elements a value of `CodecChainHasMarker` presents, each read by name: the
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
namespace detail_extensions_has_0x12 {
void extensions_has_0x12() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.extensions_has_0x12())) -> decltype(s.extensions_has_0x12()) {
    return s.extensions_has_0x12();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(extensions_has_0x12(s))) -> decltype(extensions_has_0x12(s)) {
    return extensions_has_0x12(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_extensions_has_0x12
/// The `extensions_has_0x12` element of a source.
inline constexpr detail_extensions_has_0x12::Read extensions_has_0x12{};
namespace detail_extensions {
void extensions() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.extensions())) -> decltype(s.extensions()) {
    return s.extensions();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(extensions(s))) -> decltype(extensions(s)) {
    return extensions(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_extensions
/// The `extensions` element of a source.
inline constexpr detail_extensions::Read extensions{};
namespace detail_payload {
void payload() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.payload())) -> decltype(s.payload()) {
    return s.payload();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(payload(s))) -> decltype(payload(s)) {
    return payload(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_payload
/// The `payload` element of a source.
inline constexpr detail_payload::Read payload{};
namespace detail_slices {
void slices() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.slices())) -> decltype(s.slices()) {
    return s.slices();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(slices(s))) -> decltype(slices(s)) {
    return slices(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_slices
/// The `slices` element of a source.
inline constexpr detail_slices::Read slices{};
}  // namespace source

/// The kind of entry `extensions.has(0x12)` names: an entry whose identifier
/// is 0x12, which is all of its value (docs/adr/0065 §6 R6a).
struct CodecChainHasMarkerExtensionsHas0x12Key {
    template <class E>
    [[nodiscard]] static bool matches(const E& entry) {
        return (static_cast<std::uint64_t>(::SCE::Generated::CodecZenohExtEntry::source::header(entry)) & 0x7Fu) == 0x12u;
    }
    /// The entry of the kind: its identifier, and nothing else to say.
    [[nodiscard]] static ::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry named() {
        return ::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry{0x12, ::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntryVariant{std::in_place_index<0>}};
    }
};

/// The fields `extensions.has(0x12)` decides (docs/adr/0065 §6 R4b, R4c) present while `extensions.has(0x12)` holds.
class CodecChainHasMarkerExtensionsHas0x12Has0x12 {
public:
    CodecChainHasMarkerExtensionsHas0x12Has0x12() = default;
    /// The value of each field, in declaration order.
    CodecChainHasMarkerExtensionsHas0x12Has0x12(::SCE::Forge::Placed<::SCE::Forge::Span<const ::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>, CodecChainHasMarkerExtensionsHas0x12Key, 4> extensions, ::SCE::Forge::List<::SCE::Generated::CodecChainHasMarkerSlice::CodecChainHasMarkerSlice, 4> slices)
        : extensions_(extensions), slices_(slices) {}
    /// The `extensions` field.
    [[nodiscard]] ::SCE::Forge::Placed<::SCE::Forge::Span<const ::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>, CodecChainHasMarkerExtensionsHas0x12Key, 4> extensions() const noexcept { return extensions_; }
    /// The `slices` field.
    [[nodiscard]] ::SCE::Forge::List<::SCE::Generated::CodecChainHasMarkerSlice::CodecChainHasMarkerSlice, 4> slices() const noexcept { return slices_; }

private:
    ::SCE::Forge::Placed<std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>, CodecChainHasMarkerExtensionsHas0x12Key, 4> extensions_{};
    ::SCE::Forge::Bounded<std::vector<::SCE::Generated::CodecChainHasMarkerSlice::CodecChainHasMarkerSlice>, 4> slices_{};
};

/// The fields `extensions.has(0x12)` decides (docs/adr/0065 §6 R4b, R4c) present while `extensions.has(0x12)` does not hold.
class CodecChainHasMarkerExtensionsHas0x12NotHas0x12 {
public:
    CodecChainHasMarkerExtensionsHas0x12NotHas0x12() = default;
    /// The value of each field, in declaration order.
    CodecChainHasMarkerExtensionsHas0x12NotHas0x12(std::optional<::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>, ::SCE::Forge::Kinds<>, ::SCE::Forge::Kinds<CodecChainHasMarkerExtensionsHas0x12Key>, 4, 1>> extensions, ::SCE::Forge::Bytes<256> payload)
        : extensions_(extensions), payload_(payload) {}
    /// The `extensions` field.
    [[nodiscard]] std::optional<::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>, ::SCE::Forge::Kinds<>, ::SCE::Forge::Kinds<CodecChainHasMarkerExtensionsHas0x12Key>, 4, 1>> extensions() const noexcept { return extensions_; }
    /// The `payload` field.
    [[nodiscard]] ::SCE::Forge::Bytes<256> payload() const noexcept { return payload_; }

private:
    std::optional<::SCE::Forge::Chain<std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>, ::SCE::Forge::Kinds<>, ::SCE::Forge::Kinds<CodecChainHasMarkerExtensionsHas0x12Key>, 4, 1>> extensions_{};
    ::SCE::Forge::Bounded<std::vector<std::uint8_t>, 256> payload_{};
};

/// The fields `extensions.has(0x12)` decides (docs/adr/0065 §6 R4b, R4c): a value holds one
/// arm, and its encode derives the gates from which.
using CodecChainHasMarkerExtensionsHas0x12 = std::variant<CodecChainHasMarkerExtensionsHas0x12Has0x12, CodecChainHasMarkerExtensionsHas0x12NotHas0x12>;

/// Whether `R` is a list a source may present for `extensions`: held to bounds
/// within its own, of any type, each entry a source of the entry codec.
template <class R, class = void>
struct is_extensions_list : std::false_type {};
template <class R>
struct is_extensions_list<R, std::enable_if_t<::SCE::Forge::Detail::IsBoundedWithin<R, 4, 1>::value>>
    : ::SCE::Generated::CodecZenohExtEntry::is_source<::SCE::Forge::Detail::ElementOf<R>> {};

/// Whether `R` is a list a source may present for `slices`: held to bounds
/// within its own, of any type, each entry a source of the entry codec.
template <class R, class = void>
struct is_slices_list : std::false_type {};
template <class R>
struct is_slices_list<R, std::enable_if_t<::SCE::Forge::Detail::IsBoundedWithin<R, 4>::value>>
    : ::SCE::Generated::CodecChainHasMarkerSlice::is_source<::SCE::Forge::Detail::ElementOf<R>> {};

/// Whether `R` is what a source may present for `extensions` where the one gate
/// on a bare entry holds (docs/adr/0065 §6 R6a): a `Placed` of the entry's
/// kind, held to a bound within the chain's, of any list of the others,
/// each a source of the entry codec.
template <class R>
struct is_extensions_placed_has_0x12 : std::false_type {};
template <class L, std::size_t Max>
struct is_extensions_placed_has_0x12<::SCE::Forge::Placed<L, CodecChainHasMarkerExtensionsHas0x12Key, Max>>
    : std::conjunction<std::bool_constant<Max <= 4>,
                       ::SCE::Generated::CodecZenohExtEntry::is_source<::SCE::Forge::Detail::ElementOf<L>>> {};

/// Whether `R` is what a source may present for `extensions` where the chain holds
/// an entry of each kind of `::SCE::Forge::Kinds<>` and of none of `::SCE::Forge::Kinds<CodecChainHasMarkerExtensionsHas0x12Key>`
/// (docs/adr/0065 §6 R6b): a `Chain` of those kinds, held to bounds within its
/// own, of any list, each entry a source of the entry codec.
template <class R>
struct is_extensions_chain_not_0x12 : std::false_type {};
template <class L, std::size_t Max, std::size_t Min>
struct is_extensions_chain_not_0x12<::SCE::Forge::Chain<L, ::SCE::Forge::Kinds<>, ::SCE::Forge::Kinds<CodecChainHasMarkerExtensionsHas0x12Key>, Max, Min>>
    : std::conjunction<::SCE::Forge::Detail::WithinBounds<4, 1, Max, Min>,
                       ::SCE::Generated::CodecZenohExtEntry::is_source<::SCE::Forge::Detail::ElementOf<L>>> {};

/// Whether `R` is what a source may present for `extensions` where the arm may
/// hold no chain: a `std::optional` of a type `is_extensions_chain_not_0x12` accepts.
template <class R>
struct is_extensions_chain_not_0x12_present : std::false_type {};
template <class C>
struct is_extensions_chain_not_0x12_present<std::optional<C>> : is_extensions_chain_not_0x12<C> {};

/// Whether `G` presents every field of `CodecChainHasMarkerExtensionsHas0x12Has0x12`, each convertible to its type.
template <class G, class = void>
struct is_extensions_has_0x12_has0x12_members : std::false_type {};
template <class G>
struct is_extensions_has_0x12_has0x12_members<G, std::void_t<decltype(source::extensions(std::declval<const G&>())), decltype(source::slices(std::declval<const G&>()))>>
    : std::conjunction<is_extensions_placed_has_0x12<std::decay_t<decltype(source::extensions(std::declval<const G&>()))>>, is_slices_list<std::decay_t<decltype(source::slices(std::declval<const G&>()))>>> {};
/// Whether `G` presents every field of `CodecChainHasMarkerExtensionsHas0x12NotHas0x12`, each convertible to its type.
template <class G, class = void>
struct is_extensions_has_0x12_not_has0x12_members : std::false_type {};
template <class G>
struct is_extensions_has_0x12_not_has0x12_members<G, std::void_t<decltype(source::extensions(std::declval<const G&>())), decltype(source::payload(std::declval<const G&>()))>>
    : std::conjunction<is_extensions_chain_not_0x12_present<std::decay_t<decltype(source::extensions(std::declval<const G&>()))>>, std::is_convertible<decltype(source::payload(std::declval<const G&>())), ::SCE::Forge::Bytes<256>>, ::SCE::Forge::Detail::Lasting<decltype(source::payload(std::declval<const G&>()))>> {};
/// Whether `A` and `B` both present `extensions` as a chain of one list, kept
/// alike.
template <class A, class B, class = void>
struct is_extensions_read_alike : std::false_type {};
template <class A, class B>
struct is_extensions_read_alike<A, B, std::void_t<::SCE::Forge::Detail::InnerRead<decltype(source::extensions(std::declval<const A&>()))>, ::SCE::Forge::Detail::InnerRead<decltype(source::extensions(std::declval<const B&>()))>>>
    : std::is_same<::SCE::Forge::Detail::InnerRead<decltype(source::extensions(std::declval<const A&>()))>, ::SCE::Forge::Detail::InnerRead<decltype(source::extensions(std::declval<const B&>()))>> {};

/// Whether `R` is what a source may present for `extensions_has_0x12`, the fields `extensions.has(0x12)`
/// decides: a `std::variant` of a type for each arm, in order, that presents its fields.
template <class R>
struct is_extensions_has_0x12_group : std::false_type {};
template <class G0, class G1>
struct is_extensions_has_0x12_group<std::variant<G0, G1>>
    : std::conjunction<is_extensions_has_0x12_has0x12_members<G0>, is_extensions_has_0x12_not_has0x12_members<G1>, is_extensions_read_alike<G0, G1>> {};

/// Whether `S` presents every element of `CodecChainHasMarker`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::header(std::declval<const S&>())), decltype(source::extensions_has_0x12(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::header(std::declval<const S&>())), uint8_t>, is_extensions_has_0x12_group<std::decay_t<decltype(source::extensions_has_0x12(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::header(std::declval<const S&>()))) && noexcept(source::extensions_has_0x12(std::declval<const S&>())) && noexcept(std::declval<::SCE::Forge::Detail::Kept<::SCE::Forge::Detail::InnerRead<decltype(source::extensions(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(source::extensions_has_0x12(std::declval<const S&>()))>>&>()))>>&>().keep(source::extensions(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(source::extensions_has_0x12(std::declval<const S&>()))>>&>()).rest())) && ::SCE::Generated::CodecZenohExtEntry::is_nothrow_source<::SCE::Forge::Detail::ElementOf<typename ::SCE::Forge::Detail::ChainOf<std::decay_t<decltype(source::extensions(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(source::extensions_has_0x12(std::declval<const S&>()))>>&>()))>>::type::list_type>>::value && noexcept(std::declval<::SCE::Forge::Detail::Kept<decltype(source::slices(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(source::extensions_has_0x12(std::declval<const S&>()))>>&>()))>&>().keep(source::slices(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(source::extensions_has_0x12(std::declval<const S&>()))>>&>()))) && ::SCE::Generated::CodecChainHasMarkerSlice::is_nothrow_source<::SCE::Forge::Detail::ElementOf<std::decay_t<decltype(source::slices(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(source::extensions_has_0x12(std::declval<const S&>()))>>&>()))>>>::value && noexcept(std::declval<::SCE::Forge::Detail::Kept<::SCE::Forge::Detail::InnerRead<decltype(source::extensions(std::declval<const std::variant_alternative_t<1, std::decay_t<decltype(source::extensions_has_0x12(std::declval<const S&>()))>>&>()))>>&>().keep((source::extensions(std::declval<const std::variant_alternative_t<1, std::decay_t<decltype(source::extensions_has_0x12(std::declval<const S&>()))>>&>()))->inner())) && ::SCE::Generated::CodecZenohExtEntry::is_nothrow_source<::SCE::Forge::Detail::ElementOf<typename ::SCE::Forge::Detail::ChainOf<std::decay_t<decltype(source::extensions(std::declval<const std::variant_alternative_t<1, std::decay_t<decltype(source::extensions_has_0x12(std::declval<const S&>()))>>&>()))>>::type::list_type>>::value && noexcept(::SCE::Forge::Detail::converts_to<::SCE::Forge::Bytes<256>>(source::payload(std::declval<const std::variant_alternative_t<1, std::decay_t<decltype(source::extensions_has_0x12(std::declval<const S&>()))>>&>())))> {};

/// The encode of `CodecChainHasMarker` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecChainHasMarker: header, extensions_has_0x12");
        const auto& _r_extensions_has_0x12 = source::extensions_has_0x12(src);
        ::SCE::Forge::Detail::Kept<::SCE::Forge::Detail::InnerRead<decltype(source::extensions(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(_r_extensions_has_0x12)>>&>()))>> _r_extensions;
        std::optional<std::size_t> _p_extensions;
        std::optional<::SCE::Forge::Bytes<256>> _e_payload;
        ::SCE::Forge::Detail::Kept<decltype(source::slices(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(_r_extensions_has_0x12)>>&>()))> _r_slices;
        if (const auto* _a = std::get_if<0>(&_r_extensions_has_0x12)) {
            {
                const auto& _c_extensions = source::extensions(*_a);
                _r_extensions.keep(_c_extensions.rest());
                _p_extensions = _c_extensions.place();
            }
            _r_slices.keep(source::slices(*_a));
        }
        if (const auto* _a = std::get_if<1>(&_r_extensions_has_0x12)) {
            if (const auto& _c_extensions = source::extensions(*_a)) {
                _r_extensions.keep(_c_extensions->inner());
            }
            _e_payload = static_cast<::SCE::Forge::Bytes<256>>(source::payload(*_a));
        }
        const uint64_t _d_payload_len = _e_payload.has_value() ? static_cast<uint64_t>(_e_payload->size()) : uint64_t{0};
        const uint32_t _d_slice_count = _r_slices.has_value() ? static_cast<uint32_t>((*_r_slices).size()) : uint32_t{0};
        std::uint64_t _f_header = static_cast<std::uint64_t>(source::header(src));
        _f_header = (_f_header & ~0x80ull) | (static_cast<std::uint64_t>(_r_extensions.has_value()) << 7);
        const uint8_t _c_header = static_cast<uint8_t>(_f_header);
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_c_header)); _e) return _e;
        if (_r_extensions) {
            {
                const std::size_t _k_extensions = (*_r_extensions).size() + (_p_extensions ? 1u : 0u);
                auto _n_extensions = (*_r_extensions).begin();
                for (std::size_t _i = 0; _i < _k_extensions; ++_i) {
                    ::SCE::Generated::CodecZenohExtEntry::Overrides _o_extensions{};
                    _o_extensions.header_z = _i + 1 < _k_extensions;
                    if (_p_extensions && *_p_extensions == _i) {
                        if (auto _e = ::SCE::Generated::CodecZenohExtEntry::Wire::encode_source(CodecChainHasMarkerExtensionsHas0x12Key::named(), w, _o_extensions); _e) return _e;
                    } else {
                        if (auto _e = ::SCE::Generated::CodecZenohExtEntry::Wire::encode_source(*_n_extensions, w, _o_extensions); _e) return _e;
                        ++_n_extensions;
                    }
                }
            }
        }
        if ((static_cast<std::uint8_t>((_r_extensions_has_0x12.index() == 0u) ? 1 : 0) == 0)) {
            if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(_d_payload_len)); _e) return _e;
        }
        if (_e_payload) {
            if (auto _e = w.write_bytes(_e_payload->data(), _e_payload->size()); _e) return _e;
        }
        if ((static_cast<std::uint8_t>((_r_extensions_has_0x12.index() == 0u) ? 1 : 0) != 0)) {
            if (auto _e = w.write_vle_u32(static_cast<std::uint32_t>(_d_slice_count)); _e) return _e;
        }
        if (_r_slices) {
            for (const auto& _x : (*_r_slices)) {
                if (auto _e = ::SCE::Generated::CodecChainHasMarkerSlice::Wire::encode_source(_x, w); _e) return _e;
            }
        }
        return std::nullopt;
    }
};

struct CodecChainHasMarker {
    CodecChainHasMarker() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecChainHasMarker(uint8_t header, CodecChainHasMarkerExtensionsHas0x12 extensions_has_0x12)
        : header_(header), extensions_has_0x12_(std::move(extensions_has_0x12)) {}
    /// The `header` element.
    [[nodiscard]] constexpr uint8_t header() const noexcept { return header_; }
    /// The fields `extensions.has(0x12)` decides, as the arm the value holds.
    [[nodiscard]] const CodecChainHasMarkerExtensionsHas0x12& extensions_has_0x12() const noexcept { return extensions_has_0x12_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecChainHasMarker> decode(::SCE::Forge::SceCursor& cursor) {
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
        std::optional<std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>> extensions;
        if ((header & 0x80) != 0) {
            std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry> _list;
            _list.reserve(4);
            bool _more = false;
            for (std::size_t _i = 0; _i < 4; ++_i) {
                auto _elem = ::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry::decode(cursor);
                if (!_elem.has_value()) return std::nullopt;
                _more = _elem->z();
                _list.push_back(*_elem);
                if (!_more) break;
            }
            if (_more && cursor.remaining() == 0) { return std::nullopt; }
            if (_more) { cursor.refuse(::SCE::Forge::CodecError::TooManyElements); return std::nullopt; }
            extensions = std::move(_list);
        }
        bool _has_extensions_18 = false;
        if (extensions.has_value()) {
            for (const auto& _e : *extensions) {
                if ((static_cast<std::uint64_t>(_e.header()) & 127ULL) == 18ULL) {
                    _has_extensions_18 = true;
                    break;
                }
            }
        }
        std::optional<uint64_t> payload_len;
        if (!_has_extensions_18) {
            auto _v_opt = cursor.read_vle_u64();
        if (!_v_opt.has_value()) return std::nullopt;
        auto _v = static_cast<std::uint64_t>(*_v_opt);
            payload_len = _v;
        }
        std::optional<std::vector<uint8_t>> payload;
        if (!_has_extensions_18) {
            const auto _len = payload_len.value();
            if (_len > 256) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange); return std::nullopt;
            }
            std::size_t _n = static_cast<std::size_t>(_len);
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            payload.emplace(raw, raw + _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        std::optional<uint32_t> slice_count;
        if (_has_extensions_18) {
            auto _v_opt = cursor.read_vle_u32();
        if (!_v_opt.has_value()) return std::nullopt;
        auto _v = static_cast<std::uint32_t>(*_v_opt);
            slice_count = _v;
        }
        std::optional<std::vector<::SCE::Generated::CodecChainHasMarkerSlice::CodecChainHasMarkerSlice>> slices;
        if (_has_extensions_18) {
            auto _n = slice_count.value();
            if (_n > 4) {
                cursor.refuse(::SCE::Forge::CodecError::TooManyElements); return std::nullopt;
            }
            std::vector<::SCE::Generated::CodecChainHasMarkerSlice::CodecChainHasMarkerSlice> _list;
            _list.reserve(cursor.entries_backed(_n));
            for (auto _i = decltype(_n){0}; _i < _n; ++_i) {
                auto _elem = ::SCE::Generated::CodecChainHasMarkerSlice::CodecChainHasMarkerSlice::decode(cursor);
                if (!_elem.has_value()) return std::nullopt;
                _list.push_back(*_elem);
            }
            slices = std::move(_list);
        }
        std::optional<::SCE::Forge::List<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry, 4, 1>> _b_extensions;
        if (extensions) {
            _b_extensions = ::SCE::Forge::List<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry, 4, 1>::make(*extensions);
            if (!_b_extensions) {
                cursor.refuse(::SCE::Forge::CodecError::TooManyElements);
                return std::nullopt;
            }
        }
        std::optional<::SCE::Forge::List<::SCE::Generated::CodecChainHasMarkerSlice::CodecChainHasMarkerSlice, 4>> _b_slices;
        if (slices) {
            _b_slices = ::SCE::Forge::List<::SCE::Generated::CodecChainHasMarkerSlice::CodecChainHasMarkerSlice, 4>::make(*slices);
            if (!_b_slices) {
                cursor.refuse(::SCE::Forge::CodecError::TooManyElements);
                return std::nullopt;
            }
        }
        std::optional<::SCE::Forge::Bytes<256>> _b_payload;
        if (payload) {
            _b_payload = ::SCE::Forge::Bytes<256>::make(*payload);
            if (!_b_payload) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);
                return std::nullopt;
            }
        }
        (void)payload_len;
        (void)slice_count;
        CodecChainHasMarkerExtensionsHas0x12 _g_extensions_has_0x12;
        if (::SCE::Forge::Detail::first_of_kind<CodecChainHasMarkerExtensionsHas0x12Key>(extensions).has_value()) {
            std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry> _rest_extensions(_b_extensions->inner().begin(), _b_extensions->inner().end());
            const auto _at_extensions = ::SCE::Forge::Detail::first_of_kind<CodecChainHasMarkerExtensionsHas0x12Key>(_rest_extensions);
            if (!_at_extensions) {
                cursor.refuse(::SCE::Forge::CodecError::PresentIfMismatch);
                return std::nullopt;
            }
            _rest_extensions.erase(_rest_extensions.begin() + static_cast<std::ptrdiff_t>(*_at_extensions));
            const auto _ch_extensions = ::SCE::Forge::Placed<::SCE::Forge::Span<const ::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>, CodecChainHasMarkerExtensionsHas0x12Key, 4>::make(::SCE::Forge::Span<const ::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>(_rest_extensions.data(), _rest_extensions.size()), *_at_extensions);
            if (!_ch_extensions) {
                cursor.refuse(::SCE::Forge::CodecError::TooManyElements);
                return std::nullopt;
            }
            _g_extensions_has_0x12.emplace<0>(*_ch_extensions, *_b_slices);
        } else if (!::SCE::Forge::Detail::first_of_kind<CodecChainHasMarkerExtensionsHas0x12Key>(extensions).has_value()) {
            std::optional<::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>, ::SCE::Forge::Kinds<>, ::SCE::Forge::Kinds<CodecChainHasMarkerExtensionsHas0x12Key>, 4, 1>> _ch_extensions;
            if (_b_extensions) {
                _ch_extensions = ::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>, ::SCE::Forge::Kinds<>, ::SCE::Forge::Kinds<CodecChainHasMarkerExtensionsHas0x12Key>, 4, 1>::make(_b_extensions->inner());
                if (!_ch_extensions) {
                    cursor.refuse(::SCE::Forge::CodecError::PresentIfMismatch);
                    return std::nullopt;
                }
            }
            _g_extensions_has_0x12.emplace<1>(_ch_extensions, *_b_payload);
        }
        CodecChainHasMarker _decoded{
            /*header=*/header,
            /*extensions_has_0x12=*/std::move(_g_extensions_has_0x12),
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
            (this->header_ >> 0) & static_cast<uint8_t>(0x1F)
        );
    }

    void set_kind(uint8_t v) noexcept {
        const uint8_t _shifted_mask =
            static_cast<uint8_t>(
                static_cast<uint8_t>(0x1F) << 0
            );
        const uint8_t _val =
            static_cast<uint8_t>(
                (static_cast<uint8_t>(v) & static_cast<uint8_t>(0x1F)) << 0
            );
        this->header_ = static_cast<uint8_t>(
            (this->header_ & static_cast<uint8_t>(~_shifted_mask)) | _val
        );
    }

    bool e() const noexcept {
        return (this->header_ & 0x80) != 0;
    }

    void set_e(bool v) noexcept {
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
        ::SCE::Forge::VectorSink _sce_sink(_sce_v);
        if (encode(_sce_sink)) {
            return std::nullopt;
        }
        return _sce_v;
    }

private:
    uint8_t header_{};
    CodecChainHasMarkerExtensionsHas0x12 extensions_has_0x12_{std::in_place_index_t<1>{}};
};

}  // namespace SCE::Generated::CodecChainHasMarker

#endif  // SCE_FORGE_CODEC_CHAIN_HAS_MARKER_H
