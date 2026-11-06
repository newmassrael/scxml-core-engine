// SCE-MAP: codec_zenoh_decl_ext_keyexpr:89 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_DECL_EXT_KEYEXPR_H
#define SCE_FORGE_CODEC_ZENOH_DECL_EXT_KEYEXPR_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_zenoh_decl_ext_keyexpr_inner.h"

namespace SCE::Generated::CodecZenohDeclExtKeyexpr {

/// The elements a value of `CodecZenohDeclExtKeyexpr` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_outer_header {
void outer_header() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.outer_header())) -> decltype(s.outer_header()) {
    return s.outer_header();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(outer_header(s))) -> decltype(outer_header(s)) {
    return outer_header(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_outer_header
/// The `outer_header` element of a source.
inline constexpr detail_outer_header::Read outer_header{};
namespace detail_inner {
void inner() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.inner())) -> decltype(s.inner()) {
    return s.inner();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(inner(s))) -> decltype(inner(s)) {
    return inner(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_inner
/// The `inner` element of a source.
inline constexpr detail_inner::Read inner{};
}  // namespace source

/// Whether `S` presents every element of `CodecZenohDeclExtKeyexpr`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::outer_header(std::declval<const S&>())), decltype(source::inner(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::outer_header(std::declval<const S&>())), uint8_t>, ::SCE::Generated::CodecZenohDeclExtKeyexprInner::is_source<std::decay_t<decltype(source::inner(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::outer_header(std::declval<const S&>()))) && noexcept(source::inner(std::declval<const S&>())) && ::SCE::Generated::CodecZenohDeclExtKeyexprInner::is_nothrow_source<std::decay_t<decltype(source::inner(std::declval<const S&>()))>>::value> {};

/// One-bit flags of `CodecZenohDeclExtKeyexpr` a parent may set in place of the ones a value
/// presents, such as the flag a chain whose entries are `CodecZenohDeclExtKeyexpr` ends them
/// on, which the chain derives from where each entry stands.
struct Overrides {
    /// The `M` flag of `outer_header`, where a parent sets it.
    std::optional<bool> outer_header_m;
    /// The `Z` flag of `outer_header`, where a parent sets it.
    std::optional<bool> outer_header_z;
};

/// The encode of `CodecZenohDeclExtKeyexpr` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w, const Overrides& overrides = Overrides{}) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecZenohDeclExtKeyexpr: outer_header, inner");
        const auto& _r_inner = source::inner(src);
        std::uint64_t _f_outer_header = static_cast<std::uint64_t>(source::outer_header(src));
        if (overrides.outer_header_m.has_value()) {
            _f_outer_header = (_f_outer_header & ~0x10ull) | (static_cast<std::uint64_t>(*overrides.outer_header_m) << 4);
        }
        if (overrides.outer_header_z.has_value()) {
            _f_outer_header = (_f_outer_header & ~0x80ull) | (static_cast<std::uint64_t>(*overrides.outer_header_z) << 7);
        }
        const uint8_t _c_outer_header = static_cast<uint8_t>(_f_outer_header);
        ::SCE::Forge::CountingSink _k_inner;
        (void)::SCE::Generated::CodecZenohDeclExtKeyexprInner::Wire::encode_source(_r_inner, _k_inner);
        const uint64_t _d_total_length = static_cast<uint64_t>(_k_inner.position());
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_c_outer_header)); _e) return _e;
        if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(_d_total_length)); _e) return _e;
        if (auto _e = ::SCE::Generated::CodecZenohDeclExtKeyexprInner::Wire::encode_source(_r_inner, w); _e) return _e;
        return std::nullopt;
    }
};

struct CodecZenohDeclExtKeyexpr {
    CodecZenohDeclExtKeyexpr() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecZenohDeclExtKeyexpr(uint8_t outer_header, ::SCE::Generated::CodecZenohDeclExtKeyexprInner::CodecZenohDeclExtKeyexprInner inner)
        : outer_header_(outer_header), inner_(std::move(inner)) {}
    /// The `outer_header` element.
    [[nodiscard]] constexpr uint8_t outer_header() const noexcept { return outer_header_; }
    /// The `inner` element, the body it holds.
    [[nodiscard]] const ::SCE::Generated::CodecZenohDeclExtKeyexprInner::CodecZenohDeclExtKeyexprInner& inner() const noexcept { return inner_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohDeclExtKeyexpr> decode(::SCE::Forge::SceCursor& cursor) {
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
        uint8_t outer_header;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            outer_header = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        auto total_length_opt = cursor.read_vle_u64();
        if (!total_length_opt.has_value()) return std::nullopt;
        auto total_length = static_cast<std::uint64_t>(*total_length_opt);
        ::SCE::Generated::CodecZenohDeclExtKeyexprInner::CodecZenohDeclExtKeyexprInner inner;
        {
            std::size_t _len = static_cast<std::size_t>(total_length);
            const std::uint8_t* _raw = cursor.peek_slice(_len);
            if (_raw == nullptr) return std::nullopt;
            ::SCE::Forge::SceCursor _inner(_raw, _len);
            auto _emb = ::SCE::Generated::CodecZenohDeclExtKeyexprInner::CodecZenohDeclExtKeyexprInner::decode(_inner);
            if (!_emb.has_value()) return std::nullopt;
            if (!cursor.advance(_len)) return std::nullopt;
            inner = std::move(*_emb);
        }
        (void)total_length;
        CodecZenohDeclExtKeyexpr _decoded{
            /*outer_header=*/outer_header,
            /*inner=*/std::move(inner),
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
            (this->outer_header_ >> 0) & static_cast<uint8_t>(0x0F)
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
        this->outer_header_ = static_cast<uint8_t>(
            (this->outer_header_ & static_cast<uint8_t>(~_shifted_mask)) | _val
        );
    }

    bool m() const noexcept {
        return (this->outer_header_ & 0x10) != 0;
    }

    void set_m(bool v) noexcept {
        if (v) {
            this->outer_header_ = static_cast<uint8_t>(this->outer_header_ | 0x10);
        } else {
            this->outer_header_ = static_cast<uint8_t>(this->outer_header_ & static_cast<uint8_t>(~0x10));
        }
    }

    uint8_t enc() const noexcept {
        return static_cast<uint8_t>(
            (this->outer_header_ >> 5) & static_cast<uint8_t>(0x03)
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
        this->outer_header_ = static_cast<uint8_t>(
            (this->outer_header_ & static_cast<uint8_t>(~_shifted_mask)) | _val
        );
    }

    bool z() const noexcept {
        return (this->outer_header_ & 0x80) != 0;
    }

    void set_z(bool v) noexcept {
        if (v) {
            this->outer_header_ = static_cast<uint8_t>(this->outer_header_ | 0x80);
        } else {
            this->outer_header_ = static_cast<uint8_t>(this->outer_header_ & static_cast<uint8_t>(~0x80));
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 65555;

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
    uint8_t outer_header_{};
    ::SCE::Generated::CodecZenohDeclExtKeyexprInner::CodecZenohDeclExtKeyexprInner inner_{};
};

}  // namespace SCE::Generated::CodecZenohDeclExtKeyexpr

#endif  // SCE_FORGE_CODEC_ZENOH_DECL_EXT_KEYEXPR_H
