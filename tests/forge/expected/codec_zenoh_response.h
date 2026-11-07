// SCE-MAP: codec_zenoh_response:76 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_RESPONSE_H
#define SCE_FORGE_CODEC_ZENOH_RESPONSE_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <variant>
#include <string>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_zenoh_ext_entry.h"
#include "codec_zenoh_reply.h"
#include "codec_zenoh_err.h"

namespace SCE::Generated::CodecZenohResponse {

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. `std::variant` carries one of N arm bodies
// (each an imported codec type); the optional Default arm is a small
// struct that bundles the runtime tag value with the catch-all body.
struct CodecZenohResponseDefault {
    uint8_t tag;
    ::SCE::Generated::CodecZenohReply::CodecZenohReply body;
};
using CodecZenohResponseVariant = std::variant<
    ::SCE::Generated::CodecZenohReply::CodecZenohReply,
    ::SCE::Generated::CodecZenohErr::CodecZenohErr,
    CodecZenohResponseDefault
>;

/// The elements a value of `CodecZenohResponse` presents, each read by name: the
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
namespace detail_request_id {
void request_id() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.request_id())) -> decltype(s.request_id()) {
    return s.request_id();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(request_id(s))) -> decltype(request_id(s)) {
    return request_id(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_request_id
/// The `request_id` element of a source.
inline constexpr detail_request_id::Read request_id{};
namespace detail_key_id {
void key_id() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.key_id())) -> decltype(s.key_id()) {
    return s.key_id();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(key_id(s))) -> decltype(key_id(s)) {
    return key_id(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_key_id
/// The `key_id` element of a source.
inline constexpr detail_key_id::Read key_id{};
namespace detail_suffix {
void suffix() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.suffix())) -> decltype(s.suffix()) {
    return s.suffix();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(suffix(s))) -> decltype(suffix(s)) {
    return suffix(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_suffix
/// The `suffix` element of a source.
inline constexpr detail_suffix::Read suffix{};
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

/// Whether `D` is the catch-all arm of a source's body: the tag it holds,
/// and its body, a source of the catch-all's codec.
template <class D, class = void>
struct is_default_arm : std::false_type {};
template <class D>
struct is_default_arm<D, std::void_t<decltype(std::declval<const D&>().tag), decltype(std::declval<const D&>().body)>>
    : std::conjunction<std::is_convertible<decltype(std::declval<const D&>().tag), std::uint64_t>,
                       ::SCE::Generated::CodecZenohReply::is_source<std::decay_t<decltype(std::declval<const D&>().body)>>> {};

/// Whether `R` is a body a source may present: the arm it holds, of a
/// `std::variant` of a source of each arm's codec in turn.
template <class R, class = void>
struct is_body : std::false_type {};
template <class R>
struct is_body<R, std::enable_if_t<::SCE::Forge::Detail::IsVariantOf<R, 3>::value>>
    : std::conjunction<::SCE::Generated::CodecZenohReply::is_source<std::variant_alternative_t<0, R>>, ::SCE::Generated::CodecZenohErr::is_source<std::variant_alternative_t<1, R>>, is_default_arm<std::variant_alternative_t<2, R>>> {};

/// Whether `S` presents every element of `CodecZenohResponse`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::header(std::declval<const S&>())), decltype(source::request_id(std::declval<const S&>())), decltype(source::key_id(std::declval<const S&>())), decltype(source::suffix(std::declval<const S&>())), decltype(source::extensions(std::declval<const S&>())), decltype(source::body(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::header(std::declval<const S&>())), uint8_t>, std::is_convertible<decltype(source::request_id(std::declval<const S&>())), uint64_t>, std::is_convertible<decltype(source::key_id(std::declval<const S&>())), uint32_t>, std::is_convertible<decltype(source::suffix(std::declval<const S&>())), std::optional<::SCE::Forge::Text<65535>>>, is_extensions_present_list<std::decay_t<decltype(source::extensions(std::declval<const S&>()))>>, is_body<std::decay_t<decltype(source::body(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::header(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint64_t>(source::request_id(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint32_t>(source::key_id(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<std::optional<::SCE::Forge::Text<65535>>>(source::suffix(std::declval<const S&>()))) && noexcept(source::extensions(std::declval<const S&>())) && ::SCE::Generated::CodecZenohExtEntry::is_nothrow_source<::SCE::Forge::Detail::ElementOf<typename std::decay_t<decltype(source::extensions(std::declval<const S&>()))>::value_type>>::value && noexcept(source::body(std::declval<const S&>())) && ::SCE::Generated::CodecZenohReply::is_nothrow_source<std::variant_alternative_t<0, std::decay_t<decltype(source::body(std::declval<const S&>()))>>>::value && ::SCE::Generated::CodecZenohErr::is_nothrow_source<std::variant_alternative_t<1, std::decay_t<decltype(source::body(std::declval<const S&>()))>>>::value && ::SCE::Generated::CodecZenohReply::is_nothrow_source<std::decay_t<decltype(std::declval<const std::variant_alternative_t<2, std::decay_t<decltype(source::body(std::declval<const S&>()))>>&>().body)>>::value> {};

/// One-bit flags of `CodecZenohResponse` a parent may set in place of the ones a value
/// presents, such as the flag a chain whose entries are `CodecZenohResponse` ends them
/// on, which the chain derives from where each entry stands.
struct Overrides {
    /// The `M` flag of `header`, where a parent sets it.
    std::optional<bool> header_m;
};

/// The encode of `CodecZenohResponse` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w, const Overrides& overrides = Overrides{}) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecZenohResponse: header, request_id, key_id, suffix, extensions, body");
        const auto& _r_body = source::body(src);
        const auto& _r_suffix = source::suffix(src);
        const std::optional<::SCE::Forge::Text<65535>> _e_suffix = _r_suffix;
        const auto& _r_extensions = source::extensions(src);
        const uint64_t _d_suffix_len = _e_suffix.has_value() ? static_cast<uint64_t>(_e_suffix->size()) : uint64_t{0};
        std::uint64_t _f_header = static_cast<std::uint64_t>(source::header(src));
        _f_header = (_f_header & ~0x20ull) | (static_cast<std::uint64_t>(_e_suffix.has_value()) << 5);
        _f_header = (_f_header & ~0x80ull) | (static_cast<std::uint64_t>(_r_extensions.has_value()) << 7);
        if (overrides.header_m.has_value()) {
            _f_header = (_f_header & ~0x40ull) | (static_cast<std::uint64_t>(*overrides.header_m) << 6);
        }
        const uint8_t _c_header = static_cast<uint8_t>(_f_header);
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_c_header)); _e) return _e;
        if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(source::request_id(src))); _e) return _e;
        if (auto _e = w.write_vle_u32(static_cast<std::uint32_t>(source::key_id(src))); _e) return _e;
        if (((static_cast<std::uint64_t>(_c_header) & 0x20ull) != 0)) {
            if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(_d_suffix_len)); _e) return _e;
        }
        if (_e_suffix) {
            if (auto _e = w.write_bytes(reinterpret_cast<const std::uint8_t*>(_e_suffix->data()), _e_suffix->size()); _e) return _e;
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
        switch (_r_body.index()) {
            case 0:
                if (auto _e = ::SCE::Generated::CodecZenohReply::Wire::encode_source(*std::get_if<0>(&_r_body), w); _e) return _e;
                break;
            case 1:
                if (auto _e = ::SCE::Generated::CodecZenohErr::Wire::encode_source(*std::get_if<1>(&_r_body), w); _e) return _e;
                break;
            case 2:
                if (auto _e = ::SCE::Generated::CodecZenohReply::Wire::encode_source(std::get_if<2>(&_r_body)->body, w); _e) return _e;
                break;
            default: break;
        }
        return std::nullopt;
    }
};

