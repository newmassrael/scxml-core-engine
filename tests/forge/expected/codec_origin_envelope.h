// SCE-MAP: codec_origin_envelope:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ORIGIN_ENVELOPE_H
#define SCE_FORGE_CODEC_ORIGIN_ENVELOPE_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <string>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_origin_leaf.h"

namespace SCE::Generated::CodecOriginEnvelope {

/// The elements a value of `CodecOriginEnvelope` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_hdr {
void hdr() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.hdr())) -> decltype(s.hdr()) {
    return s.hdr();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(hdr(s))) -> decltype(hdr(s)) {
    return hdr(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_hdr
/// The `hdr` element of a source.
inline constexpr detail_hdr::Read hdr{};
namespace detail_note {
void note() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.note())) -> decltype(s.note()) {
    return s.note();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(note(s))) -> decltype(note(s)) {
    return note(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_note
/// The `note` element of a source.
inline constexpr detail_note::Read note{};
namespace detail_required {
void required() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.required())) -> decltype(s.required()) {
    return s.required();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(required(s))) -> decltype(required(s)) {
    return required(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_required
/// The `required` element of a source.
inline constexpr detail_required::Read required{};
namespace detail_optional {
void optional() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.optional())) -> decltype(s.optional()) {
    return s.optional();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(optional(s))) -> decltype(optional(s)) {
    return optional(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_optional
/// The `optional` element of a source.
inline constexpr detail_optional::Read optional{};
namespace detail_items {
void items() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.items())) -> decltype(s.items()) {
    return s.items();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(items(s))) -> decltype(items(s)) {
    return items(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_items
/// The `items` element of a source.
inline constexpr detail_items::Read items{};
}  // namespace source

/// Whether `R` is what a source may present for `optional`, a body a gate
/// decides: a `std::optional` of a source of the body's codec.
template <class R>
struct is_optional_body : std::false_type {};
template <class B>
struct is_optional_body<std::optional<B>> : ::SCE::Generated::CodecOriginLeaf::is_source<B> {};

/// Whether `R` is a list a source may present for `items`: held to bounds
/// within its own, of any type, each entry a source of the entry codec.
template <class R, class = void>
struct is_items_list : std::false_type {};
template <class R>
struct is_items_list<R, std::enable_if_t<::SCE::Forge::Detail::IsBoundedWithin<R, 3>::value>>
    : ::SCE::Generated::CodecOriginLeaf::is_source<::SCE::Forge::Detail::ElementOf<R>> {};

/// Whether `S` presents every element of `CodecOriginEnvelope`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::hdr(std::declval<const S&>())), decltype(source::note(std::declval<const S&>())), decltype(source::required(std::declval<const S&>())), decltype(source::optional(std::declval<const S&>())), decltype(source::items(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::hdr(std::declval<const S&>())), uint8_t>, std::is_convertible<decltype(source::note(std::declval<const S&>())), ::SCE::Forge::Text<16>>, ::SCE::Generated::CodecOriginLeaf::is_source<std::decay_t<decltype(source::required(std::declval<const S&>()))>>, is_optional_body<std::decay_t<decltype(source::optional(std::declval<const S&>()))>>, is_items_list<std::decay_t<decltype(source::items(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::hdr(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<::SCE::Forge::Text<16>>(source::note(std::declval<const S&>()))) && noexcept(source::required(std::declval<const S&>())) && ::SCE::Generated::CodecOriginLeaf::is_nothrow_source<std::decay_t<decltype(source::required(std::declval<const S&>()))>>::value && noexcept(source::optional(std::declval<const S&>())) && ::SCE::Generated::CodecOriginLeaf::is_nothrow_source<typename std::decay_t<decltype(source::optional(std::declval<const S&>()))>::value_type>::value && noexcept(source::items(std::declval<const S&>())) && ::SCE::Generated::CodecOriginLeaf::is_nothrow_source<::SCE::Forge::Detail::ElementOf<std::decay_t<decltype(source::items(std::declval<const S&>()))>>>::value> {};

/// The encode of `CodecOriginEnvelope` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecOriginEnvelope: hdr, note, required, optional, items");
        const auto& _r_note = source::note(src);
        const ::SCE::Forge::Text<16> _e_note = _r_note;
        const auto& _r_required = source::required(src);
        const auto& _r_optional = source::optional(src);
        const auto& _r_items = source::items(src);
        const uint8_t _d_note_len = static_cast<uint8_t>(_e_note.size());
        const uint8_t _d_m = static_cast<uint8_t>(_r_items.size());
        std::uint64_t _f_hdr = static_cast<std::uint64_t>(source::hdr(src));
        _f_hdr = (_f_hdr & ~0x1ull) | static_cast<std::uint64_t>(_r_optional.has_value());
        const uint8_t _c_hdr = static_cast<uint8_t>(_f_hdr);
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_c_hdr)); _e) return _e;
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_d_note_len)); _e) return _e;
        if (auto _e = w.write_bytes(reinterpret_cast<const std::uint8_t*>(_e_note.data()), _e_note.size()); _e) return _e;
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_d_m)); _e) return _e;
        if (auto _e = ::SCE::Generated::CodecOriginLeaf::Wire::encode_source(_r_required, w); _e) return _e;
        if (_r_optional) {
            if (auto _e = ::SCE::Generated::CodecOriginLeaf::Wire::encode_source(*_r_optional, w); _e) return _e;
        }
        for (const auto& _x : _r_items) {
            if (auto _e = ::SCE::Generated::CodecOriginLeaf::Wire::encode_source(_x, w); _e) return _e;
        }
        return std::nullopt;
    }
};

