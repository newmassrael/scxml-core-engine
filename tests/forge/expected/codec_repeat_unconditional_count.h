// SCE-MAP: codec_repeat_unconditional_count:34 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_REPEAT_UNCONDITIONAL_COUNT_H
#define SCE_FORGE_CODEC_REPEAT_UNCONDITIONAL_COUNT_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_repeat_elem.h"

namespace SCE::Generated::CodecRepeatUnconditionalCount {

/// The elements a value of `CodecRepeatUnconditionalCount` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_options {
void options() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.options())) -> decltype(s.options()) {
    return s.options();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(options(s))) -> decltype(options(s)) {
    return options(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_options
/// The `options` element of a source.
inline constexpr detail_options::Read options{};
namespace detail_options_h {
void options_h() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.options_h())) -> decltype(s.options_h()) {
    return s.options_h();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(options_h(s))) -> decltype(options_h(s)) {
    return options_h(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_options_h
/// The `options_h` element of a source.
inline constexpr detail_options_h::Read options_h{};
namespace detail_links {
void links() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.links())) -> decltype(s.links()) {
    return s.links();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(links(s))) -> decltype(links(s)) {
    return links(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_links
/// The `links` element of a source.
inline constexpr detail_links::Read links{};
namespace detail_weights {
void weights() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.weights())) -> decltype(s.weights()) {
    return s.weights();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(weights(s))) -> decltype(weights(s)) {
    return weights(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_weights
/// The `weights` element of a source.
inline constexpr detail_weights::Read weights{};
namespace detail_links_weights {
void links_weights() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.links_weights())) -> decltype(s.links_weights()) {
    return s.links_weights();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(links_weights(s))) -> decltype(links_weights(s)) {
    return links_weights(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_links_weights
/// The `links_weights` element of a source.
inline constexpr detail_links_weights::Read links_weights{};
}  // namespace source

/// The fields `options.H` decides (docs/adr/0065 §6 R4b, R4c) present while `options.H` holds.
class CodecRepeatUnconditionalCountOptionsHH {
public:
    CodecRepeatUnconditionalCountOptionsHH() = default;
    /// The value of each field, in declaration order.
    CodecRepeatUnconditionalCountOptionsHH(::SCE::Forge::Aligned<::SCE::Forge::List<::SCE::Generated::CodecRepeatElem::CodecRepeatElem, 64>, ::SCE::Forge::List<::SCE::Generated::CodecRepeatElem::CodecRepeatElem, 64>> links_weights)
        : links_weights_(links_weights) {}
    /// The `links_weights` field.
    [[nodiscard]] ::SCE::Forge::Aligned<::SCE::Forge::List<::SCE::Generated::CodecRepeatElem::CodecRepeatElem, 64>, ::SCE::Forge::List<::SCE::Generated::CodecRepeatElem::CodecRepeatElem, 64>> links_weights() const noexcept { return links_weights_; }

private:
    ::SCE::Forge::Aligned<::SCE::Forge::Bounded<std::vector<::SCE::Generated::CodecRepeatElem::CodecRepeatElem>, 64>, ::SCE::Forge::Bounded<std::vector<::SCE::Generated::CodecRepeatElem::CodecRepeatElem>, 64>> links_weights_{};
};

/// The fields `options.H` decides (docs/adr/0065 §6 R4b, R4c) present while `options.H` does not hold.
class CodecRepeatUnconditionalCountOptionsHNotH {
public:
    CodecRepeatUnconditionalCountOptionsHNotH() = default;
    /// The value of each field, in declaration order.
    CodecRepeatUnconditionalCountOptionsHNotH(::SCE::Forge::List<::SCE::Generated::CodecRepeatElem::CodecRepeatElem, 64> links)
        : links_(links) {}
    /// The `links` field.
    [[nodiscard]] ::SCE::Forge::List<::SCE::Generated::CodecRepeatElem::CodecRepeatElem, 64> links() const noexcept { return links_; }

private:
    ::SCE::Forge::Bounded<std::vector<::SCE::Generated::CodecRepeatElem::CodecRepeatElem>, 64> links_{};
};

/// The fields `options.H` decides (docs/adr/0065 §6 R4b, R4c): a value holds one
/// arm, and its encode derives the gates from which.
using CodecRepeatUnconditionalCountOptionsH = std::variant<CodecRepeatUnconditionalCountOptionsHH, CodecRepeatUnconditionalCountOptionsHNotH>;

/// Whether `R` is a list a source may present for `links`: held to bounds
/// within its own, of any type, each entry a source of the entry codec.
template <class R, class = void>
struct is_links_list : std::false_type {};
template <class R>
struct is_links_list<R, std::enable_if_t<::SCE::Forge::Detail::IsBoundedWithin<R, 64>::value>>
    : ::SCE::Generated::CodecRepeatElem::is_source<::SCE::Forge::Detail::ElementOf<R>> {};

/// Whether `R` is a list a source may present for `weights`: held to bounds
/// within its own, of any type, each entry a source of the entry codec.
template <class R, class = void>
struct is_weights_list : std::false_type {};
template <class R>
struct is_weights_list<R, std::enable_if_t<::SCE::Forge::Detail::IsBoundedWithin<R, 64>::value>>
    : ::SCE::Generated::CodecRepeatElem::is_source<::SCE::Forge::Detail::ElementOf<R>> {};

/// Whether `R` is what a source may present for `links_weights`, two lists of one
/// count (docs/adr/0065 §6 R5): an `Aligned` of a list `is_links_list`
/// accepts and one `is_weights_list` accepts.
template <class R>
struct is_links_weights_aligned : std::false_type {};
template <class A, class B>
struct is_links_weights_aligned<::SCE::Forge::Aligned<A, B>>
    : std::conjunction<is_links_list<A>, is_weights_list<B>> {};

/// Whether `G` presents every field of `CodecRepeatUnconditionalCountOptionsHH`, each convertible to its type.
template <class G, class = void>
struct is_options_h_h_members : std::false_type {};
template <class G>
struct is_options_h_h_members<G, std::void_t<decltype(source::links_weights(std::declval<const G&>()))>>
    : std::conjunction<is_links_weights_aligned<std::decay_t<decltype(source::links_weights(std::declval<const G&>()))>>> {};
/// Whether `G` presents every field of `CodecRepeatUnconditionalCountOptionsHNotH`, each convertible to its type.
template <class G, class = void>
struct is_options_h_not_h_members : std::false_type {};
template <class G>
struct is_options_h_not_h_members<G, std::void_t<decltype(source::links(std::declval<const G&>()))>>
    : std::conjunction<is_links_list<std::decay_t<decltype(source::links(std::declval<const G&>()))>>> {};
/// Whether `A` presents `links` as the type the two lists of one count
/// `B` presents hold it as, which converts to what `A` presents.
template <class A, class B, class = void>
struct is_links_kept_alike_in_links_weights : std::false_type {};
template <class A, class B>
struct is_links_kept_alike_in_links_weights<A, B, std::void_t<decltype(source::links(std::declval<const A&>())), decltype(std::declval<const std::decay_t<decltype(source::links_weights(std::declval<const B&>()))>&>().first())>>
    : std::conjunction<std::is_same<std::decay_t<decltype(source::links(std::declval<const A&>()))>, std::decay_t<decltype(std::declval<const std::decay_t<decltype(source::links_weights(std::declval<const B&>()))>&>().first())>>,
                       std::is_convertible<decltype(std::declval<const std::decay_t<decltype(source::links_weights(std::declval<const B&>()))>&>().first()), decltype(source::links(std::declval<const A&>()))>> {};

/// Whether `R` is what a source may present for `options_h`, the fields `options.H`
/// decides: a `std::variant` of a type for each arm, in order, that presents its fields.
template <class R>
struct is_options_h_group : std::false_type {};
template <class G0, class G1>
struct is_options_h_group<std::variant<G0, G1>>
    : std::conjunction<is_options_h_h_members<G0>, is_options_h_not_h_members<G1>, is_links_kept_alike_in_links_weights<G1, G0>> {};

/// Whether `S` presents every element of `CodecRepeatUnconditionalCount`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::options(std::declval<const S&>())), decltype(source::options_h(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::options(std::declval<const S&>())), uint8_t>, is_options_h_group<std::decay_t<decltype(source::options_h(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::options(std::declval<const S&>()))) && noexcept(source::options_h(std::declval<const S&>())) && noexcept(std::declval<::SCE::Forge::Detail::Kept<decltype(source::links_weights(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(source::options_h(std::declval<const S&>()))>>&>()))>&>().keep(source::links_weights(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(source::options_h(std::declval<const S&>()))>>&>()))) && ::SCE::Generated::CodecRepeatElem::is_nothrow_source<::SCE::Forge::Detail::ElementOf<std::decay_t<decltype(std::declval<const std::decay_t<decltype(source::links_weights(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(source::options_h(std::declval<const S&>()))>>&>()))>&>().first())>>>::value && ::SCE::Generated::CodecRepeatElem::is_nothrow_source<::SCE::Forge::Detail::ElementOf<std::decay_t<decltype(std::declval<const std::decay_t<decltype(source::links_weights(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(source::options_h(std::declval<const S&>()))>>&>()))>&>().second())>>>::value && noexcept(std::declval<::SCE::Forge::Detail::Kept<decltype(source::links(std::declval<const std::variant_alternative_t<1, std::decay_t<decltype(source::options_h(std::declval<const S&>()))>>&>()))>&>().keep(std::declval<const std::decay_t<decltype(source::links_weights(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(source::options_h(std::declval<const S&>()))>>&>()))>&>().first())) && noexcept(std::declval<::SCE::Forge::Detail::Kept<decltype(std::declval<const std::decay_t<decltype(source::links_weights(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(source::options_h(std::declval<const S&>()))>>&>()))>&>().second())>&>().keep(std::declval<const std::decay_t<decltype(source::links_weights(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(source::options_h(std::declval<const S&>()))>>&>()))>&>().second())) && noexcept(std::declval<::SCE::Forge::Detail::Kept<decltype(source::links(std::declval<const std::variant_alternative_t<1, std::decay_t<decltype(source::options_h(std::declval<const S&>()))>>&>()))>&>().keep(source::links(std::declval<const std::variant_alternative_t<1, std::decay_t<decltype(source::options_h(std::declval<const S&>()))>>&>()))) && ::SCE::Generated::CodecRepeatElem::is_nothrow_source<::SCE::Forge::Detail::ElementOf<std::decay_t<decltype(source::links(std::declval<const std::variant_alternative_t<1, std::decay_t<decltype(source::options_h(std::declval<const S&>()))>>&>()))>>>::value> {};

/// The encode of `CodecRepeatUnconditionalCount` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecRepeatUnconditionalCount: options, options_h");
        const auto& _r_options_h = source::options_h(src);
        ::SCE::Forge::Detail::Kept<decltype(source::links(std::declval<const std::variant_alternative_t<1, std::decay_t<decltype(_r_options_h)>>&>()))> _r_links;
        ::SCE::Forge::Detail::Kept<decltype(std::declval<const std::decay_t<decltype(source::links_weights(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(_r_options_h)>>&>()))>&>().second())> _r_weights;
        ::SCE::Forge::Detail::Kept<decltype(source::links_weights(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(_r_options_h)>>&>()))> _r_links_weights;
        if (const auto* _a = std::get_if<0>(&_r_options_h)) {
            _r_links_weights.keep(source::links_weights(*_a));
            _r_links.keep(_r_links_weights->first());
            _r_weights.keep(_r_links_weights->second());
        }
        if (const auto* _a = std::get_if<1>(&_r_options_h)) {
            _r_links.keep(source::links(*_a));
        }
        const uint8_t _d_links_len = _r_links.has_value() ? static_cast<uint8_t>((*_r_links).size()) : uint8_t{0};
        std::uint64_t _f_options = static_cast<std::uint64_t>(source::options(src));
        _f_options = (_f_options & ~0x1ull) | static_cast<std::uint64_t>(_r_weights.has_value());
        const uint8_t _c_options = static_cast<uint8_t>(_f_options);
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_c_options)); _e) return _e;
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_d_links_len)); _e) return _e;
        if (_r_links) {
            for (const auto& _x : (*_r_links)) {
                if (auto _e = ::SCE::Generated::CodecRepeatElem::Wire::encode_source(_x, w); _e) return _e;
            }
        }
        if (_r_weights) {
            for (const auto& _x : (*_r_weights)) {
                if (auto _e = ::SCE::Generated::CodecRepeatElem::Wire::encode_source(_x, w); _e) return _e;
            }
        }
        return std::nullopt;
    }
};

