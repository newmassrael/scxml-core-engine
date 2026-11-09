// SCE-MAP: codec_zenoh_wireexpr:55 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_WIREEXPR_H
#define SCE_FORGE_CODEC_ZENOH_WIREEXPR_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <string>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecZenohWireexpr {

/// The elements a value of `CodecZenohWireexpr` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
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
}  // namespace source

/// Whether `S` presents every element of `CodecZenohWireexpr`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::id(std::declval<const S&>())), decltype(source::suffix(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::id(std::declval<const S&>())), uint64_t>, std::is_convertible<decltype(source::suffix(std::declval<const S&>())), std::optional<::SCE::Forge::Text<65535>>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint64_t>(source::id(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<std::optional<::SCE::Forge::Text<65535>>>(source::suffix(std::declval<const S&>())))> {};

/// The encode of `CodecZenohWireexpr` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecZenohWireexpr: id, suffix");
        const auto& _r_suffix = source::suffix(src);
        const std::optional<::SCE::Forge::Text<65535>> _e_suffix = _r_suffix;
        const std::uint8_t n = static_cast<std::uint8_t>(_e_suffix.has_value() ? 1 : 0);
        (void)n;
        const uint64_t _d_suffix_len = _e_suffix.has_value() ? static_cast<uint64_t>(_e_suffix->size()) : uint64_t{0};
        if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(source::id(src))); _e) return _e;
        if (((static_cast<std::uint64_t>(n) & 0x1ull) != 0)) {
            if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(_d_suffix_len)); _e) return _e;
        }
        if (_e_suffix) {
            if (auto _e = w.write_bytes(reinterpret_cast<const std::uint8_t*>(_e_suffix->data()), _e_suffix->size()); _e) return _e;
        }
        return std::nullopt;
    }
};

/// The flag input `N` a source of `CodecZenohWireexpr` says, which its encode is not
/// handed (docs/adr/0065 §6 R4a).
template <class S>
[[nodiscard]] std::uint8_t input_n(const S& src) noexcept(is_nothrow_source<S>::value) {
    static_assert(is_source<S>::value, "S presents every element of CodecZenohWireexpr");
    return static_cast<std::uint8_t>(static_cast<std::optional<::SCE::Forge::Text<65535>>>(source::suffix(src)).has_value() ? 1 : 0);
}

struct CodecZenohWireexpr {
    CodecZenohWireexpr() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecZenohWireexpr(uint64_t id, std::optional<::SCE::Forge::Text<65535>> suffix)
        : id_(id), suffix_(suffix) {}
    /// The `id` element.
    [[nodiscard]] constexpr uint64_t id() const noexcept { return id_; }
    /// The `suffix` element, where a gate leaves it in.
    [[nodiscard]] std::optional<::SCE::Forge::Text<65535>> suffix() const noexcept { if (!suffix_) { return std::nullopt; } return ::SCE::Forge::Text<65535>(*suffix_); }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohWireexpr> decode(::SCE::Forge::SceCursor& cursor, std::uint8_t n) {
        // Declared-but-unconsumed flag inputs: defensive (void) suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly under -Wunused.
        (void)n;
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
        auto id_opt = cursor.read_vle_u64();
        if (!id_opt.has_value()) return std::nullopt;
        auto id = static_cast<std::uint64_t>(*id_opt);
        std::optional<uint64_t> suffix_len;
        if ((n & 0x01) != 0) {
            auto _v_opt = cursor.read_vle_u64();
        if (!_v_opt.has_value()) return std::nullopt;
        auto _v = static_cast<std::uint64_t>(*_v_opt);
            suffix_len = _v;
        }
        std::optional<std::string> suffix;
        if ((n & 0x01) != 0) {
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
        std::optional<::SCE::Forge::Text<65535>> _b_suffix;
        if (suffix) {
            _b_suffix = ::SCE::Forge::Text<65535>::make(*suffix);
            if (!_b_suffix) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);
                return std::nullopt;
            }
        }
        (void)suffix_len;
        CodecZenohWireexpr _decoded{
            /*id=*/id,
            /*suffix=*/_b_suffix,
        };
        return _decoded;
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 65553;

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
    uint64_t id_{};
    std::optional<::SCE::Forge::Bounded<std::string, 65535>> suffix_{};
};

}  // namespace SCE::Generated::CodecZenohWireexpr

#endif  // SCE_FORGE_CODEC_ZENOH_WIREEXPR_H