struct CodecOriginEnvelope {
    CodecOriginEnvelope() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecOriginEnvelope(uint8_t hdr, ::SCE::Forge::Text<16> note, ::SCE::Generated::CodecOriginLeaf::CodecOriginLeaf required, std::optional<::SCE::Generated::CodecOriginLeaf::CodecOriginLeaf> optional, ::SCE::Forge::List<::SCE::Generated::CodecOriginLeaf::CodecOriginLeaf, 3> items)
        : hdr_(hdr), note_(note), required_(std::move(required)), optional_(std::move(optional)), items_(items) {}
    /// The `hdr` element.
    [[nodiscard]] constexpr uint8_t hdr() const noexcept { return hdr_; }
    /// The `note` element.
    [[nodiscard]] ::SCE::Forge::Text<16> note() const noexcept { return note_; }
    /// The `required` element, the body it holds.
    [[nodiscard]] const ::SCE::Generated::CodecOriginLeaf::CodecOriginLeaf& required() const noexcept { return required_; }
    /// The `optional` element, the body it holds where a gate leaves it in.
    [[nodiscard]] const std::optional<::SCE::Generated::CodecOriginLeaf::CodecOriginLeaf>& optional() const noexcept { return optional_; }
    /// The `items` element, the entries it holds.
    [[nodiscard]] ::SCE::Forge::List<::SCE::Generated::CodecOriginLeaf::CodecOriginLeaf, 3> items() const noexcept { return items_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecOriginEnvelope> decode(::SCE::Forge::SceCursor& cursor) {
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
        uint8_t hdr;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            hdr = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        uint8_t note_len;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            note_len = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::string note;
        {
            const auto _len = note_len;
            if (_len > 16) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange); return std::nullopt;
            }
            std::size_t _n = static_cast<std::size_t>(_len);
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            if (!::SCE::Forge::is_valid_utf8(raw, _n)) return std::nullopt;
            note.assign(reinterpret_cast<const char*>(raw), _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        uint8_t m;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            m = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        auto _emb_required = ::SCE::Generated::CodecOriginLeaf::CodecOriginLeaf::decode(cursor);
        if (!_emb_required.has_value()) return std::nullopt;
        auto required = std::move(*_emb_required);
        std::optional<::SCE::Generated::CodecOriginLeaf::CodecOriginLeaf> optional;
        if ((hdr & 0x01) != 0) {
            auto _emb = ::SCE::Generated::CodecOriginLeaf::CodecOriginLeaf::decode(cursor);
            if (!_emb.has_value()) return std::nullopt;
            optional = std::move(*_emb);
        }
        std::vector<::SCE::Generated::CodecOriginLeaf::CodecOriginLeaf> items;
        if (m > 3) {
            cursor.refuse(::SCE::Forge::CodecError::TooManyElements); return std::nullopt;
        }
        items.reserve(cursor.entries_backed(m));
        for (auto _i = decltype(m){0}; _i < m; ++_i) {
            auto _elem = ::SCE::Generated::CodecOriginLeaf::CodecOriginLeaf::decode(cursor);
            if (!_elem.has_value()) return std::nullopt;
            items.push_back(*_elem);
        }
        const auto _b_items = ::SCE::Forge::List<::SCE::Generated::CodecOriginLeaf::CodecOriginLeaf, 3>::make(items);
        if (!_b_items) {
            cursor.refuse(::SCE::Forge::CodecError::TooManyElements);
            return std::nullopt;
        }
        const auto _b_note = ::SCE::Forge::Text<16>::make(note);
        if (!_b_note) {
            cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);
            return std::nullopt;
        }
        (void)note_len;
        (void)m;
        CodecOriginEnvelope _decoded{
            /*hdr=*/hdr,
            /*note=*/*_b_note,
            /*required=*/std::move(required),
            /*optional=*/std::move(optional),
            /*items=*/*_b_items,
        };
        return _decoded;
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool has_opt() const noexcept {
        return (this->hdr_ & 0x01) != 0;
    }

    void set_has_opt(bool v) noexcept {
        if (v) {
            this->hdr_ = static_cast<uint8_t>(this->hdr_ | 0x01);
        } else {
            this->hdr_ = static_cast<uint8_t>(this->hdr_ & static_cast<uint8_t>(~0x01));
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 105;

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
    uint8_t hdr_{};
    ::SCE::Forge::Bounded<std::string, 16> note_{};
    ::SCE::Generated::CodecOriginLeaf::CodecOriginLeaf required_{};
    std::optional<::SCE::Generated::CodecOriginLeaf::CodecOriginLeaf> optional_{};
    ::SCE::Forge::Bounded<std::vector<::SCE::Generated::CodecOriginLeaf::CodecOriginLeaf>, 3> items_{};
};

}  // namespace SCE::Generated::CodecOriginEnvelope

#endif  // SCE_FORGE_CODEC_ORIGIN_ENVELOPE_H
