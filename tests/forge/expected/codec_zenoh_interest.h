// SCE-MAP: codec_zenoh_interest:73 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_INTEREST_H
#define SCE_FORGE_CODEC_ZENOH_INTEREST_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_zenoh_ext_entry.h"
#include "codec_zenoh_interest_body.h"

namespace SCE::Generated::CodecZenohInterest {

/// The elements a value of `CodecZenohInterest` presents, each read by name: the
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
namespace detail_id {
void id() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.id())) -> decltype(s.id()) {
    return s.id();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(id(s))) -> decltype(id(s)) {
    return id(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_id
/// The `id` element of a source.
inline constexpr detail_id::Read id{};
namespace detail_header_current_or_future {
void header_current_or_future() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.header_current_or_future())) -> decltype(s.header_current_or_future()) {
    return s.header_current_or_future();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(header_current_or_future(s))) -> decltype(header_current_or_future(s)) {
    return header_current_or_future(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_header_current_or_future
/// The `header_current_or_future` element of a source.
inline constexpr detail_header_current_or_future::Read header_current_or_future{};
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

/// The fields `header.CURRENT || header.FUTURE` decides (docs/adr/0065 §6 R4b, R4c) present while `header.CURRENT` holds and `header.FUTURE` does not hold.
class CodecZenohInterestHeaderCurrentOrFutureCurrent {
public:
    CodecZenohInterestHeaderCurrentOrFutureCurrent() = default;
    /// The value of each field, in declaration order.
    CodecZenohInterestHeaderCurrentOrFutureCurrent(::SCE::Generated::CodecZenohInterestBody::CodecZenohInterestBody body)
        : body_(std::move(body)) {}
    /// The `body` field.
    [[nodiscard]] const ::SCE::Generated::CodecZenohInterestBody::CodecZenohInterestBody& body() const noexcept { return body_; }

private:
    ::SCE::Generated::CodecZenohInterestBody::CodecZenohInterestBody body_{};
};

/// The fields `header.CURRENT || header.FUTURE` decides (docs/adr/0065 §6 R4b, R4c) present while `header.CURRENT` does not hold and `header.FUTURE` holds.
class CodecZenohInterestHeaderCurrentOrFutureFuture {
public:
    CodecZenohInterestHeaderCurrentOrFutureFuture() = default;
    /// The value of each field, in declaration order.
    CodecZenohInterestHeaderCurrentOrFutureFuture(::SCE::Generated::CodecZenohInterestBody::CodecZenohInterestBody body)
        : body_(std::move(body)) {}
    /// The `body` field.
    [[nodiscard]] const ::SCE::Generated::CodecZenohInterestBody::CodecZenohInterestBody& body() const noexcept { return body_; }

private:
    ::SCE::Generated::CodecZenohInterestBody::CodecZenohInterestBody body_{};
};

/// The fields `header.CURRENT || header.FUTURE` decides (docs/adr/0065 §6 R4b, R4c) present while `header.CURRENT` holds and `header.FUTURE` holds.
class CodecZenohInterestHeaderCurrentOrFutureCurrentFuture {
public:
    CodecZenohInterestHeaderCurrentOrFutureCurrentFuture() = default;
    /// The value of each field, in declaration order.
    CodecZenohInterestHeaderCurrentOrFutureCurrentFuture(::SCE::Generated::CodecZenohInterestBody::CodecZenohInterestBody body)
        : body_(std::move(body)) {}
    /// The `body` field.
    [[nodiscard]] const ::SCE::Generated::CodecZenohInterestBody::CodecZenohInterestBody& body() const noexcept { return body_; }

private:
    ::SCE::Generated::CodecZenohInterestBody::CodecZenohInterestBody body_{};
};

/// The fields `header.CURRENT || header.FUTURE` decides (docs/adr/0065 §6 R4b, R4c): a value holds one
/// arm or none of them, and its encode derives the gates from which.
using CodecZenohInterestHeaderCurrentOrFuture = std::variant<CodecZenohInterestHeaderCurrentOrFutureCurrent, CodecZenohInterestHeaderCurrentOrFutureFuture, CodecZenohInterestHeaderCurrentOrFutureCurrentFuture>;

/// Whether `R` is a list a source may present for `extensions`: held to bounds
/// within its own, of any type, each entry a source of the entry codec.
template <class R, class = void>
struct is_extensions_list : std::false_type {};
template <class R>
struct is_extensions_list<R, std::enable_if_t<::SCE::Forge::Detail::IsBoundedWithin<R, ::SCE::Forge::kUnbounded, 1>::value>>
    : ::SCE::Generated::CodecZenohExtEntry::is_source<::SCE::Forge::Detail::ElementOf<R>> {};

/// Whether `R` is what a source may present for `extensions`, a list a gate
/// decides: a `std::optional` of a list `is_extensions_list` accepts.
template <class R>
struct is_extensions_present_list : std::false_type {};
template <class L>
struct is_extensions_present_list<std::optional<L>> : is_extensions_list<L> {};

/// Whether `G` presents every field of `CodecZenohInterestHeaderCurrentOrFutureCurrent`, each convertible to its type.
template <class G, class = void>
struct is_header_current_or_future_current_members : std::false_type {};
template <class G>
struct is_header_current_or_future_current_members<G, std::void_t<decltype(source::body(std::declval<const G&>()))>>
    : std::conjunction<::SCE::Generated::CodecZenohInterestBody::is_source<std::decay_t<decltype(source::body(std::declval<const G&>()))>>> {};
/// Whether `G` presents every field of `CodecZenohInterestHeaderCurrentOrFutureFuture`, each convertible to its type.
template <class G, class = void>
struct is_header_current_or_future_future_members : std::false_type {};
template <class G>
struct is_header_current_or_future_future_members<G, std::void_t<decltype(source::body(std::declval<const G&>()))>>
    : std::conjunction<::SCE::Generated::CodecZenohInterestBody::is_source<std::decay_t<decltype(source::body(std::declval<const G&>()))>>> {};
/// Whether `G` presents every field of `CodecZenohInterestHeaderCurrentOrFutureCurrentFuture`, each convertible to its type.
template <class G, class = void>
struct is_header_current_or_future_current_future_members : std::false_type {};
template <class G>
struct is_header_current_or_future_current_future_members<G, std::void_t<decltype(source::body(std::declval<const G&>()))>>
    : std::conjunction<::SCE::Generated::CodecZenohInterestBody::is_source<std::decay_t<decltype(source::body(std::declval<const G&>()))>>> {};
/// Whether `A` and `B` both present `body`, as one type.
template <class A, class B, class = void>
struct is_body_read_alike : std::false_type {};
template <class A, class B>
struct is_body_read_alike<A, B, std::void_t<decltype(source::body(std::declval<const A&>())), decltype(source::body(std::declval<const B&>()))>>
    : std::is_same<decltype(source::body(std::declval<const A&>())), decltype(source::body(std::declval<const B&>()))> {};

/// Whether `R` is what a source may present for `header_current_or_future`, the fields `header.CURRENT || header.FUTURE`
/// decides: a `std::optional` of a `std::variant` of a type for each arm, in order, that presents its fields.
template <class R>
struct is_header_current_or_future_group : std::false_type {};
template <class G0, class G1, class G2>
struct is_header_current_or_future_group<std::optional<std::variant<G0, G1, G2>>>
    : std::conjunction<is_header_current_or_future_current_members<G0>, is_header_current_or_future_future_members<G1>, is_header_current_or_future_current_future_members<G2>, is_body_read_alike<G0, G1>, is_body_read_alike<G0, G2>> {};

/// Whether `S` presents every element of `CodecZenohInterest`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::header(std::declval<const S&>())), decltype(source::id(std::declval<const S&>())), decltype(source::header_current_or_future(std::declval<const S&>())), decltype(source::extensions(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::header(std::declval<const S&>())), uint8_t>, std::is_convertible<decltype(source::id(std::declval<const S&>())), uint64_t>, is_header_current_or_future_group<std::decay_t<decltype(source::header_current_or_future(std::declval<const S&>()))>>, is_extensions_present_list<std::decay_t<decltype(source::extensions(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::header(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint64_t>(source::id(std::declval<const S&>()))) && noexcept(source::header_current_or_future(std::declval<const S&>())) && noexcept(std::declval<::SCE::Forge::Detail::Kept<decltype(source::body(std::declval<const std::variant_alternative_t<0, typename std::decay_t<decltype(source::header_current_or_future(std::declval<const S&>()))>::value_type>&>()))>&>().keep(source::body(std::declval<const std::variant_alternative_t<0, typename std::decay_t<decltype(source::header_current_or_future(std::declval<const S&>()))>::value_type>&>()))) && ::SCE::Generated::CodecZenohInterestBody::is_nothrow_source<std::decay_t<decltype(source::body(std::declval<const std::variant_alternative_t<0, typename std::decay_t<decltype(source::header_current_or_future(std::declval<const S&>()))>::value_type>&>()))>>::value && noexcept(std::declval<::SCE::Forge::Detail::Kept<decltype(source::body(std::declval<const std::variant_alternative_t<1, typename std::decay_t<decltype(source::header_current_or_future(std::declval<const S&>()))>::value_type>&>()))>&>().keep(source::body(std::declval<const std::variant_alternative_t<1, typename std::decay_t<decltype(source::header_current_or_future(std::declval<const S&>()))>::value_type>&>()))) && ::SCE::Generated::CodecZenohInterestBody::is_nothrow_source<std::decay_t<decltype(source::body(std::declval<const std::variant_alternative_t<1, typename std::decay_t<decltype(source::header_current_or_future(std::declval<const S&>()))>::value_type>&>()))>>::value && noexcept(std::declval<::SCE::Forge::Detail::Kept<decltype(source::body(std::declval<const std::variant_alternative_t<2, typename std::decay_t<decltype(source::header_current_or_future(std::declval<const S&>()))>::value_type>&>()))>&>().keep(source::body(std::declval<const std::variant_alternative_t<2, typename std::decay_t<decltype(source::header_current_or_future(std::declval<const S&>()))>::value_type>&>()))) && ::SCE::Generated::CodecZenohInterestBody::is_nothrow_source<std::decay_t<decltype(source::body(std::declval<const std::variant_alternative_t<2, typename std::decay_t<decltype(source::header_current_or_future(std::declval<const S&>()))>::value_type>&>()))>>::value && noexcept(source::extensions(std::declval<const S&>())) && ::SCE::Generated::CodecZenohExtEntry::is_nothrow_source<::SCE::Forge::Detail::ElementOf<typename std::decay_t<decltype(source::extensions(std::declval<const S&>()))>::value_type>>::value> {};

/// The encode of `CodecZenohInterest` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecZenohInterest: header, id, header_current_or_future, extensions");
        const auto& _r_extensions = source::extensions(src);
        const auto& _r_header_current_or_future = source::header_current_or_future(src);
        ::SCE::Forge::Detail::Kept<decltype(source::body(std::declval<const std::variant_alternative_t<0, typename std::decay_t<decltype(_r_header_current_or_future)>::value_type>&>()))> _r_body;
        if (_r_header_current_or_future) {
            if (const auto* _a = std::get_if<0>(&*_r_header_current_or_future)) {
                _r_body.keep(source::body(*_a));
            }
            if (const auto* _a = std::get_if<1>(&*_r_header_current_or_future)) {
                _r_body.keep(source::body(*_a));
            }
            if (const auto* _a = std::get_if<2>(&*_r_header_current_or_future)) {
                _r_body.keep(source::body(*_a));
            }
        }
        std::uint64_t _f_header = static_cast<std::uint64_t>(source::header(src));
        _f_header = (_f_header & ~0x20ull) | (static_cast<std::uint64_t>(static_cast<std::uint8_t>(!_r_header_current_or_future.has_value() ? 0 : ((_r_header_current_or_future->index() == 0u || _r_header_current_or_future->index() == 2u) ? 1 : 0))) << 5);
        _f_header = (_f_header & ~0x40ull) | (static_cast<std::uint64_t>(static_cast<std::uint8_t>(!_r_header_current_or_future.has_value() ? 0 : ((_r_header_current_or_future->index() == 1u || _r_header_current_or_future->index() == 2u) ? 1 : 0))) << 6);
        _f_header = (_f_header & ~0x80ull) | (static_cast<std::uint64_t>(_r_extensions.has_value()) << 7);
        const uint8_t _c_header = static_cast<uint8_t>(_f_header);
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_c_header)); _e) return _e;
        if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(source::id(src))); _e) return _e;
        if (_r_body) {
            if (auto _e = ::SCE::Generated::CodecZenohInterestBody::Wire::encode_source(*_r_body, w); _e) return _e;
        }
        if (_r_extensions) {
            {
                std::size_t _left_extensions = (*_r_extensions).size();
                for (const auto& _x : (*_r_extensions)) {
                    --_left_extensions;
                    ::SCE::Generated::CodecZenohExtEntry::Overrides _o_extensions{};
                    _o_extensions.header_z = _left_extensions != 0;
                    if (auto _e = ::SCE::Generated::CodecZenohExtEntry::Wire::encode_source(_x, w, _o_extensions); _e) return _e;
                }
            }
        }
        return std::nullopt;
    }
};