struct CodecZenohResponse {
    CodecZenohResponse() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecZenohResponse(uint8_t header, uint64_t request_id, uint32_t key_id, std::optional<::SCE::Forge::Text<65535>> suffix, std::optional<::SCE::Forge::List<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry, ::SCE::Forge::kUnbounded, 1>> extensions, CodecZenohResponseVariant body)
        : header_(header), request_id_(request_id), key_id_(key_id), suffix_(suffix), extensions_(extensions), body_(std::move(body)) {}
    /// The `header` element.
    [[nodiscard]] constexpr uint8_t header() const noexcept { return header_; }
    /// The `request_id` element.
    [[nodiscard]] constexpr uint64_t request_id() const noexcept { return request_id_; }
    /// The `key_id` element.
    [[nodiscard]] constexpr uint32_t key_id() const noexcept { return key_id_; }
    /// The `suffix` element, where a gate leaves it in.
    [[nodiscard]] std::optional<::SCE::Forge::Text<65535>> suffix() const noexcept { if (!suffix_) { return std::nullopt; } return ::SCE::Forge::Text<65535>(*suffix_); }
    /// The `extensions` element, the entries it holds where a gate leaves them in.
    [[nodiscard]] std::optional<::SCE::Forge::List<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry, ::SCE::Forge::kUnbounded, 1>> extensions() const noexcept { if (!extensions_) { return std::nullopt; } return ::SCE::Forge::List<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry, ::SCE::Forge::kUnbounded, 1>(*extensions_); }
    /// The arm the value holds, each arm's body the value of its codec.
    [[nodiscard]] const CodecZenohResponseVariant& body() const noexcept { return body_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohResponse> decode(::SCE::Forge::SceCursor& cursor) {
        // RFC §synth-5-B peek-byte / streaming-prefix:
        // streaming prefix decode (variable-length fields supported via
        // per-field present_if/tlv-chain/embed/repeat helpers). Peek-byte
        // mode additionally peeks the cursor's next byte for variant tag
        // without advancing — arm body decoder reads it as its own
        // header byte.
        uint8_t header;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            header = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        auto request_id_opt = cursor.read_vle_u64();
        if (!request_id_opt.has_value()) return std::nullopt;
        auto request_id = static_cast<std::uint64_t>(*request_id_opt);
        auto key_id_opt = cursor.read_vle_u32();
        if (!key_id_opt.has_value()) return std::nullopt;
        auto key_id = static_cast<std::uint32_t>(*key_id_opt);
        std::optional<uint64_t> suffix_len;
        if ((header & 0x20) != 0) {
            auto _v_opt = cursor.read_vle_u64();
        if (!_v_opt.has_value()) return std::nullopt;
        auto _v = static_cast<std::uint64_t>(*_v_opt);
            suffix_len = _v;
        }
        std::optional<std::string> suffix;
        if ((header & 0x20) != 0) {
            const auto _len = suffix_len.value();
            if (_len > 65535) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange); return std::nullopt;
            }
            std::size_t _n = static_cast<std::size_t>(_len);
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            if (!::SCE::Forge::is_valid_utf8(raw, _n)) return std::nullopt;
            suffix.emplace(reinterpret_cast<const char*>(raw), _n);
            if (!cursor.advance(_n)) return std::nullopt;
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
        const std::uint8_t* _peek_raw = cursor.peek_slice(1);
        if (_peek_raw == nullptr) return std::nullopt;
        const std::uint8_t _peek = _peek_raw[0];
        // Dispatch on tag value into the matching arm body.
        CodecZenohResponseVariant body;
        switch (static_cast<uint8_t>((_peek >> 0) & static_cast<uint8_t>(0x1F))) {
            case 4: {
                auto _arm = ::SCE::Generated::CodecZenohReply::CodecZenohReply::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 5: {
                auto _arm = ::SCE::Generated::CodecZenohErr::CodecZenohErr::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            default: {
                auto _arm = ::SCE::Generated::CodecZenohReply::CodecZenohReply::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = CodecZenohResponseDefault{
                    /*tag=*/static_cast<uint8_t>((_peek >> 0) & static_cast<uint8_t>(0x1F)),
                    /*body=*/*_arm,
                };
                break;
            }
        }
        std::optional<::SCE::Forge::List<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry, ::SCE::Forge::kUnbounded, 1>> _b_extensions;
        if (extensions) {
            _b_extensions = ::SCE::Forge::List<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry, ::SCE::Forge::kUnbounded, 1>::make(*extensions);
            if (!_b_extensions) {
                cursor.refuse(::SCE::Forge::CodecError::TooManyElements);
                return std::nullopt;
            }
        }
        std::optional<::SCE::Forge::Text<65535>> _b_suffix;
        if (suffix) {
            _b_suffix = ::SCE::Forge::Text<65535>::make(*suffix);
            if (!_b_suffix) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);
                return std::nullopt;
            }
        }
        (void)suffix_len;
        CodecZenohResponse _decoded{
            /*header=*/header,
            /*request_id=*/request_id,
            /*key_id=*/key_id,
            /*suffix=*/_b_suffix,
            /*extensions=*/_b_extensions,
            /*body=*/std::move(body),
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

    bool n() const noexcept {
        return (this->header_ & 0x20) != 0;
    }

    void set_n(bool v) noexcept {
        if (v) {
            this->header_ = static_cast<uint8_t>(this->header_ | 0x20);
        } else {
            this->header_ = static_cast<uint8_t>(this->header_ & static_cast<uint8_t>(~0x20));
        }
    }

    bool m() const noexcept {
        return (this->header_ & 0x40) != 0;
    }

    void set_m(bool v) noexcept {
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
    /// freshly-encoded byte vector. Callers targeting zero-alloc hot
    /// paths should call `encode` directly against a caller-owned sink.
    [[nodiscard]] std::vector<std::uint8_t> encode_to_vec() const {
        std::vector<std::uint8_t> _sce_v;
        ::SCE::Forge::VectorSink _sce_sink(_sce_v);
        (void)encode(_sce_sink);
        return _sce_v;
    }

private:
    uint8_t header_{0x1bu};
    uint64_t request_id_{};
    uint32_t key_id_{};
    std::optional<::SCE::Forge::Bounded<std::string, 65535>> suffix_{};
    std::optional<::SCE::Forge::Bounded<std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>, ::SCE::Forge::kUnbounded, 1>> extensions_{};
    CodecZenohResponseVariant body_{std::in_place_index_t<0>{}};
};

}  // namespace SCE::Generated::CodecZenohResponse

#endif  // SCE_FORGE_CODEC_ZENOH_RESPONSE_H
