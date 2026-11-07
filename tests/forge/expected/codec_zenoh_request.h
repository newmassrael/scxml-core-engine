// SCE-MAP: codec_zenoh_request:73 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_REQUEST_H
#define SCE_FORGE_CODEC_ZENOH_REQUEST_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <variant>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_zenoh_wireexpr.h"
#include "codec_zenoh_ext_entry.h"
#include "codec_zenoh_msg_put.h"
#include "codec_zenoh_msg_del.h"
#include "codec_zenoh_query.h"

namespace SCE::Generated::CodecZenohRequest {

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. `std::variant` carries one of N arm bodies
// (each an imported codec type); the optional Default arm is a small
// struct that bundles the runtime tag value with the catch-all body.
struct CodecZenohRequestDefault {
    uint8_t tag;
    ::SCE::Generated::CodecZenohQuery::CodecZenohQuery body;
};
using CodecZenohRequestVariant = std::variant<
    ::SCE::Generated::CodecZenohMsgPut::CodecZenohMsgPut,
    ::SCE::Generated::CodecZenohMsgDel::CodecZenohMsgDel,
    ::SCE::Generated::CodecZenohQuery::CodecZenohQuery,
    CodecZenohRequestDefault
>;

/// The elements a value of `CodecZenohRequest` presents, each read by name: the
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
namespace detail_rid {
void rid() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.rid())) -> decltype(s.rid()) {
    return s.rid();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(rid(s))) -> decltype(rid(s)) {
    return rid(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_rid
/// The `rid` element of a source.
inline constexpr detail_rid::Read rid{};
namespace detail_keyexpr {
void keyexpr() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.keyexpr())) -> decltype(s.keyexpr()) {
    return s.keyexpr();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(keyexpr(s))) -> decltype(keyexpr(s)) {
    return keyexpr(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_keyexpr
/// The `keyexpr` element of a source.
inline constexpr detail_keyexpr::Read keyexpr{};
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
                       ::SCE::Generated::CodecZenohQuery::is_source<std::decay_t<decltype(std::declval<const D&>().body)>>> {};

/// Whether `R` is a body a source may present: the arm it holds, of a
/// `std::variant` of a source of each arm's codec in turn.
template <class R, class = void>
struct is_body : std::false_type {};
template <class R>
struct is_body<R, std::enable_if_t<::SCE::Forge::Detail::IsVariantOf<R, 4>::value>>
    : std::conjunction<::SCE::Generated::CodecZenohMsgPut::is_source<std::variant_alternative_t<0, R>>, ::SCE::Generated::CodecZenohMsgDel::is_source<std::variant_alternative_t<1, R>>, ::SCE::Generated::CodecZenohQuery::is_source<std::variant_alternative_t<2, R>>, is_default_arm<std::variant_alternative_t<3, R>>> {};

/// Whether `S` presents every element of `CodecZenohRequest`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::header(std::declval<const S&>())), decltype(source::rid(std::declval<const S&>())), decltype(source::keyexpr(std::declval<const S&>())), decltype(source::extensions(std::declval<const S&>())), decltype(source::body(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::header(std::declval<const S&>())), uint8_t>, std::is_convertible<decltype(source::rid(std::declval<const S&>())), uint64_t>, ::SCE::Generated::CodecZenohWireexpr::is_source<std::decay_t<decltype(source::keyexpr(std::declval<const S&>()))>>, is_extensions_present_list<std::decay_t<decltype(source::extensions(std::declval<const S&>()))>>, is_body<std::decay_t<decltype(source::body(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::header(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint64_t>(source::rid(std::declval<const S&>()))) && noexcept(source::keyexpr(std::declval<const S&>())) && ::SCE::Generated::CodecZenohWireexpr::is_nothrow_source<std::decay_t<decltype(source::keyexpr(std::declval<const S&>()))>>::value && noexcept(source::extensions(std::declval<const S&>())) && ::SCE::Generated::CodecZenohExtEntry::is_nothrow_source<::SCE::Forge::Detail::ElementOf<typename std::decay_t<decltype(source::extensions(std::declval<const S&>()))>::value_type>>::value && noexcept(source::body(std::declval<const S&>())) && ::SCE::Generated::CodecZenohMsgPut::is_nothrow_source<std::variant_alternative_t<0, std::decay_t<decltype(source::body(std::declval<const S&>()))>>>::value && ::SCE::Generated::CodecZenohMsgDel::is_nothrow_source<std::variant_alternative_t<1, std::decay_t<decltype(source::body(std::declval<const S&>()))>>>::value && ::SCE::Generated::CodecZenohQuery::is_nothrow_source<std::variant_alternative_t<2, std::decay_t<decltype(source::body(std::declval<const S&>()))>>>::value && ::SCE::Generated::CodecZenohQuery::is_nothrow_source<std::decay_t<decltype(std::declval<const std::variant_alternative_t<3, std::decay_t<decltype(source::body(std::declval<const S&>()))>>&>().body)>>::value> {};

/// One-bit flags of `CodecZenohRequest` a parent may set in place of the ones a value
/// presents, such as the flag a chain whose entries are `CodecZenohRequest` ends them
/// on, which the chain derives from where each entry stands.
struct Overrides {
    /// The `M` flag of `header`, where a parent sets it.
    std::optional<bool> header_m;
};

/// The encode of `CodecZenohRequest` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w, const Overrides& overrides = Overrides{}) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecZenohRequest: header, rid, keyexpr, extensions, body");
        const auto& _r_body = source::body(src);
        const auto& _r_keyexpr = source::keyexpr(src);
        const auto& _r_extensions = source::extensions(src);
        std::uint64_t _f_header = static_cast<std::uint64_t>(source::header(src));
        _f_header = (_f_header & ~0x20ull) | (static_cast<std::uint64_t>(::SCE::Generated::CodecZenohWireexpr::input_n(_r_keyexpr)) << 5);
        _f_header = (_f_header & ~0x80ull) | (static_cast<std::uint64_t>(_r_extensions.has_value()) << 7);
        if (overrides.header_m.has_value()) {
            _f_header = (_f_header & ~0x40ull) | (static_cast<std::uint64_t>(*overrides.header_m) << 6);
        }
        const uint8_t _c_header = static_cast<uint8_t>(_f_header);
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_c_header)); _e) return _e;
        if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(source::rid(src))); _e) return _e;
        if (auto _e = ::SCE::Generated::CodecZenohWireexpr::Wire::encode_source(_r_keyexpr, w); _e) return _e;
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
                if (auto _e = ::SCE::Generated::CodecZenohMsgPut::Wire::encode_source(*std::get_if<0>(&_r_body), w); _e) return _e;
                break;
            case 1:
                if (auto _e = ::SCE::Generated::CodecZenohMsgDel::Wire::encode_source(*std::get_if<1>(&_r_body), w); _e) return _e;
                break;
            case 2:
                if (auto _e = ::SCE::Generated::CodecZenohQuery::Wire::encode_source(*std::get_if<2>(&_r_body), w); _e) return _e;
                break;
            case 3:
                if (auto _e = ::SCE::Generated::CodecZenohQuery::Wire::encode_source(std::get_if<3>(&_r_body)->body, w); _e) return _e;
                break;
            default: break;
        }
        return std::nullopt;
    }
};