struct CodecRepeatUnconditionalCount {
    CodecRepeatUnconditionalCount() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecRepeatUnconditionalCount(uint8_t options, CodecRepeatUnconditionalCountOptionsH options_h)
        : options_(options), options_h_(std::move(options_h)) {}
    /// The `options` element.
    [[nodiscard]] constexpr uint8_t options() const noexcept { return options_; }
    /// The fields `options.H` decides, as the arm the value holds.
    [[nodiscard]] const CodecRepeatUnconditionalCountOptionsH& options_h() const noexcept { return options_h_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecRepeatUnconditionalCount> decode(::SCE::Forge::SceCursor& cursor) {
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
        uint8_t options;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            options = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        uint8_t links_len;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            links_len = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::vector<::SCE::Generated::CodecRepeatElem::CodecRepeatElem> links;
        if (links_len > 64) {
            cursor.refuse(::SCE::Forge::CodecError::TooManyElements); return std::nullopt;
        }
        links.reserve(cursor.entries_backed(links_len));
        for (auto _i = decltype(links_len){0}; _i < links_len; ++_i) {
            auto _elem = ::SCE::Generated::CodecRepeatElem::CodecRepeatElem::decode(cursor);
            if (!_elem.has_value()) return std::nullopt;
            links.push_back(*_elem);
        }
        std::optional<std::vector<::SCE::Generated::CodecRepeatElem::CodecRepeatElem>> weights;
        if ((options & 0x01) != 0) {
            auto _n = links_len;
            if (_n > 64) {
                cursor.refuse(::SCE::Forge::CodecError::TooManyElements); return std::nullopt;
            }
            std::vector<::SCE::Generated::CodecRepeatElem::CodecRepeatElem> _list;
            _list.reserve(cursor.entries_backed(_n));
            for (auto _i = decltype(_n){0}; _i < _n; ++_i) {
                auto _elem = ::SCE::Generated::CodecRepeatElem::CodecRepeatElem::decode(cursor);
                if (!_elem.has_value()) return std::nullopt;
                _list.push_back(*_elem);
            }
            weights = std::move(_list);
        }
        const auto _b_links = ::SCE::Forge::List<::SCE::Generated::CodecRepeatElem::CodecRepeatElem, 64>::make(links);
        if (!_b_links) {
            cursor.refuse(::SCE::Forge::CodecError::TooManyElements);
            return std::nullopt;
        }
        std::optional<::SCE::Forge::List<::SCE::Generated::CodecRepeatElem::CodecRepeatElem, 64>> _b_weights;
        if (weights) {
            _b_weights = ::SCE::Forge::List<::SCE::Generated::CodecRepeatElem::CodecRepeatElem, 64>::make(*weights);
            if (!_b_weights) {
                cursor.refuse(::SCE::Forge::CodecError::TooManyElements);
                return std::nullopt;
            }
        }
        (void)links_len;
        CodecRepeatUnconditionalCountOptionsH _g_options_h;
        if ((((options >> 0) & 0x1) != 0)) {
            const auto _al_links_weights = ::SCE::Forge::Aligned<::SCE::Forge::List<::SCE::Generated::CodecRepeatElem::CodecRepeatElem, 64>, ::SCE::Forge::List<::SCE::Generated::CodecRepeatElem::CodecRepeatElem, 64>>::make(*_b_links, *_b_weights);
            if (!_al_links_weights) {
                cursor.refuse(::SCE::Forge::CodecError::TooManyElements);
                return std::nullopt;
            }
            _g_options_h.emplace<0>(*_al_links_weights);
        } else if ((((options >> 0) & 0x1) == 0)) {
            _g_options_h.emplace<1>(*_b_links);
        }
        CodecRepeatUnconditionalCount _decoded{
            /*options=*/options,
            /*options_h=*/std::move(_g_options_h),
        };
        return _decoded;
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool h() const noexcept {
        return (this->options_ & 0x01) != 0;
    }

    void set_h(bool v) noexcept {
        if (v) {
            this->options_ = static_cast<uint8_t>(this->options_ | 0x01);
        } else {
            this->options_ = static_cast<uint8_t>(this->options_ & static_cast<uint8_t>(~0x01));
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 258;

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
    uint8_t options_{};
    CodecRepeatUnconditionalCountOptionsH options_h_{std::in_place_index_t<1>{}};
};

}  // namespace SCE::Generated::CodecRepeatUnconditionalCount

#endif  // SCE_FORGE_CODEC_REPEAT_UNCONDITIONAL_COUNT_H
