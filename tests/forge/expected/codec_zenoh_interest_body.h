// SCE-MAP: codec_zenoh_interest_body:56 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_INTEREST_BODY_H
#define SCE_FORGE_CODEC_ZENOH_INTEREST_BODY_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_zenoh_wireexpr.h"

namespace SCE::Generated::CodecZenohInterestBody {

/// The elements a value of `CodecZenohInterestBody` presents, each read by name: the
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
}  // namespace source

/// Whether `R` is what a source may present for `keyexpr`, a body a gate
/// decides: a `std::optional` of a source of the body's codec.
template <class R>
struct is_keyexpr_body : std::false_type {};
template <class B>
struct is_keyexpr_body<std::optional<B>> : ::SCE::Generated::CodecZenohWireexpr::is_source<B> {};

/// Whether `S` presents every element of `CodecZenohInterestBody`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::header(std::declval<const S&>())), decltype(source::keyexpr(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::header(std::declval<const S&>())), uint8_t>, is_keyexpr_body<std::decay_t<decltype(source::keyexpr(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::header(std::declval<const S&>()))) && noexcept(source::keyexpr(std::declval<const S&>())) && ::SCE::Generated::CodecZenohWireexpr::is_nothrow_source<typename std::decay_t<decltype(source::keyexpr(std::declval<const S&>()))>::value_type>::value> {};

/// One-bit flags of `CodecZenohInterestBody` a parent may set in place of the ones a value
/// presents, such as the flag a chain whose entries are `CodecZenohInterestBody` ends them
/// on, which the chain derives from where each entry stands.
struct Overrides {
    /// The `keyexprs` flag of `header`, where a parent sets it.
    std::optional<bool> header_keyexprs;
    /// The `subscribers` flag of `header`, where a parent sets it.
    std::optional<bool> header_subscribers;
    /// The `queryables` flag of `header`, where a parent sets it.
    std::optional<bool> header_queryables;
    /// The `tokens` flag of `header`, where a parent sets it.
    std::optional<bool> header_tokens;
    /// The `M` flag of `header`, where a parent sets it.
    std::optional<bool> header_m;
    /// The `aggregate` flag of `header`, where a parent sets it.
    std::optional<bool> header_aggregate;
};

/// The encode of `CodecZenohInterestBody` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w, const Overrides& overrides = Overrides{}) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecZenohInterestBody: header, keyexpr");
        const auto& _r_keyexpr = source::keyexpr(src);
        std::uint64_t _f_header = static_cast<std::uint64_t>(source::header(src));
        _f_header = (_f_header & ~0x10ull) | (static_cast<std::uint64_t>(_r_keyexpr.has_value()) << 4);
        if (_r_keyexpr.has_value()) {
            _f_header = (_f_header & ~0x20ull) | (static_cast<std::uint64_t>((_r_keyexpr.has_value() ? ::SCE::Generated::CodecZenohWireexpr::input_n(*_r_keyexpr) : std::uint8_t{0})) << 5);
        }
        if (overrides.header_keyexprs.has_value()) {
            _f_header = (_f_header & ~0x1ull) | static_cast<std::uint64_t>(*overrides.header_keyexprs);
        }
        if (overrides.header_subscribers.has_value()) {
            _f_header = (_f_header & ~0x2ull) | (static_cast<std::uint64_t>(*overrides.header_subscribers) << 1);
        }
        if (overrides.header_queryables.has_value()) {
            _f_header = (_f_header & ~0x4ull) | (static_cast<std::uint64_t>(*overrides.header_queryables) << 2);
        }
        if (overrides.header_tokens.has_value()) {
            _f_header = (_f_header & ~0x8ull) | (static_cast<std::uint64_t>(*overrides.header_tokens) << 3);
        }
        if (overrides.header_m.has_value()) {
            _f_header = (_f_header & ~0x40ull) | (static_cast<std::uint64_t>(*overrides.header_m) << 6);
        }
        if (overrides.header_aggregate.has_value()) {
            _f_header = (_f_header & ~0x80ull) | (static_cast<std::uint64_t>(*overrides.header_aggregate) << 7);
        }
        const uint8_t _c_header = static_cast<uint8_t>(_f_header);
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_c_header)); _e) return _e;
        if (_r_keyexpr) {
            if (auto _e = ::SCE::Generated::CodecZenohWireexpr::Wire::encode_source(*_r_keyexpr, w); _e) return _e;
        }
        return std::nullopt;
    }
};

