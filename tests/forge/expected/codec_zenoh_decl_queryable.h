// SCE-MAP: codec_zenoh_decl_queryable:46 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_DECL_QUERYABLE_H
#define SCE_FORGE_CODEC_ZENOH_DECL_QUERYABLE_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_zenoh_wireexpr.h"

namespace SCE::Generated::CodecZenohDeclQueryable {

/// The elements a value of `CodecZenohDeclQueryable` presents, each read by name: the
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
namespace detail_wireexpr {
void wireexpr() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.wireexpr())) -> decltype(s.wireexpr()) {
    return s.wireexpr();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(wireexpr(s))) -> decltype(wireexpr(s)) {
    return wireexpr(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_wireexpr
/// The `wireexpr` element of a source.
inline constexpr detail_wireexpr::Read wireexpr{};
namespace detail_z {
void z() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.z())) -> decltype(s.z()) {
    return s.z();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(z(s))) -> decltype(z(s)) {
    return z(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_z
/// The `z` element of a source.
inline constexpr detail_z::Read z{};
namespace detail_ext_type {
void ext_type() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.ext_type())) -> decltype(s.ext_type()) {
    return s.ext_type();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(ext_type(s))) -> decltype(ext_type(s)) {
    return ext_type(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_ext_type
/// The `ext_type` element of a source.
inline constexpr detail_ext_type::Read ext_type{};
namespace detail_ext_value {
void ext_value() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.ext_value())) -> decltype(s.ext_value()) {
    return s.ext_value();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(ext_value(s))) -> decltype(ext_value(s)) {
    return ext_value(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_ext_value
/// The `ext_value` element of a source.
inline constexpr detail_ext_value::Read ext_value{};
}  // namespace source

/// The fields `Z` decides (docs/adr/0065 §6 R4b), present together while
/// `Z` holds: a value holds all of them or none, and its encode derives the
/// gate from which.
class CodecZenohDeclQueryableZ {
public:
    CodecZenohDeclQueryableZ() = default;
    /// The value of each field, in declaration order.
    constexpr CodecZenohDeclQueryableZ(uint8_t ext_type, uint64_t ext_value) noexcept
        : ext_type_(ext_type), ext_value_(ext_value) {}
    /// The `ext_type` field.
    [[nodiscard]] constexpr uint8_t ext_type() const noexcept { return ext_type_; }
    /// The `ext_value` field.
    [[nodiscard]] constexpr uint64_t ext_value() const noexcept { return ext_value_; }

private:
    uint8_t ext_type_{};
    uint64_t ext_value_{};
};

/// Whether `G` presents every field of `CodecZenohDeclQueryableZ`, each convertible to its type.
template <class G, class = void>
struct is_z_members : std::false_type {};
template <class G>
struct is_z_members<G, std::void_t<decltype(source::ext_type(std::declval<const G&>())), decltype(source::ext_value(std::declval<const G&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::ext_type(std::declval<const G&>())), uint8_t>, std::is_convertible<decltype(source::ext_value(std::declval<const G&>())), uint64_t>> {};
/// Whether `R` is what a source may present for `z`, the fields `Z`
/// decides: a `std::optional` of a type `is_z_members` accepts.
template <class R>
struct is_z_group : std::false_type {};
template <class G>
struct is_z_group<std::optional<G>> : is_z_members<G> {};

/// Whether `S` presents every element of `CodecZenohDeclQueryable`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::id(std::declval<const S&>())), decltype(source::wireexpr(std::declval<const S&>())), decltype(source::z(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::id(std::declval<const S&>())), uint32_t>, ::SCE::Generated::CodecZenohWireexpr::is_source<std::decay_t<decltype(source::wireexpr(std::declval<const S&>()))>>, is_z_group<std::decay_t<decltype(source::z(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint32_t>(source::id(std::declval<const S&>()))) && noexcept(source::wireexpr(std::declval<const S&>())) && ::SCE::Generated::CodecZenohWireexpr::is_nothrow_source<std::decay_t<decltype(source::wireexpr(std::declval<const S&>()))>>::value && noexcept(source::z(std::declval<const S&>())) && noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::ext_type(std::declval<const typename std::decay_t<decltype(source::z(std::declval<const S&>()))>::value_type&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint64_t>(source::ext_value(std::declval<const typename std::decay_t<decltype(source::z(std::declval<const S&>()))>::value_type&>())))> {};

/// The encode of `CodecZenohDeclQueryable` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecZenohDeclQueryable: id, wireexpr, z");
        const auto& _r_wireexpr = source::wireexpr(src);
        const auto& _r_z = source::z(src);
        std::optional<uint8_t> _e_ext_type;
        std::optional<uint64_t> _e_ext_value;
        if (_r_z) {
            _e_ext_type = static_cast<uint8_t>(source::ext_type(*_r_z));
            _e_ext_value = static_cast<uint64_t>(source::ext_value(*_r_z));
        }
        if (auto _e = w.write_vle_u32(static_cast<std::uint32_t>(source::id(src))); _e) return _e;
        if (auto _e = ::SCE::Generated::CodecZenohWireexpr::Wire::encode_source(_r_wireexpr, w); _e) return _e;
        if (_e_ext_type) {
            if (auto _e = w.write_u8(static_cast<std::uint8_t>((*_e_ext_type))); _e) return _e;
        }
        if (_e_ext_value) {
            if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>((*_e_ext_value))); _e) return _e;
        }
        return std::nullopt;
    }
};

/// The flag input `N` a source of `CodecZenohDeclQueryable` says, which its encode is not
/// handed (docs/adr/0065 §6 R4a).
template <class S>
[[nodiscard]] std::uint8_t input_n(const S& src) noexcept(is_nothrow_source<S>::value) {
    static_assert(is_source<S>::value, "S presents every element of CodecZenohDeclQueryable");
    return ::SCE::Generated::CodecZenohWireexpr::input_n(source::wireexpr(src));
}

/// The flag input `Z` a source of `CodecZenohDeclQueryable` says, which its encode is not
/// handed (docs/adr/0065 §6 R4a).
template <class S>
[[nodiscard]] std::uint8_t input_z(const S& src) noexcept(is_nothrow_source<S>::value) {
    static_assert(is_source<S>::value, "S presents every element of CodecZenohDeclQueryable");
    return static_cast<std::uint8_t>(source::z(src).has_value() ? 1 : 0);
}

struct CodecZenohDeclQueryable {
    CodecZenohDeclQueryable() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecZenohDeclQueryable(uint32_t id, ::SCE::Generated::CodecZenohWireexpr::CodecZenohWireexpr wireexpr, std::optional<CodecZenohDeclQueryableZ> z)
        : id_(id), wireexpr_(std::move(wireexpr)), z_(z) {}
    /// The `id` element.
    [[nodiscard]] constexpr uint32_t id() const noexcept { return id_; }
    /// The `wireexpr` element, the body it holds.
    [[nodiscard]] const ::SCE::Generated::CodecZenohWireexpr::CodecZenohWireexpr& wireexpr() const noexcept { return wireexpr_; }
    /// The fields `Z` decides, where its gate leaves them in.
    [[nodiscard]] constexpr const std::optional<CodecZenohDeclQueryableZ>& z() const noexcept { return z_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohDeclQueryable> decode(::SCE::Forge::SceCursor& cursor, std::uint8_t n, std::uint8_t z) {
        // Declared-but-unconsumed flag inputs: defensive (void) suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly under -Wunused.
        (void)n;
        (void)z;
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
        auto id_opt = cursor.read_vle_u32();
        if (!id_opt.has_value()) return std::nullopt;
        auto id = static_cast<std::uint32_t>(*id_opt);
        auto _emb_wireexpr = ::SCE::Generated::CodecZenohWireexpr::CodecZenohWireexpr::decode(cursor, n);
        if (!_emb_wireexpr.has_value()) return std::nullopt;
        auto wireexpr = std::move(*_emb_wireexpr);
        std::optional<uint8_t> ext_type;
        if ((z & 0x01) != 0) {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            ext_type = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<uint64_t> ext_value;
        if ((z & 0x01) != 0) {
            auto _v_opt = cursor.read_vle_u64();
        if (!_v_opt.has_value()) return std::nullopt;
        auto _v = static_cast<std::uint64_t>(*_v_opt);
            ext_value = _v;
        }
        std::optional<CodecZenohDeclQueryableZ> _g_z;
        if (ext_type.has_value() && ext_value.has_value()) {
            _g_z.emplace(*ext_type, *ext_value);
        }
        CodecZenohDeclQueryable _decoded{
            /*id=*/id,
            /*wireexpr=*/std::move(wireexpr),
            /*z=*/_g_z,
        };
        return _decoded;
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 65570;

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
    uint32_t id_{};
    ::SCE::Generated::CodecZenohWireexpr::CodecZenohWireexpr wireexpr_{};
    std::optional<CodecZenohDeclQueryableZ> z_{};
};

}  // namespace SCE::Generated::CodecZenohDeclQueryable

#endif  // SCE_FORGE_CODEC_ZENOH_DECL_QUERYABLE_H
