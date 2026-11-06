// SCE-MAP: codec_zenoh_reply:54 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_REPLY_H
#define SCE_FORGE_CODEC_ZENOH_REPLY_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_zenoh_ext_entry.h"
#include "codec_zenoh_push_body.h"

namespace SCE::Generated::CodecZenohReply {

/// The elements a value of `CodecZenohReply` presents, each read by name: the
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
namespace detail_consolidation {
void consolidation() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.consolidation())) -> decltype(s.consolidation()) {
    return s.consolidation();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(consolidation(s))) -> decltype(consolidation(s)) {
    return consolidation(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_consolidation
/// The `consolidation` element of a source.
inline constexpr detail_consolidation::Read consolidation{};
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

/// Whether `S` presents every element of `CodecZenohReply`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::header(std::declval<const S&>())), decltype(source::consolidation(std::declval<const S&>())), decltype(source::extensions(std::declval<const S&>())), decltype(source::body(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::header(std::declval<const S&>())), uint8_t>, std::is_convertible<decltype(source::consolidation(std::declval<const S&>())), std::optional<uint8_t>>, is_extensions_present_list<std::decay_t<decltype(source::extensions(std::declval<const S&>()))>>, ::SCE::Generated::CodecZenohPushBody::is_source<std::decay_t<decltype(source::body(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::header(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<std::optional<uint8_t>>(source::consolidation(std::declval<const S&>()))) && noexcept(source::extensions(std::declval<const S&>())) && ::SCE::Generated::CodecZenohExtEntry::is_nothrow_source<::SCE::Forge::Detail::ElementOf<typename std::decay_t<decltype(source::extensions(std::declval<const S&>()))>::value_type>>::value && noexcept(source::body(std::declval<const S&>())) && ::SCE::Generated::CodecZenohPushBody::is_nothrow_source<std::decay_t<decltype(source::body(std::declval<const S&>()))>>::value> {};

/// One-bit flags of `CodecZenohReply` a parent may set in place of the ones a value
/// presents, such as the flag a chain whose entries are `CodecZenohReply` ends them
/// on, which the chain derives from where each entry stands.
struct Overrides {
    /// The `X` flag of `header`, where a parent sets it.
    std::optional<bool> header_x;
};

/// The encode of `CodecZenohReply` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w, const Overrides& overrides = Overrides{}) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecZenohReply: header, consolidation, extensions, body");
        const auto& _r_consolidation = source::consolidation(src);
        const std::optional<uint8_t> _e_consolidation = _r_consolidation;
        const auto& _r_extensions = source::extensions(src);
        const auto& _r_body = source::body(src);
        std::uint64_t _f_header = static_cast<std::uint64_t>(source::header(src));
        _f_header = (_f_header & ~0x20ull) | (static_cast<std::uint64_t>(_e_consolidation.has_value()) << 5);
        _f_header = (_f_header & ~0x80ull) | (static_cast<std::uint64_t>(_r_extensions.has_value()) << 7);
        if (overrides.header_x.has_value()) {
            _f_header = (_f_header & ~0x40ull) | (static_cast<std::uint64_t>(*overrides.header_x) << 6);
        }
        const uint8_t _c_header = static_cast<uint8_t>(_f_header);
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_c_header)); _e) return _e;
        if (_e_consolidation) {
            if (auto _e = w.write_u8(static_cast<std::uint8_t>((*_e_consolidation))); _e) return _e;
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
        if (auto _e = ::SCE::Generated::CodecZenohPushBody::Wire::encode_source(_r_body, w); _e) return _e;
        return std::nullopt;
    }
};

struct CodecZenohReply {
    CodecZenohReply() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecZenohReply(uint8_t header, std::optional<uint8_t> consolidation, std::optional<::SCE::Forge::List<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry, ::SCE::Forge::kUnbounded, 1>> extensions, ::SCE::Generated::CodecZenohPushBody::CodecZenohPushBody body)
        : header_(header), consolidation_(consolidation), extensions_(extensions), body_(std::move(body)) {}
    /// The `header` element.
    [[nodiscard]] constexpr uint8_t header() const noexcept { return header_; }
    /// The `consolidation` element, where a gate leaves it in.
    [[nodiscard]] constexpr std::optional<uint8_t> consolidation() const noexcept { return consolidation_; }
    /// The `extensions` element, the entries it holds where a gate leaves them in.
    [[nodiscard]] std::optional<::SCE::Forge::List<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry, ::SCE::Forge::kUnbounded, 1>> extensions() const noexcept { if (!extensions_) { return std::nullopt; } return ::SCE::Forge::List<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry, ::SCE::Forge::kUnbounded, 1>(*extensions_); }
    /// The `body` element, the body it holds.
    [[nodiscard]] const ::SCE::Generated::CodecZenohPushBody::CodecZenohPushBody& body() const noexcept { return body_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohReply> decode(::SCE::Forge::SceCursor& cursor) {
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
        std::optional<uint8_t> consolidation;
        if ((header & 0x20) != 0) {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            consolidation = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
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
        auto _emb_body = ::SCE::Generated::CodecZenohPushBody::CodecZenohPushBody::decode(cursor);
        if (!_emb_body.has_value()) return std::nullopt;
        auto body = std::move(*_emb_body);
        std::optional<::SCE::Forge::List<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry, ::SCE::Forge::kUnbounded, 1>> _b_extensions;
        if (extensions) {
            _b_extensions = ::SCE::Forge::List<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry, ::SCE::Forge::kUnbounded, 1>::make(*extensions);
            if (!_b_extensions) {
                cursor.refuse(::SCE::Forge::CodecError::TooManyElements);
                return std::nullopt;
            }
        }
        CodecZenohReply _decoded{
            /*header=*/header,
            /*consolidation=*/consolidation,
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

    bool c() const noexcept {
        return (this->header_ & 0x20) != 0;
    }

    void set_c(bool v) noexcept {
        if (v) {
            this->header_ = static_cast<uint8_t>(this->header_ | 0x20);
        } else {
            this->header_ = static_cast<uint8_t>(this->header_ & static_cast<uint8_t>(~0x20));
        }
    }

    bool x() const noexcept {
        return (this->header_ & 0x40) != 0;
    }

    void set_x(bool v) noexcept {
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
    uint8_t header_{0x04u};
    std::optional<uint8_t> consolidation_{};
    std::optional<::SCE::Forge::Bounded<std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>, ::SCE::Forge::kUnbounded, 1>> extensions_{};
    ::SCE::Generated::CodecZenohPushBody::CodecZenohPushBody body_{};
};

}  // namespace SCE::Generated::CodecZenohReply

#endif  // SCE_FORGE_CODEC_ZENOH_REPLY_H
