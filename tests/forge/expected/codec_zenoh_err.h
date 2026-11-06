// SCE-MAP: codec_zenoh_err:57 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_ERR_H
#define SCE_FORGE_CODEC_ZENOH_ERR_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_zenoh_encoding.h"
#include "codec_zenoh_ext_entry.h"

namespace SCE::Generated::CodecZenohErr {

/// The elements a value of `CodecZenohErr` presents, each read by name: the
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
namespace detail_encoding {
void encoding() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.encoding())) -> decltype(s.encoding()) {
    return s.encoding();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(encoding(s))) -> decltype(encoding(s)) {
    return encoding(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_encoding
/// The `encoding` element of a source.
inline constexpr detail_encoding::Read encoding{};
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
}  // namespace source

/// Whether `R` is what a source may present for `encoding`, a body a gate
/// decides: a `std::optional` of a source of the body's codec.
template <class R>
struct is_encoding_body : std::false_type {};
template <class B>
struct is_encoding_body<std::optional<B>> : ::SCE::Generated::CodecZenohEncoding::is_source<B> {};

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

/// Whether `S` presents every element of `CodecZenohErr`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::header(std::declval<const S&>())), decltype(source::encoding(std::declval<const S&>())), decltype(source::extensions(std::declval<const S&>())), decltype(source::payload(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::header(std::declval<const S&>())), uint8_t>, is_encoding_body<std::decay_t<decltype(source::encoding(std::declval<const S&>()))>>, is_extensions_present_list<std::decay_t<decltype(source::extensions(std::declval<const S&>()))>>, std::is_convertible<decltype(source::payload(std::declval<const S&>())), ::SCE::Forge::Bytes<4294967295>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::header(std::declval<const S&>()))) && noexcept(source::encoding(std::declval<const S&>())) && ::SCE::Generated::CodecZenohEncoding::is_nothrow_source<typename std::decay_t<decltype(source::encoding(std::declval<const S&>()))>::value_type>::value && noexcept(source::extensions(std::declval<const S&>())) && ::SCE::Generated::CodecZenohExtEntry::is_nothrow_source<::SCE::Forge::Detail::ElementOf<typename std::decay_t<decltype(source::extensions(std::declval<const S&>()))>::value_type>>::value && noexcept(::SCE::Forge::Detail::converts_to<::SCE::Forge::Bytes<4294967295>>(source::payload(std::declval<const S&>())))> {};

/// One-bit flags of `CodecZenohErr` a parent may set in place of the ones a value
/// presents, such as the flag a chain whose entries are `CodecZenohErr` ends them
/// on, which the chain derives from where each entry stands.
struct Overrides {
    /// The `X` flag of `header`, where a parent sets it.
    std::optional<bool> header_x;
};

/// The encode of `CodecZenohErr` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w, const Overrides& overrides = Overrides{}) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecZenohErr: header, encoding, extensions, payload");
        const auto& _r_encoding = source::encoding(src);
        const auto& _r_extensions = source::extensions(src);
        const auto& _r_payload = source::payload(src);
        const ::SCE::Forge::Bytes<4294967295> _e_payload = _r_payload;
        const uint64_t _d_payload_len = static_cast<uint64_t>(_e_payload.size());
        std::uint64_t _f_header = static_cast<std::uint64_t>(source::header(src));
        _f_header = (_f_header & ~0x40ull) | (static_cast<std::uint64_t>(_r_encoding.has_value()) << 6);
        _f_header = (_f_header & ~0x80ull) | (static_cast<std::uint64_t>(_r_extensions.has_value()) << 7);
        if (overrides.header_x.has_value()) {
            _f_header = (_f_header & ~0x20ull) | (static_cast<std::uint64_t>(*overrides.header_x) << 5);
        }
        const uint8_t _c_header = static_cast<uint8_t>(_f_header);
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_c_header)); _e) return _e;
        if (_r_encoding) {
            if (auto _e = ::SCE::Generated::CodecZenohEncoding::Wire::encode_source(*_r_encoding, w); _e) return _e;
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
        if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(_d_payload_len)); _e) return _e;
        if (auto _e = w.write_bytes(_e_payload.data(), _e_payload.size()); _e) return _e;
        return std::nullopt;
    }
};