struct CodecZenohInterestBody {
    CodecZenohInterestBody() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecZenohInterestBody(uint8_t header, std::optional<::SCE::Generated::CodecZenohWireexpr::CodecZenohWireexpr> keyexpr)
        : header_(header), keyexpr_(std::move(keyexpr)) {}
    /// The `header` element.
    [[nodiscard]] constexpr uint8_t header() const noexcept { return header_; }
    /// The `keyexpr` element, the body it holds where a gate leaves it in.
    [[nodiscard]] const std::optional<::SCE::Generated::CodecZenohWireexpr::CodecZenohWireexpr>& keyexpr() const noexcept { return keyexpr_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohInterestBody> decode(::SCE::Forge::SceCursor& cursor) {
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
        std::optional<::SCE::Generated::CodecZenohWireexpr::CodecZenohWireexpr> keyexpr;
        if ((header & 0x10) != 0) {
            auto _emb = ::SCE::Generated::CodecZenohWireexpr::CodecZenohWireexpr::decode(cursor, static_cast<std::uint8_t>((header >> 5) & 0x1));
            if (!_emb.has_value()) return std::nullopt;
            keyexpr = std::move(*_emb);
        }
        CodecZenohInterestBody _decoded{
            /*header=*/header,
            /*keyexpr=*/std::move(keyexpr),
        };
        return _decoded;
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool keyexprs() const noexcept {
        return (this->header_ & 0x01) != 0;
    }

    void set_keyexprs(bool v) noexcept {
        if (v) {
            this->header_ = static_cast<uint8_t>(this->header_ | 0x01);
        } else {
            this->header_ = static_cast<uint8_t>(this->header_ & static_cast<uint8_t>(~0x01));
        }
    }

    bool subscribers() const noexcept {
        return (this->header_ & 0x02) != 0;
    }

    void set_subscribers(bool v) noexcept {
        if (v) {
            this->header_ = static_cast<uint8_t>(this->header_ | 0x02);
        } else {
            this->header_ = static_cast<uint8_t>(this->header_ & static_cast<uint8_t>(~0x02));
        }
    }

    bool queryables() const noexcept {
        return (this->header_ & 0x04) != 0;
    }

    void set_queryables(bool v) noexcept {
        if (v) {
            this->header_ = static_cast<uint8_t>(this->header_ | 0x04);
        } else {
            this->header_ = static_cast<uint8_t>(this->header_ & static_cast<uint8_t>(~0x04));
        }
    }

    bool tokens() const noexcept {
        return (this->header_ & 0x08) != 0;
    }

    void set_tokens(bool v) noexcept {
        if (v) {
            this->header_ = static_cast<uint8_t>(this->header_ | 0x08);
        } else {
            this->header_ = static_cast<uint8_t>(this->header_ & static_cast<uint8_t>(~0x08));
        }
    }

    bool restricted() const noexcept {
        return (this->header_ & 0x10) != 0;
    }

    void set_restricted(bool v) noexcept {
        if (v) {
            this->header_ = static_cast<uint8_t>(this->header_ | 0x10);
        } else {
            this->header_ = static_cast<uint8_t>(this->header_ & static_cast<uint8_t>(~0x10));
        }
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

    bool aggregate() const noexcept {
        return (this->header_ & 0x80) != 0;
    }

    void set_aggregate(bool v) noexcept {
        if (v) {
            this->header_ = static_cast<uint8_t>(this->header_ | 0x80);
        } else {
            this->header_ = static_cast<uint8_t>(this->header_ & static_cast<uint8_t>(~0x80));
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 65554;

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
    uint8_t header_{};
    std::optional<::SCE::Generated::CodecZenohWireexpr::CodecZenohWireexpr> keyexpr_{};
};

}  // namespace SCE::Generated::CodecZenohInterestBody

#endif  // SCE_FORGE_CODEC_ZENOH_INTEREST_BODY_H