struct CodecZenohRequest {
    CodecZenohRequest() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecZenohRequest(uint8_t header, uint64_t rid, ::SCE::Generated::CodecZenohWireexpr::CodecZenohWireexpr keyexpr, std::optional<::SCE::Forge::List<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry, ::SCE::Forge::kUnbounded, 1>> extensions, CodecZenohRequestVariant body)
        : header_(header), rid_(rid), keyexpr_(std::move(keyexpr)), extensions_(extensions), body_(std::move(body)) {}
    /// The `header` element.
    [[nodiscard]] constexpr uint8_t header() const noexcept { return header_; }
    /// The `rid` element.
    [[nodiscard]] constexpr uint64_t rid() const noexcept { return rid_; }
    /// The `keyexpr` element, the body it holds.
    [[nodiscard]] const ::SCE::Generated::CodecZenohWireexpr::CodecZenohWireexpr& keyexpr() const noexcept { return keyexpr_; }
    /// The `extensions` element, the entries it holds where a gate leaves them in.
    [[nodiscard]] std::optional<::SCE::Forge::List<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry, ::SCE::Forge::kUnbounded, 1>> extensions() const noexcept { if (!extensions_) { return std::nullopt; } return ::SCE::Forge::List<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry, ::SCE::Forge::kUnbounded, 1>(*extensions_); }
    /// The arm the value holds, each arm's body the value of its codec.
    [[nodiscard]] const CodecZenohRequestVariant& body() const noexcept { return body_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohRequest> decode(::SCE::Forge::SceCursor& cursor) {
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
        auto rid_opt = cursor.read_vle_u64();
        if (!rid_opt.has_value()) return std::nullopt;
        auto rid = static_cast<std::uint64_t>(*rid_opt);
        auto _emb_keyexpr = ::SCE::Generated::CodecZenohWireexpr::CodecZenohWireexpr::decode(cursor, static_cast<std::uint8_t>((header >> 5) & 0x1));
        if (!_emb_keyexpr.has_value()) return std::nullopt;
        auto keyexpr = std::move(*_emb_keyexpr);
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
        CodecZenohRequestVariant body;
        switch (static_cast<uint8_t>((_peek >> 0) & static_cast<uint8_t>(0x1F))) {
            case 1: {
                auto _arm = ::SCE::Generated::CodecZenohMsgPut::CodecZenohMsgPut::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 2: {
                auto _arm = ::SCE::Generated::CodecZenohMsgDel::CodecZenohMsgDel::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            case 3: {
                auto _arm = ::SCE::Generated::CodecZenohQuery::CodecZenohQuery::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = *_arm;
                break;
            }
            default: {
                auto _arm = ::SCE::Generated::CodecZenohQuery::CodecZenohQuery::decode(cursor);
                if (!_arm.has_value()) return std::nullopt;
                body = CodecZenohRequestDefault{
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
        CodecZenohRequest _decoded{
            /*header=*/header,
            /*rid=*/rid,
            /*keyexpr=*/std::move(keyexpr),
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
    uint8_t header_{0x1cu};
    uint64_t rid_{};
    ::SCE::Generated::CodecZenohWireexpr::CodecZenohWireexpr keyexpr_{};
    std::optional<::SCE::Forge::Bounded<std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>, ::SCE::Forge::kUnbounded, 1>> extensions_{};
    CodecZenohRequestVariant body_{std::in_place_index_t<0>{}};
};

}  // namespace SCE::Generated::CodecZenohRequest

#endif  // SCE_FORGE_CODEC_ZENOH_REQUEST_H