struct CodecZenohErr {
    CodecZenohErr() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecZenohErr(uint8_t header, std::optional<::SCE::Generated::CodecZenohEncoding::CodecZenohEncoding> encoding, std::optional<::SCE::Forge::List<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry, ::SCE::Forge::kUnbounded, 1>> extensions, ::SCE::Forge::Bytes<4294967295> payload)
        : header_(header), encoding_(std::move(encoding)), extensions_(extensions), payload_(payload) {}
    /// The `header` element.
    [[nodiscard]] constexpr uint8_t header() const noexcept { return header_; }
    /// The `encoding` element, the body it holds where a gate leaves it in.
    [[nodiscard]] const std::optional<::SCE::Generated::CodecZenohEncoding::CodecZenohEncoding>& encoding() const noexcept { return encoding_; }
    /// The `extensions` element, the entries it holds where a gate leaves them in.
    [[nodiscard]] std::optional<::SCE::Forge::List<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry, ::SCE::Forge::kUnbounded, 1>> extensions() const noexcept { if (!extensions_) { return std::nullopt; } return ::SCE::Forge::List<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry, ::SCE::Forge::kUnbounded, 1>(*extensions_); }
    /// The `payload` element.
    [[nodiscard]] ::SCE::Forge::Bytes<4294967295> payload() const noexcept { return payload_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohErr> decode(::SCE::Forge::SceCursor& cursor) {
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
        std::optional<::SCE::Generated::CodecZenohEncoding::CodecZenohEncoding> encoding;
        if ((header & 0x40) != 0) {
            auto _emb = ::SCE::Generated::CodecZenohEncoding::CodecZenohEncoding::decode(cursor);
            if (!_emb.has_value()) return std::nullopt;
            encoding = std::move(*_emb);
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
        auto payload_len_opt = cursor.read_vle_u64();
        if (!payload_len_opt.has_value()) return std::nullopt;
        auto payload_len = static_cast<std::uint64_t>(*payload_len_opt);
        std::vector<uint8_t> payload;
        {
            const auto _len = payload_len;
            if (_len > 4294967295ULL) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange); return std::nullopt;
            }
            if (static_cast<std::uint64_t>(_len) > static_cast<std::uint64_t>(SIZE_MAX) - 0u) return std::nullopt;
            std::size_t _n = static_cast<std::size_t>(_len);
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            payload.assign(raw, raw + _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        std::optional<::SCE::Forge::List<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry, ::SCE::Forge::kUnbounded, 1>> _b_extensions;
        if (extensions) {
            _b_extensions = ::SCE::Forge::List<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry, ::SCE::Forge::kUnbounded, 1>::make(*extensions);
            if (!_b_extensions) {
                cursor.refuse(::SCE::Forge::CodecError::TooManyElements);
                return std::nullopt;
            }
        }
        const auto _b_payload = ::SCE::Forge::Bytes<4294967295>::make(payload);
        if (!_b_payload) {
            cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);
            return std::nullopt;
        }
        (void)payload_len;
        CodecZenohErr _decoded{
            /*header=*/header,
            /*encoding=*/std::move(encoding),
            /*extensions=*/_b_extensions,
            /*payload=*/*_b_payload,
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

    bool x() const noexcept {
        return (this->header_ & 0x20) != 0;
    }

    void set_x(bool v) noexcept {
        if (v) {
            this->header_ = static_cast<uint8_t>(this->header_ | 0x20);
        } else {
            this->header_ = static_cast<uint8_t>(this->header_ & static_cast<uint8_t>(~0x20));
        }
    }

    bool e() const noexcept {
        return (this->header_ & 0x40) != 0;
    }

    void set_e(bool v) noexcept {
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
    uint8_t header_{0x05u};
    std::optional<::SCE::Generated::CodecZenohEncoding::CodecZenohEncoding> encoding_{};
    std::optional<::SCE::Forge::Bounded<std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry>, ::SCE::Forge::kUnbounded, 1>> extensions_{};
    ::SCE::Forge::Bounded<std::vector<std::uint8_t>, 4294967295> payload_{};
};

}  // namespace SCE::Generated::CodecZenohErr

#endif  // SCE_FORGE_CODEC_ZENOH_ERR_H