struct CodecZenohInterest {
    CodecZenohInterest() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecZenohInterest(uint8_t header, uint64_t id, std::optional<CodecZenohInterestHeaderCurrentOrFuture> header_current_or_future, std::optional<::SCE::Forge::List<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry, ::SCE::Forge::kUnbounded, 1>> extensions)
        : header_(header), id_(id), header_current_or_future_(std::move(header_current_or_future)), extensions_(extensions) {}
    /// The `header` element.
    [[nodiscard]] constexpr uint8_t header() const noexcept { return header_; }
    /// The `id` element.
    [[nodiscard]] constexpr uint64_t id() const noexcept { return id_; }
    /// The fields `header.CURRENT || header.FUTURE` decides, where its gate leaves them in.
    [[nodiscard]] const std::optional<CodecZenohInterestHeaderCurrentOrFuture>& header_current_or_future() const noexcept { return header_current_or_future_; }
    /// The `extensions` element, the entries it holds where a gate leaves them in.
    [[nodiscard]] std::optional<::SCE::Forge::List<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry, ::SCE::Forge::kUnbounded, 1>> extensions() const noexcept { if (!extensions_) { return std::nullopt; } return ::SCE::Forge::List<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry, ::SCE::Forge::kUnbounded, 1>(*extensions_); }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohInterest> decode(::SCE::Forge::SceCursor& cursor) {
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
        auto id_opt = cursor.read_vle_u64();
        if (!id_opt.has_value()) return std::nullopt;
        auto id = static_cast<std::uint64_t>(*id_opt);
        std::optional<::SCE::Generated::CodecZenohInterestBody::CodecZenohInterestBody> body;
        if ((header & 0x20) != 0 || (header & 0x40) != 0) {
            auto _emb = ::SCE::Generated::CodecZenohInterestBody::CodecZenohInterestBody::decode(cursor);
            if (!_emb.has_value()) return std::nullopt;
            body = std::move(*_emb);
        }
        std::optional<std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>> extensions;
        if ((header & 0x80) != 0) {
            std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry> _list;
            bool _more = false;
            for (;;) {
                auto _elem = ::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry::decode(cursor);
                if (!_elem.has_value()) return std::nullopt;
                _more = _elem->z();
                _list.push_back(*_elem);
                if (!_more) break;
            }
            if (_more && cursor.remaining() == 0) { return std::nullopt; }
            extensions = std::move(_list);
        }
        std::optional<::SCE::Forge::List<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry, ::SCE::Forge::kUnbounded, 1>> _b_extensions;
        if (extensions) {
            _b_extensions = ::SCE::Forge::List<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry, ::SCE::Forge::kUnbounded, 1>::make(*extensions);
            if (!_b_extensions) {
                cursor.refuse(::SCE::Forge::CodecError::TooManyElements);
                return std::nullopt;
            }
        }
        std::optional<CodecZenohInterestHeaderCurrentOrFuture> _g_header_current_or_future;
        if ((((header >> 5) & 0x1) != 0) && (((header >> 6) & 0x1) == 0)) {
            _g_header_current_or_future.emplace(std::in_place_index<0>, std::move(*body));
        } else if ((((header >> 5) & 0x1) == 0) && (((header >> 6) & 0x1) != 0)) {
            _g_header_current_or_future.emplace(std::in_place_index<1>, std::move(*body));
        } else if ((((header >> 5) & 0x1) != 0) && (((header >> 6) & 0x1) != 0)) {
            _g_header_current_or_future.emplace(std::in_place_index<2>, std::move(*body));
        }
        CodecZenohInterest _decoded{
            /*header=*/header,
            /*id=*/id,
            /*header_current_or_future=*/std::move(_g_header_current_or_future),
            /*extensions=*/_b_extensions,
        };
        return _decoded;
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    uint8_t mid() const noexcept {
        return static_cast<uint8_t>(
            (this->header_ >> 0) & static_cast<uint8_t>(0x1F)
        );
    }

    void set_mid(uint8_t v) noexcept {
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

    bool current() const noexcept {
        return (this->header_ & 0x20) != 0;
    }

    void set_current(bool v) noexcept {
        if (v) {
            this->header_ = static_cast<uint8_t>(this->header_ | 0x20);
        } else {
            this->header_ = static_cast<uint8_t>(this->header_ & static_cast<uint8_t>(~0x20));
        }
    }

    bool future() const noexcept {
        return (this->header_ & 0x40) != 0;
    }

    void set_future(bool v) noexcept {
        if (v) {
            this->header_ = static_cast<uint8_t>(this->header_ | 0x40);
        } else {
            this->header_ = static_cast<uint8_t>(this->header_ & static_cast<uint8_t>(~0x40));
        }
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
    uint8_t header_{0x19u};
    uint64_t id_{};
    std::optional<CodecZenohInterestHeaderCurrentOrFuture> header_current_or_future_{};
    std::optional<::SCE::Forge::Bounded<std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>, ::SCE::Forge::kUnbounded, 1>> extensions_{};
};

}  // namespace SCE::Generated::CodecZenohInterest

#endif  // SCE_FORGE_CODEC_ZENOH_INTEREST_H
